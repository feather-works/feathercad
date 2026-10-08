//! DRW-013: primitivas CAD f64 => Drawing SVG, sem alterar o modelo.
//! cargo run --manifest-path feathercad/crates/feathercad-ponte/Cargo.toml \
//!   --example validacao_drawing_cad -j 2 > /tmp/feathercad-drawing.svg

use feathercad_core::{documento::Geometria, geom::Ponto, vista::Vista};
use feathercad_ponte::drawing_export::cena_das_primitivas;
use featheroffice_drawing::exportar_svg;

fn main() -> Result<(), String> {
    // Coordenadas grandes: convertidas para f32 apenas DEPOIS da câmera f64.
    let mut vista = Vista::inicial(600.0, 400.0);
    vista.centro = Ponto::novo(1.0e12, 1.0e12);
    vista.escala = 3.0;
    let formas = vec![
        Geometria::Linha {
            a: Ponto::novo(1.0e12 - 60.0, 1.0e12),
            b: Ponto::novo(1.0e12 + 60.0, 1.0e12),
        },
        Geometria::Circulo {
            centro: Ponto::novo(1.0e12, 1.0e12),
            raio: 32.0,
        },
        Geometria::Arco {
            centro: Ponto::novo(1.0e12, 1.0e12),
            raio: 56.0,
            inicio: 45.0,
            fim: 135.0,
        },
    ];
    let cena = cena_das_primitivas(&vista, &formas)
        .map_err(|e| format!("Projecao invalida: {e:?}"))?;
    if cena.figuras.len() < 3 {
        return Err("Faltaram primitivas vetoriais CAD.".into());
    }
    let svg = exportar_svg(&cena)
        .map_err(|e| format!("SVG invalido: {e:?}"))?;
    eprintln!(
        "DRW-013 CAD: {} segmentos projetados para {}x{} pixels; modelo CAD intacto",
        cena.figuras.len(), cena.largura, cena.altura
    );
    print!("{svg}");
    Ok(())
}
