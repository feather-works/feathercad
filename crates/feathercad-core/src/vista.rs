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
    #[serde(default)]
    pub rotacao_x: f64,
    #[serde(default)]
    pub rotacao_z: f64,
}

impl Vista {
    /// Vista inicial: origem no meio da área, 1 pixel por unidade.
    pub fn inicial(largura: f64, altura: f64) -> Self {
        Self { centro: Ponto::novo(0.0, 0.0), escala: 1.0, largura, altura, rotacao_x: 0.0, rotacao_z: 0.0 }
    }

    /// Ponto do desenho → coordenadas projetadas na tela antes da escala e translação da câmera.
    pub fn projetar(&self, p: Ponto) -> (f64, f64) {
        if self.rotacao_x.abs() < 1e-6 && self.rotacao_z.abs() < 1e-6 {
            (p.x, p.y)
        } else {
            let rx = self.rotacao_x.to_radians();
            let rz = self.rotacao_z.to_radians();
            let cos_z = rz.cos();
            let sin_z = rz.sin();
            let cos_x = rx.cos();
            let sin_x = rx.sin();
            let x1 = p.x * cos_z + p.y * sin_z;
            let y1 = -p.x * sin_z + p.y * cos_z;
            let yv = y1 * cos_x + p.z * sin_x;
            (x1, yv)
        }
    }

    /// Ponto do desenho → pixel da tela (o eixo Y da tela cresce para baixo).
    pub fn para_tela(&self, p: Ponto) -> (f64, f64) {
        let (cx, cy) = self.projetar(self.centro);
        let (xv, yv) = self.projetar(p);
        (
            (xv - cx) * self.escala + self.largura / 2.0,
            self.altura / 2.0 - (yv - cy) * self.escala,
        )
    }

    /// Pixel da tela → ponto do desenho no plano Z = 0 (ou plano ativo da câmera).
    pub fn para_mundo(&self, x: f64, y: f64) -> Ponto {
        self.para_mundo_em_z(x, y, 0.0)
    }

    /// Pixel da tela → ponto do desenho numa elevação Z específica (plano de trabalho).
    pub fn para_mundo_em_z(&self, x: f64, y: f64, z_plano: f64) -> Ponto {
        self.para_mundo_em_z_com_ref(x, y, z_plano, None)
    }

    /// Pixel da tela → ponto do desenho considerando elevação Z e ponto de referência.
    /// Em visões rasantes ou laterais (como Frontal ZX ou Lateral ZY onde X é profundidade),
    /// o raio projeta no plano vertical facing a câmera, permitindo movimentar o mouse livremente.
    pub fn para_mundo_em_z_com_ref(&self, x: f64, y: f64, z_plano: f64, ref_ponto: Option<Ponto>) -> Ponto {
        let dx = (x - self.largura / 2.0) / self.escala;
        let dy = (self.altura / 2.0 - y) / self.escala;
        if self.rotacao_x.abs() < 1e-6 && self.rotacao_z.abs() < 1e-6 {
            Ponto::novo3d(self.centro.x + dx, self.centro.y + dy, z_plano)
        } else {
            let rx = self.rotacao_x.to_radians();
            let rz = self.rotacao_z.to_radians();
            let cos_z = rz.cos();
            let sin_z = rz.sin();
            let cos_x = rx.cos();
            let sin_x = rx.sin();

            let u_x = cos_z;
            let u_y = sin_z;

            let v_x = -sin_z * cos_x;
            let v_y = cos_z * cos_x;
            let v_z = sin_x;

            let d_x = -sin_z * sin_x;
            let d_y = cos_z * sin_x;
            let d_z = -cos_x;

            let p0_x = self.centro.x + dx * u_x + dy * v_x;
            let p0_y = self.centro.y + dx * u_y + dy * v_y;
            let p0_z = self.centro.z + dy * v_z;

            let ref_p = ref_ponto.unwrap_or(self.centro);

            if d_z.abs() > 0.15 {
                let t = (z_plano - p0_z) / d_z;
                Ponto::novo3d(p0_x + t * d_x, p0_y + t * d_y, z_plano)
            } else if d_x.abs() >= d_y.abs() {
                let t = (ref_p.x - p0_x) / d_x;
                Ponto::novo3d(ref_p.x, p0_y + t * d_y, p0_z + t * d_z)
            } else {
                let t = (ref_p.y - p0_y) / d_y;
                Ponto::novo3d(p0_x + t * d_x, ref_p.y, p0_z + t * d_z)
            }
        }
    }

