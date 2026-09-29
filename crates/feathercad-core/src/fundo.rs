//! Onde ficam as linhas (ou pontos) do fundo, já em pixels da tela. O espaçamento
//! se adapta ao zoom: sempre múltiplo do passo base por potência de 10, para o
//! snap (que usa o passo base) continuar caindo nos cruzamentos.

use crate::geom::Ponto;
use crate::vista::Vista;

/// Menor espaço aceito entre duas linhas na tela, em pixels.
const MIN_PX: f64 = 14.0;
/// De quantas em quantas linhas há uma "principal" (mais forte).
const PRINCIPAL_A_CADA: i64 = 5;
/// Teto de linhas por eixo (a vista absurda não trava o quadro).
const TETO_POR_EIXO: i64 = 600;

/// Uma linha do fundo: posição em pixels e se é principal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marca {
    pub pos: f32,
    pub principal: bool,
}

/// O passo (em unidades do desenho) mostrado neste zoom.
pub fn passo_visivel(base: f64, escala: f64) -> f64 {
    let mut passo = base;
    while passo * escala < MIN_PX && passo.is_finite() && passo < 1e12 {
        passo *= 10.0;
    }
    while passo / 10.0 * escala >= MIN_PX && passo > 1e-9 {
        passo /= 10.0;
    }
    passo
}

fn eixo(min: f64, max: f64, passo: f64, converter: impl Fn(f64) -> f64) -> Vec<Marca> {
    let (primeiro, ultimo) = ((min / passo).floor() as i64, (max / passo).ceil() as i64);
    if ultimo - primeiro > TETO_POR_EIXO {
        return Vec::new();
    }
    (primeiro..=ultimo)
        .map(|i| Marca { pos: converter(i as f64 * passo) as f32, principal: i.rem_euclid(PRINCIPAL_A_CADA) == 0 })
        .collect()
}

/// As marcas do fundo: (posições em X, posições em Y), em pixels.
pub fn marcas(vista: &Vista, base: f64) -> (Vec<Marca>, Vec<Marca>) {
    let passo = passo_visivel(base, vista.escala);
    let (min, max) = vista.area_visivel();
    (
        eixo(min.x, max.x, passo, |x| vista.para_tela(Ponto::novo(x, 0.0)).0),
        eixo(min.y, max.y, passo, |y| vista.para_tela(Ponto::novo(0.0, y)).1),
    )
}
