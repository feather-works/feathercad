//! A janela por onde se olha o desenho (zoom e pan). Quem guarda a vista é a
//! interface, mas **toda conta sobre ela é daqui**: o TypeScript manda o gesto
//! (arrastou tantos pixels, girou a roda) e recebe a vista nova.

use serde::{Deserialize, Serialize};

use crate::geom::Ponto;

const ESCALA_MIN: f64 = 1e-6;
const ESCALA_MAX: f64 = 1e9;
const FOLGA_AO_AJUSTAR: f64 = 0.05;

/// Centro do desenho na tela, escala (pixels por unidade) e tamanho da área.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vista {
    pub centro: Ponto,
    pub escala: f64,
    pub largura: f64,
    pub altura: f64,
}

impl Vista {
    /// Vista inicial: origem no meio da área, 1 pixel por unidade.
    pub fn inicial(largura: f64, altura: f64) -> Self {
        Self { centro: Ponto::novo(0.0, 0.0), escala: 1.0, largura, altura }
    }

    /// Ponto do desenho → pixel da tela (o eixo Y da tela cresce para baixo).
    pub fn para_tela(&self, p: Ponto) -> (f64, f64) {
        (
            (p.x - self.centro.x) * self.escala + self.largura / 2.0,
            self.altura / 2.0 - (p.y - self.centro.y) * self.escala,
        )
    }

    /// Pixel da tela → ponto do desenho.
    pub fn para_mundo(&self, x: f64, y: f64) -> Ponto {
        Ponto::novo(
            self.centro.x + (x - self.largura / 2.0) / self.escala,
            self.centro.y + (self.altura / 2.0 - y) / self.escala,
        )
    }

    /// Arrasta a vista `dx`, `dy` pixels (o desenho acompanha o mouse).
    pub fn deslocar(&self, dx: f64, dy: f64) -> Self {
        let mut v = *self;
        v.centro.x -= dx / self.escala;
        v.centro.y += dy / self.escala;
        v
    }

    /// Multiplica a escala por `fator` mantendo parado o ponto sob (`x`, `y`).
    pub fn zoom(&self, x: f64, y: f64, fator: f64) -> Self {
        let sob_o_cursor = self.para_mundo(x, y);
        let escala = (self.escala * fator).clamp(ESCALA_MIN, ESCALA_MAX);
        Self {
            centro: Ponto::novo(
                sob_o_cursor.x - (x - self.largura / 2.0) / escala,
                sob_o_cursor.y + (y - self.altura / 2.0) / escala,
            ),
            escala,
            ..*self
        }
    }

    /// Multiplica a escala por `fator` mantendo parado o centro da janela.
    pub fn zoom_centro(&self, fator: f64) -> Self {
        self.zoom(self.largura / 2.0, self.altura / 2.0, fator)
    }

    /// Enquadra a caixa (mínimo, máximo) com uma folga de 5% em cada lado
    /// ("zoom extensão"). Sem caixa (desenho vazio) volta à vista inicial.
    pub fn ajustar(&self, caixa: Option<(Ponto, Ponto)>) -> Self {
        let Some((min, max)) = caixa else {
            return Self::inicial(self.largura, self.altura);
        };
        let (dx, dy) = (max.x - min.x, max.y - min.y);
        let util = 1.0 - 2.0 * FOLGA_AO_AJUSTAR;
        let por_x = if dx > f64::EPSILON { self.largura * util / dx } else { f64::INFINITY };
        let por_y = if dy > f64::EPSILON { self.altura * util / dy } else { f64::INFINITY };
        let escala = por_x.min(por_y);
        Self {
            centro: Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0),
            escala: if escala.is_finite() { escala.clamp(ESCALA_MIN, ESCALA_MAX) } else { 1.0 },
            ..*self
        }
    }

    /// Os cantos do desenho visíveis: (mínimo, máximo).
    pub fn area_visivel(&self) -> (Ponto, Ponto) {
        let a = self.para_mundo(0.0, self.altura);
        let b = self.para_mundo(self.largura, 0.0);
        (a, b)
    }

    /// Uma distância em pixels, em unidades do desenho.
    pub fn pixels_em_unidades(&self, pixels: f64) -> f64 {
        pixels / self.escala
    }
}
