//! A "caneta" das folhas: desenha em **mm de papel** a partir do canto inferior esquerdo da folha, já na escala dela, e grava tudo
//! de uma vez (um passo de desfazer). Evita repetir a conversão mm → unidades do desenho em cada prancha.

use feathercad_core::documento::{AlinhaH, AlinhaV, Geometria};
use feathercad_core::folha::InfoFolha;
use feathercad_core::geom::{Ponto, Vertice};

use crate::agente::Agente;
use crate::json::Res;
use crate::pecas::{aplicar, Peca, Pincel};

pub type Mm = (f64, f64);

pub struct Caneta {
    origem: Ponto,
    /// Unidades do desenho por mm de papel.
    pub k: f64,
    pincel: Pincel,
}

impl Caneta {
    pub fn nova(f: &InfoFolha, camada: &str) -> Self {
        Self { origem: f.min, k: f.escala / 1000.0, pincel: Pincel::novo(camada) }
    }

    /// Do papel (mm) para o desenho.
    pub fn pt(&self, p: Mm) -> Ponto {
        Ponto::novo(self.origem.x + p.0 * self.k, self.origem.y + p.1 * self.k)
    }

    /// Camada e cor (`None` = a da camada) das próximas formas.
    pub fn em(&mut self, camada: &str, cor: Option<u32>) -> &mut Self {
        self.pincel.em(camada, cor);
        self
    }

    pub fn linha(&mut self, a: Mm, b: Mm) {
        let (a, b) = (self.pt(a), self.pt(b));
        self.pincel.linha(a, b);
    }

    pub fn poli(&mut self, pts: &[Mm]) {
        let v: Vec<Ponto> = pts.iter().map(|p| self.pt(*p)).collect();
        self.pincel.polilinha(&v);
    }

    pub fn fechada(&mut self, pts: &[Mm]) {
        let vertices = pts.iter().map(|p| Vertice { p: self.pt(*p), bulge: 0.0 }).collect();
        self.pincel.pecas.push(Peca { camada: self.pincel.camada.clone(), cor: self.pincel.cor, g: Geometria::Polilinha { vertices, fechada: true } });
    }

    pub fn ret(&mut self, a: Mm, b: Mm) {
        self.fechada(&[a, (b.0, a.1), b, (a.0, b.1)]);
    }

    pub fn texto(&mut self, p: Mm, altura_mm: f64, giro: f64, txt: &str, h: AlinhaH, v: AlinhaV) {
        let ponto = self.pt(p);
        self.pincel.texto(ponto, altura_mm * self.k, giro, txt, h, v);
    }

    /// Texto alinhado à esquerda, base na linha (o mais comum).
    pub fn rotulo(&mut self, p: Mm, altura_mm: f64, txt: &str) {
        self.texto(p, altura_mm, 0.0, txt, AlinhaH::Esquerda, AlinhaV::Base);
    }

    pub fn centro(&mut self, p: Mm, altura_mm: f64, txt: &str) {
        self.texto(p, altura_mm, 0.0, txt, AlinhaH::Centro, AlinhaV::Meio);
    }

    pub fn circulo(&mut self, c: Mm, raio_mm: f64) {
        let (centro, raio) = (self.pt(c), raio_mm * self.k);
        self.pincel.pecas.push(Peca { camada: self.pincel.camada.clone(), cor: self.pincel.cor, g: Geometria::Circulo { centro, raio } });
    }

    /// Região hachurada com um padrão (`ANSI31`, `ANSI32`, `AREIA`…) ou `SOLID`; `espaco_mm` é o espaçamento das linhas no papel
    /// (o motor dimensiona o padrão pelo tamanho da região, e aqui isso é desfeito para o traço não variar de peça para peça).
    pub fn hachura(&mut self, pts: &[Mm], padrao: &str, espaco_mm: f64) {
        if pts.len() < 3 {
            return;
        }
        let contorno: Vec<Vertice> = pts.iter().map(|p| Vertice { p: self.pt(*p), bulge: 0.0 }).collect();
        let (min, max) = contorno.iter().fold((Ponto::novo(f64::MAX, f64::MAX), Ponto::novo(f64::MIN, f64::MIN)), |(a, b), v| (Ponto::novo(a.x.min(v.p.x), a.y.min(v.p.y)), Ponto::novo(b.x.max(v.p.x), b.y.max(v.p.y))));
        let dimensao = (max.x - min.x).max(max.y - min.y).max(1e-9);
        let (linhas, nome) = feathercad_core::padroes::para_contornos(padrao, espaco_mm * self.k * 40.0 / dimensao, std::slice::from_ref(&contorno));
        self.pincel.pecas.push(Peca { camada: self.pincel.camada.clone(), cor: self.pincel.cor, g: Geometria::Hachura { contornos: vec![contorno], padrao: linhas, nome } });
    }

    pub fn gravar(self, ag: &mut Agente) -> Res<()> {
        aplicar(&mut ag.doc, self.pincel.pecas).map(|_| ())
    }
}

/// Cores das classes de criticidade (contrastam nos dois temas).
pub const COR_CRITICO: u32 = 0xE0_30_30;
pub const COR_ATENCAO: u32 = 0xE0_80_00;
pub const COR_NORMAL: u32 = 0x20_A0_50;
/// Aterro (greide acima do existente) e escavação (abaixo).
pub const COR_ATERRO: u32 = 0xD0_40_40;
pub const COR_ESCAVACAO: u32 = 0x30_80_D0;
/// Existente e projeto, como nas outras folhas.
pub const COR_EXISTENTE: u32 = 0xFF_7F_00;
pub const COR_PROJETO: u32 = 0xFF_00_FF;
pub const COR_GRADE: u32 = 0x90_90_90;

pub fn cor_do_nivel(n: feathercad_core::viario::criticidade::Nivel) -> u32 {
    use feathercad_core::viario::criticidade::Nivel;
    match n {
        Nivel::Critico => COR_CRITICO,
        Nivel::Atencao => COR_ATENCAO,
        Nivel::Normal => COR_NORMAL,
    }
}