    /// Arrasta a vista `dx`, `dy` pixels (o desenho acompanha o mouse).
    pub fn deslocar(&self, dx: f64, dy: f64) -> Self {
        let mut v = *self;
        if v.rotacao_x.abs() < 1e-6 && v.rotacao_z.abs() < 1e-6 {
            v.centro.x -= dx / self.escala;
            v.centro.y += dy / self.escala;
        } else {
            let rx = self.rotacao_x.to_radians();
            let rz = self.rotacao_z.to_radians();
            let cos_z = rz.cos();
            let sin_z = rz.sin();
            let cos_x = rx.cos();
            let sin_x = rx.sin();
            let d_xv = -dx / self.escala;
            let d_yv = dy / self.escala;
            v.centro.x += d_xv * cos_z - d_yv * sin_z * cos_x;
            v.centro.y += d_xv * sin_z + d_yv * cos_z * cos_x;
            v.centro.z += d_yv * sin_x;
        }
        v
    }

    /// Multiplica a escala por `fator` mantendo parado o ponto sob (`x`, `y`).
    pub fn zoom(&self, x: f64, y: f64, fator: f64) -> Self {
        let (cx, cy) = self.projetar(self.centro);
        let xv_sob_cursor = cx + (x - self.largura / 2.0) / self.escala;
        let yv_sob_cursor = cy + (self.altura / 2.0 - y) / self.escala;
        let escala = (self.escala * fator).clamp(ESCALA_MIN, ESCALA_MAX);
        let novo_cx = xv_sob_cursor - (x - self.largura / 2.0) / escala;
        let novo_cy = yv_sob_cursor - (self.altura / 2.0 - y) / escala;
        let d_xv = novo_cx - cx;
        let d_yv = novo_cy - cy;
        let mut v = *self;
        v.escala = escala;
        if v.rotacao_x.abs() < 1e-6 && v.rotacao_z.abs() < 1e-6 {
            v.centro.x += d_xv;
            v.centro.y += d_yv;
        } else {
            let rx = self.rotacao_x.to_radians();
            let rz = self.rotacao_z.to_radians();
            let cos_z = rz.cos();
            let sin_z = rz.sin();
            let cos_x = rx.cos();
            let sin_x = rx.sin();
            v.centro.x += d_xv * cos_z - d_yv * sin_z * cos_x;
            v.centro.y += d_xv * sin_z + d_yv * cos_z * cos_x;
            v.centro.z += d_yv * sin_x;
        }
        v
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
        let util = 1.0 - 2.0 * FOLGA_AO_AJUSTAR;
        let centro_mundo = Ponto::novo3d(
            (min.x + max.x) / 2.0,
            (min.y + max.y) / 2.0,
            (min.z + max.z) / 2.0,
        );
        if self.rotacao_x.abs() < 1e-6 && self.rotacao_z.abs() < 1e-6 {
            let (dx, dy) = (max.x - min.x, max.y - min.y);
            let por_x = if dx > f64::EPSILON { self.largura * util / dx } else { f64::INFINITY };
            let por_y = if dy > f64::EPSILON { self.altura * util / dy } else { f64::INFINITY };
            let escala = por_x.min(por_y);
            Self {
                centro: centro_mundo,
                escala: if escala.is_finite() { escala.clamp(ESCALA_MIN, ESCALA_MAX) } else { 1.0 },
                ..*self
            }
        } else {
            let cantos = [
                Ponto::novo3d(min.x, min.y, min.z),
                Ponto::novo3d(max.x, min.y, min.z),
                Ponto::novo3d(min.x, max.y, min.z),
                Ponto::novo3d(max.x, max.y, min.z),
                Ponto::novo3d(min.x, min.y, max.z),
                Ponto::novo3d(max.x, min.y, max.z),
                Ponto::novo3d(min.x, max.y, max.z),
                Ponto::novo3d(max.x, max.y, max.z),
            ];
            let mut p_min = (f64::INFINITY, f64::INFINITY);
            let mut p_max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for c in cantos {
                let (px, py) = self.projetar(c);
                p_min.0 = p_min.0.min(px);
                p_min.1 = p_min.1.min(py);
                p_max.0 = p_max.0.max(px);
                p_max.1 = p_max.1.max(py);
            }
            let dx = (p_max.0 - p_min.0).max(1e-6);
            let dy = (p_max.1 - p_min.1).max(1e-6);
            let por_x = self.largura * util / dx;
            let por_y = self.altura * util / dy;
            let escala = por_x.min(por_y).clamp(ESCALA_MIN, ESCALA_MAX);
            Self {
                centro: centro_mundo,
                escala,
                ..*self
            }
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
