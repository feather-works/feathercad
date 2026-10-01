//! O que o cursor "gruda": snap ao objeto (`snap.rs`), ortogonal e guias. O
//! TypeScript manda a posição crua do mouse e só desenha o que volta.

use crate::documento::Documento;
use crate::geom::Ponto;
use crate::snap::{achar, ModosSnap, TipoSnap};

/// Os interruptores do rodapé que mexem no cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ajudas {
    /// Restringe ao eixo X ou Y a partir do ponto anterior.
    pub orto: bool,
    /// Snap à grade (com o passo, em unidades); `None` = desligado.
    pub grade: Option<f64>,
    /// Snap ao objeto ligado (F3); `modos` diz quais tipos.
    pub osnap: bool,
    pub modos: ModosSnap,
    /// De quantos em quantos graus o Orto trava a direção (90 = só horizontal e vertical; 45, 30, 15…).
    pub angulo: f64,
    /// Direção travada por `<45` na linha de comando (graus): o cursor só anda nessa reta a partir do ponto anterior.
    pub direcao: Option<f64>,
}

impl Default for Ajudas {
    fn default() -> Self {
        Self { orto: false, grade: None, osnap: true, modos: ModosSnap::default(), angulo: 90.0, direcao: None }
    }
}

/// Um segmento auxiliar para a interface desenhar (em coordenadas de mundo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guia {
    pub a: Ponto,
    pub b: Ponto,
}

/// O ponto onde o cursor de fato caiu.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolvido {
    pub ponto: Ponto,
    pub guias: Vec<Guia>,
    /// Há marca de OSNAP neste ponto?
    pub marca: bool,
    /// De que tipo é a marca.
    pub tipo: Option<TipoSnap>,
}

fn na_grade(v: f64, passo: f64) -> f64 {
    (v / passo).round() * passo
}

/// Aplica o ortogonal (se ligado e com ponto anterior) e a grade.
fn orto_e_grade(base: Option<Ponto>, bruto: Ponto, aj: &Ajudas) -> (Ponto, Vec<Guia>) {
    let mut p = bruto;
    let mut guias = Vec::new();
    let mut travado_x = false;
    let mut travado_y = false;
    if let (true, Some(b)) = (aj.orto || aj.direcao.is_some(), base) {
        let (dx, dy) = (bruto.x - b.x, bruto.y - b.y);
        let passo = if aj.angulo.is_finite() && aj.angulo > 0.0 { aj.angulo } else { 90.0 };
        // A direção travada é o múltiplo de `passo` mais perto do mouse; o ponto é a projeção do mouse nela.
        let travada = match aj.direcao {
            Some(graus) => graus.to_radians(),
            None => ((dy.atan2(dx).to_degrees() / passo).round() * passo).to_radians(),
        };
        let (seno, cosseno) = travada.sin_cos();
        let comprimento = dx * cosseno + dy * seno;
        p = Ponto::novo3d(b.x + comprimento * cosseno, b.y + comprimento * seno, b.z);
        // Nos eixos, o valor exato (sem resto de ponto flutuante); fora deles, nenhum eixo fica livre para a grade.
        let no_eixo_x = seno.abs() < 1e-9;
        let no_eixo_y = cosseno.abs() < 1e-9;
        if no_eixo_x {
            p.y = b.y;
        }
        if no_eixo_y {
            p.x = b.x;
        }
        travado_y = no_eixo_x || !no_eixo_y;
        travado_x = no_eixo_y || !no_eixo_x;
        guias.push(Guia { a: b, b: p });
    }
    if let Some(passo) = aj.grade.filter(|s| *s > 0.0) {
        if !travado_x {
            p.x = na_grade(p.x, passo);
        }
        if !travado_y {
            p.y = na_grade(p.y, passo);
        }
    }
    (p, guias)
}

/// Resolve a posição crua do mouse. O snap ao objeto ganha de tudo.
/// `tolerancia` é o alcance do OSNAP em unidades do desenho.
pub fn resolver(doc: &Documento, base: Option<Ponto>, bruto: Ponto, tolerancia: f64, aj: &Ajudas) -> Resolvido {
    if aj.osnap {
        if let Some(a) = achar(doc, base, bruto, tolerancia, &aj.modos) {
            return Resolvido { ponto: a.ponto, guias: Vec::new(), marca: true, tipo: Some(a.tipo) };
        }
    }
    let (ponto, guias) = orto_e_grade(base, bruto, aj);
    Resolvido { ponto, guias, marca: false, tipo: None }
}
