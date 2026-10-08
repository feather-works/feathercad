//! Projeção CAD f64 -> Drawing f32 **após** a câmera.
//!
//! Este adaptador serve à exportação/prévia vetorial: o motor CAD, o snap,
//! o modelo, os arquivos e as tolerâncias continuam exclusivamente em f64.
//! Nunca converta entidades CAD para f32 antes de `Vista::para_tela`.
//! Cores/propriedades de camada e entidades complexas requerem uma etapa
//! posterior; por enquanto o contrato limita-se às primitivas de contorno.

use feathercad_core::{documento::Geometria, quadro, vista::Vista};
use featheroffice_drawing::{Cena, Cor, Estilo, Figura, Ponto};

const MAX_SEGMENTOS: usize = 20_000;
// Pixels em que o espaçamento unitário ainda é representável por f32.
const MAX_COORDENADA: f64 = 16_777_216.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErroProjecao {
    VistaInvalida,
    CoordenadaForaDoAlcance,
    MuitosSegmentos,
}

fn ponto(x: f64, y: f64) -> Result<Ponto, ErroProjecao> {
    if !x.is_finite() || !y.is_finite()
        || x.abs() > MAX_COORDENADA || y.abs() > MAX_COORDENADA
    {
        return Err(ErroProjecao::CoordenadaForaDoAlcance);
    }
    Ok(Ponto::novo(x as f32, y as f32))
}

/// Extrai contornos CAD para uma cena SVG, em pixels da viewport.
///
/// `quadro::tracos` é o renderizador geométrico já usado no CAD:
/// mantém amostragem de arcos/bulge e projeção f64 da câmera, evitando
/// qualquer implementação duplicada de geometria de precisão.
///
/// `Geometria`/vista originais são somente emprestadas, nunca modificadas.
/// A saída é uma visualização, **não** uma conversão de dados CAD para edição.
pub fn cena_das_primitivas(
    vista: &Vista,
    geometrias: &[Geometria],
) -> Result<Cena, ErroProjecao> {
    if !vista.escala.is_finite() || vista.escala <= 0.0
        || !vista.largura.is_finite() || !vista.altura.is_finite()
        || vista.largura <= 0.0 || vista.altura <= 0.0
        || vista.largura > MAX_COORDENADA || vista.altura > MAX_COORDENADA
        || !vista.centro.x.is_finite() || !vista.centro.y.is_finite()
        || !vista.centro.z.is_finite()
        || !vista.rotacao_x.is_finite() || !vista.rotacao_z.is_finite()
    {
        return Err(ErroProjecao::VistaInvalida);
    }

    let mut cena = Cena::nova(vista.largura as f32, vista.altura as f32);
    for geometria in geometrias {
        // O renderizador CAD pressupõe dois vértices em polilinhas.
        if matches!(geometria, Geometria::Polilinha { vertices, .. } if vertices.len() < 2) {
            continue;
        }
        let tracos = quadro::tracos(geometria, vista);
        if tracos.len() > MAX_SEGMENTOS.saturating_sub(cena.figuras.len()) {
            return Err(ErroProjecao::MuitosSegmentos);
        }
        for ((ax, ay), (bx, by)) in tracos {
            let de = ponto(ax, ay)?;
            let ate = ponto(bx, by)?;
            cena.adicionar(Figura::Polilinha {
                pontos: vec![de, ate],
                fechada: false,
                estilo: Estilo {
                    preenchimento: None,
                    contorno: Some(Cor::rgb(34, 80, 160)),
                    espessura: 1.0,
                },
            });
        }
    }
    Ok(cena)
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::geom::Ponto as PontoCad;

    #[test]
    fn subtracao_usa_f64_antes_de_projetar_coordenadas_cad_grandes() {
        let mut vista = Vista::inicial(400.0, 200.0);
        vista.centro = PontoCad::novo(1.0e12, 1.0e12);
        vista.escala = 4.0;
        let geo = Geometria::Linha {
            a: PontoCad::novo(1.0e12, 1.0e12),
            b: PontoCad::novo(1.0e12 + 5.0, 1.0e12),
        };
        let cena = cena_das_primitivas(&vista, &[geo]).unwrap();
        assert_eq!(cena.figuras.len(), 1);
        let Figura::Polilinha { pontos, .. } = &cena.figuras[0] else {
            panic!("linha projetada precisa ser polilinha");
        };
        assert_eq!(pontos.as_slice(), &[Ponto::novo(200.0, 100.0), Ponto::novo(220.0, 100.0)]);
    }

    #[test]
    fn rejeita_vista_degenerada_e_nao_altera_geometria_original() {
        let mut vista = Vista::inicial(800.0, 600.0);
        let arco = Geometria::Arco {
            centro: PontoCad::novo(0.0, 0.0),
            raio: 20.0,
            inicio: 0.0,
            fim: 90.0,
        };
        let original = arco.clone();
        let cena = cena_das_primitivas(&vista, &[arco.clone()]).unwrap();
        assert!(cena.figuras.len() > 1);
        assert_eq!(arco, original);
        vista.escala = 0.0;
        assert_eq!(cena_das_primitivas(&vista, &[arco]), Err(ErroProjecao::VistaInvalida));
    }
}
