//! APP-013: o motor CAD deve preservar f64 e apenas projetar para f32 na borda da GUI.
//! Estes testes não importam featheroffice-drawing: Drawing é motor visual,
//! não substitui operações geométricas, snapping nem serialização CAD.

use feathercad_core::geom::{Ponto, Transf};
use feathercad_core::vista::Vista;

fn aproximadamente(recebido: f64, esperado: f64, tolerancia: f64) {
    assert!(
        (recebido - esperado).abs() <= tolerancia,
        "{recebido} != {esperado} (tolerancia {tolerancia})"
    );
}

#[test]
fn coordenadas_utm_com_detalhes_milimetricos_continuam_distintas() {
    // Magnitudes reais de um desenho topográfico (norte UTM ~7,4 milhões).
    let base = Ponto::novo3d(7_394_300.0, 333_300.0, 125.0);
    let proximo = Ponto::novo3d(7_394_300.001, 333_300.002, 125.003);

    aproximadamente(base.distancia(proximo), (0.001_f64.powi(2) + 0.002_f64.powi(2)).sqrt(), 1e-9);
    aproximadamente(base.distancia_3d(proximo), (0.001_f64.powi(2) + 0.002_f64.powi(2) + 0.003_f64.powi(2)).sqrt(), 1e-9);
    assert_ne!(base.x, proximo.x);
    assert_ne!(base.y, proximo.y);

    // Uma substituição por featheroffice-drawing::Ponto (f32) apagaria esses detalhes.
    assert_eq!(base.x as f32, proximo.x as f32);
    assert_eq!(base.y as f32, proximo.y as f32);
}

#[test]
fn desenhar_depois_da_projecao_evitaria_perda_de_precisao_de_mundo() {
    let mut vista = Vista::inicial(1600.0, 900.0);
    vista.centro = Ponto::novo(7_394_300.0, 333_300.0);
    vista.escala = 1000.0;
    let ponto = Ponto::novo(7_394_300.125, 333_300.25);

    // CAD calcula o deslocamento em f64 antes de entregar pixels lógicos à GUI.
    let (x, y) = vista.para_tela(ponto);
    aproximadamente(x, 925.0, 1e-6);
    aproximadamente(y, 200.0, 1e-6);
    aproximadamente(x as f32 as f64, 925.0, 1e-4);
    aproximadamente(y as f32 as f64, 200.0, 1e-4);

    // Converter coordenadas absolutas para f32 ANTES do offset seria incorreto.
    let x_errado = ((ponto.x as f32) - (vista.centro.x as f32)) * vista.escala as f32 + 800.0;
    assert_ne!(x_errado, x as f32);
}

#[test]
fn tela_para_mundo_preserva_pequenos_deslocamentos_apos_zoom() {
    let mut vista = Vista::inicial(1920.0, 1080.0);
    vista.centro = Ponto::novo(7_394_300.0, 333_300.0);
    vista.escala = 10_000.0;
    let ponto = Ponto::novo(7_394_300.001, 333_300.002);

    let (px, py) = vista.para_tela(ponto);
    let reconstruido = vista.para_mundo(px, py);
    aproximadamente(reconstruido.x, ponto.x, 2e-9);
    aproximadamente(reconstruido.y, ponto.y, 2e-9);

    // A tolerância de snap de 2 px é de 0,2 mm neste zoom, calculada em f64.
    aproximadamente(vista.pixels_em_unidades(2.0), 0.0002, 1e-12);
}

#[test]
fn transformacao_cad_em_f64_nao_arredonda_deslocamentos_milimetrados() {
    let ponto = Ponto::novo3d(7_394_300.0, 333_300.0, 125.0125);
    let mover = Transf::mover(0.0125, -0.0025);
    let deslocado = mover.aplicar(ponto);

    aproximadamente(deslocado.x - ponto.x, 0.0125, 2e-9);
    aproximadamente(deslocado.y - ponto.y, -0.0025, 2e-9);
    assert_eq!(deslocado.z, ponto.z);

    let restaurado = mover.inversa().unwrap().aplicar(deslocado);
    aproximadamente(restaurado.x, ponto.x, 2e-9);
    aproximadamente(restaurado.y, ponto.y, 2e-9);
    assert_eq!(restaurado.z, ponto.z);
}

#[test]
fn coordenadas_locais_de_tela_podem_ser_f32_mas_nao_devem_voltar_ao_documento() {
    let mut vista = Vista::inicial(1280.0, 720.0);
    vista.centro = Ponto::novo(7_394_300.0, 333_300.0);
    vista.escala = 2000.0;
    let ponto_cad = Ponto::novo3d(7_394_300.05, 333_300.0125, 42.123456789);
    let (x, y) = vista.para_tela(ponto_cad);
    let posicao_visual = (x as f32, y as f32);

    // O rasterizador pode quantizar sua coordenada visual sem alterar o CAD.
    assert!(posicao_visual.0.is_finite() && posicao_visual.1.is_finite());
    assert!(posicao_visual.0 >= 0.0 && posicao_visual.0 <= vista.largura as f32);
    assert!(posicao_visual.1 >= 0.0 && posicao_visual.1 <= vista.altura as f32);
    assert_eq!(ponto_cad.z, 42.123456789);
    assert_ne!(ponto_cad.x, (ponto_cad.x as f32) as f64);
}
