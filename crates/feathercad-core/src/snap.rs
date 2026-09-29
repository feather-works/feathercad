//! Snap ao objeto (OSNAP): os pontos notáveis onde o cursor pode grudar e quais tipos estão ligados.
//!
//! Cada entidade é decomposta em **primitivas** (segmento reto ou arco/círculo; o bulge da polilinha vira arco
//! exato). Dos candidatos dentro do alcance ganha o mais perto do cursor; o "Mais próximo" só vale quando
//! nenhum outro tipo achou nada. Perpendicular e Tangente pedem o ponto anterior (`base`).

use crate::documento::{Documento, Geometria};
use crate::geom::{normalizar_graus, Ponto, Vertice};

/// O tipo de um ponto de snap. A ordem é a de desempate (mais à frente perde o empate).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TipoSnap {
    Extremidade,
    Intersecao,
    Meio,
    Centro,
    Quadrante,
    No,
    Perpendicular,
    Tangente,
    Proximo,
}

impl TipoSnap {
    pub const TODOS: [TipoSnap; 9] = [Self::Extremidade, Self::Intersecao, Self::Meio, Self::Centro, Self::Quadrante, Self::No, Self::Perpendicular, Self::Tangente, Self::Proximo];

    /// O nome que trafega na ponte e nos arquivos de preferência.
    pub fn nome(self) -> &'static str {
        match self {
            Self::Extremidade => "extremidade",
            Self::Intersecao => "intersecao",
            Self::Meio => "meio",
            Self::Centro => "centro",
            Self::Quadrante => "quadrante",
            Self::No => "no",
            Self::Perpendicular => "perpendicular",
            Self::Tangente => "tangente",
            Self::Proximo => "proximo",
        }
    }

    pub fn de_nome(nome: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|t| t.nome() == nome)
    }
}

/// Quais tipos de snap estão ligados.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModosSnap {
    ligados: [bool; 9],
}

impl Default for ModosSnap {
    /// De fábrica (como o AutoCAD): extremidade, interseção, meio, centro, quadrante e nó.
    fn default() -> Self {
        let mut m = Self { ligados: [false; 9] };
        for t in [TipoSnap::Extremidade, TipoSnap::Intersecao, TipoSnap::Meio, TipoSnap::Centro, TipoSnap::Quadrante, TipoSnap::No] {
            m.definir(t, true);
        }
        m
    }
}

impl ModosSnap {
    pub fn ligado(&self, t: TipoSnap) -> bool {
        self.ligados[t as usize]
    }

    pub fn definir(&mut self, t: TipoSnap, ligado: bool) {
        self.ligados[t as usize] = ligado;
    }
}

/// Um ponto onde o cursor pode grudar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Achado {
    pub ponto: Ponto,
    pub tipo: TipoSnap,
}

/// Uma peça simples de uma entidade.
#[derive(Clone, Copy, Debug)]
enum Prim {
    Seg(Ponto, Ponto),
    /// `varredura` em graus, anti-horária a partir de `inicio`; 360 = círculo.
    Arco { centro: Ponto, raio: f64, inicio: f64, varredura: f64 },
}

fn no_arco(inicio: f64, varredura: f64, angulo: f64) -> bool {
    varredura >= 360.0 - 1e-9 || normalizar_graus(angulo - inicio) <= varredura + 1e-9
}

fn ponto_no_arco(centro: Ponto, raio: f64, graus: f64) -> Ponto {
    Ponto::novo(centro.x + raio * graus.to_radians().cos(), centro.y + raio * graus.to_radians().sin())
}

/// O trecho `a → b` com `bulge` como primitiva.
fn prim_de_trecho(a: Ponto, b: Ponto, bulge: f64) -> Option<Prim> {
    let corda = a.distancia(b);
    if corda < f64::EPSILON {
        return None;
    }
    if bulge.abs() < 1e-9 || !bulge.is_finite() {
        return Some(Prim::Seg(a, b));
    }
    let (ux, uy) = ((b.x - a.x) / corda, (b.y - a.y) / corda);
    let desloc = (corda / 2.0) * (1.0 - bulge * bulge) / (2.0 * bulge);
    let centro = Ponto::novo((a.x + b.x) / 2.0 - uy * desloc, (a.y + b.y) / 2.0 + ux * desloc);
    let varredura = 4.0 * bulge.atan().to_degrees();
    let (inicio, varredura) = if varredura >= 0.0 { (centro.angulo_ate(a), varredura) } else { (centro.angulo_ate(b), -varredura) };
    Some(Prim::Arco { centro, raio: centro.distancia(a), inicio, varredura })
}

fn prims_da_polilinha(vertices: &[Vertice], fechada: bool, saida: &mut Vec<Prim>) {
    let n = vertices.len();
    let trechos = if fechada { n } else { n.saturating_sub(1) };
    for i in 0..trechos {
        if let Some(p) = prim_de_trecho(vertices[i].p, vertices[(i + 1) % n].p, vertices[i].bulge) {
            saida.push(p);
        }
    }
}

fn prims_da_entidade(g: &Geometria, saida: &mut Vec<Prim>) {
    match g {
        Geometria::Linha { a, b } => saida.push(Prim::Seg(*a, *b)),
        Geometria::Polilinha { vertices, fechada } => prims_da_polilinha(vertices, *fechada, saida),
        Geometria::Circulo { centro, raio } => saida.push(Prim::Arco { centro: *centro, raio: *raio, inicio: 0.0, varredura: 360.0 }),
        Geometria::Arco { centro, raio, inicio, fim } => saida.push(Prim::Arco { centro: *centro, raio: *raio, inicio: *inicio, varredura: crate::geom::varredura_graus(*inicio, *fim) }),
        _ => {}
    }
}

/// O ponto da primitiva mais perto de `p`.
fn mais_proximo(prim: &Prim, p: Ponto) -> Ponto {
    match *prim {
        Prim::Seg(a, b) => {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            Ponto::novo(a.x + t * dx, a.y + t * dy)
        }
        Prim::Arco { centro, raio, inicio, varredura } => {
            let ang = centro.angulo_ate(p);
            if no_arco(inicio, varredura, ang) {
                return ponto_no_arco(centro, raio, ang);
            }
            let (a, b) = (ponto_no_arco(centro, raio, inicio), ponto_no_arco(centro, raio, inicio + varredura));
            if a.distancia(p) <= b.distancia(p) { a } else { b }
        }
    }
}

fn distancia_da_prim(prim: &Prim, p: Ponto) -> f64 {
    mais_proximo(prim, p).distancia(p)
}

// ---------------------------------------------------------------- interseções

fn seg_seg(a: Ponto, b: Ponto, c: Ponto, d: Ponto) -> Option<Ponto> {
    let (r, s) = ((b.x - a.x, b.y - a.y), (d.x - c.x, d.y - c.y));
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let (qp0, qp1) = (c.x - a.x, c.y - a.y);
    let t = (qp0 * s.1 - qp1 * s.0) / den;
    let u = (qp0 * r.1 - qp1 * r.0) / den;
    let f = -1e-9..=1.0 + 1e-9;
    (f.contains(&t) && f.contains(&u)).then(|| Ponto::novo(a.x + t * r.0, a.y + t * r.1))
}

fn seg_circulo(a: Ponto, b: Ponto, centro: Ponto, raio: f64) -> Vec<Ponto> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (fx, fy) = (a.x - centro.x, a.y - centro.y);
    let (qa, qb, qc) = (dx * dx + dy * dy, 2.0 * (fx * dx + fy * dy), fx * fx + fy * fy - raio * raio);
    let disc = qb * qb - 4.0 * qa * qc;
    if qa < 1e-18 || disc < 0.0 {
        return Vec::new();
    }
    let raiz = disc.sqrt();
    [(-qb - raiz) / (2.0 * qa), (-qb + raiz) / (2.0 * qa)].into_iter().filter(|t| (-1e-9..=1.0 + 1e-9).contains(t)).map(|t| Ponto::novo(a.x + t * dx, a.y + t * dy)).collect()
}

fn circulo_circulo(c1: Ponto, r1: f64, c2: Ponto, r2: f64) -> Vec<Ponto> {
    let d = c1.distancia(c2);
    if d < 1e-12 || d > r1 + r2 || d < (r1 - r2).abs() {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let (ux, uy) = ((c2.x - c1.x) / d, (c2.y - c1.y) / d);
    let m = Ponto::novo(c1.x + a * ux, c1.y + a * uy);
    vec![Ponto::novo(m.x - h * uy, m.y + h * ux), Ponto::novo(m.x + h * uy, m.y - h * ux)]
}

fn dentro(prim: &Prim, p: Ponto) -> bool {
    match *prim {
        Prim::Seg(..) => true,
        Prim::Arco { centro, inicio, varredura, .. } => no_arco(inicio, varredura, centro.angulo_ate(p)),
    }
}

fn intersecoes(x: &Prim, y: &Prim) -> Vec<Ponto> {
    let brutas = match (*x, *y) {
        (Prim::Seg(a, b), Prim::Seg(c, d)) => return seg_seg(a, b, c, d).into_iter().collect(),
        (Prim::Seg(a, b), Prim::Arco { centro, raio, .. }) | (Prim::Arco { centro, raio, .. }, Prim::Seg(a, b)) => seg_circulo(a, b, centro, raio),
        (Prim::Arco { centro: c1, raio: r1, .. }, Prim::Arco { centro: c2, raio: r2, .. }) => circulo_circulo(c1, r1, c2, r2),
    };
    brutas.into_iter().filter(|p| dentro(x, *p) && dentro(y, *p)).collect()
}

// ---------------------------------------------------------------- candidatos

fn achados_da_prim(prim: &Prim, modos: &ModosSnap, base: Option<Ponto>, saida: &mut Vec<Achado>) {
    let mut dar = |ponto: Ponto, tipo: TipoSnap| {
        if modos.ligado(tipo) {
            saida.push(Achado { ponto, tipo });
        }
    };
    match *prim {
        Prim::Seg(a, b) => {
            dar(a, TipoSnap::Extremidade);
            dar(b, TipoSnap::Extremidade);
            dar(Ponto::novo((a.x + b.x) / 2.0, (a.y + b.y) / 2.0), TipoSnap::Meio);
            if let Some(o) = base {
                dar(mais_proximo(prim, o), TipoSnap::Perpendicular);
            }
        }
        Prim::Arco { centro, raio, inicio, varredura } => {
            dar(centro, TipoSnap::Centro);
            if varredura < 360.0 - 1e-9 {
                dar(ponto_no_arco(centro, raio, inicio), TipoSnap::Extremidade);
                dar(ponto_no_arco(centro, raio, inicio + varredura), TipoSnap::Extremidade);
                dar(ponto_no_arco(centro, raio, inicio + varredura / 2.0), TipoSnap::Meio);
            }
            for q in [0.0, 90.0, 180.0, 270.0] {
                if no_arco(inicio, varredura, q) {
                    dar(ponto_no_arco(centro, raio, q), TipoSnap::Quadrante);
                }
            }
            if let Some(o) = base {
                let d = centro.distancia(o);
                if d > 1e-12 {
                    let ang = centro.angulo_ate(o);
                    for a in [ang, ang + 180.0] {
                        if no_arco(inicio, varredura, a) {
                            dar(ponto_no_arco(centro, raio, a), TipoSnap::Perpendicular);
                        }
                    }
                }
                if d > raio + 1e-9 {
                    // Os dois pontos de tangência: a `acos(raio/d)` de cada lado da direção centro → base.
                    let (ang, abertura) = (centro.angulo_ate(o), (raio / d).acos().to_degrees());
                    for a in [ang + abertura, ang - abertura] {
                        if no_arco(inicio, varredura, a) {
                            dar(ponto_no_arco(centro, raio, a), TipoSnap::Tangente);
                        }
                    }
                }
            }
        }
    }
}

/// O melhor ponto de snap perto de `cursor` (dentro de `tolerancia`), ou `None`.
pub fn achar(doc: &Documento, base: Option<Ponto>, cursor: Ponto, tolerancia: f64, modos: &ModosSnap) -> Option<Achado> {
    let mut candidatos: Vec<Achado> = Vec::new();
    let mut proximas: Vec<Prim> = Vec::new();
    for e in doc.entidades().iter().filter(|e| doc.visivel(e)) {
        let mut prims = Vec::new();
        prims_da_entidade(&e.geometria, &mut prims);
        // Todo ponto de snap está SOBRE uma primitiva: só quem passa dentro do alcance do cursor importa.
        // (O centro é a exceção: fica fora da curva, e gruda também com o cursor em cima dele.)
        if modos.ligado(TipoSnap::Centro) {
            for p in &prims {
                if let Prim::Arco { centro, .. } = p {
                    if centro.distancia(cursor) <= tolerancia {
                        candidatos.push(Achado { ponto: *centro, tipo: TipoSnap::Centro });
                    }
                }
            }
        }
        let nesta: Vec<Prim> = prims.into_iter().filter(|p| distancia_da_prim(p, cursor) <= tolerancia).collect();
        for p in &nesta {
            achados_da_prim(p, modos, base, &mut candidatos);
        }
        if let Geometria::Texto { ponto, .. } | Geometria::Insercao { ponto, .. } | Geometria::Ponto { p: ponto } = &e.geometria {
            if modos.ligado(TipoSnap::No) {
                candidatos.push(Achado { ponto: *ponto, tipo: TipoSnap::No });
            }
        }
        proximas.extend(nesta);
    }
    if modos.ligado(TipoSnap::Intersecao) {
        for (i, x) in proximas.iter().enumerate() {
            for y in &proximas[i + 1..] {
                candidatos.extend(intersecoes(x, y).into_iter().map(|ponto| Achado { ponto, tipo: TipoSnap::Intersecao }));
            }
        }
    }
    let melhor = candidatos
        .into_iter()
        .filter(|c| c.ponto.distancia(cursor) <= tolerancia)
        .min_by(|a, b| a.ponto.distancia(cursor).total_cmp(&b.ponto.distancia(cursor)).then(a.tipo.cmp(&b.tipo)));
    if melhor.is_some() || !modos.ligado(TipoSnap::Proximo) {
        return melhor;
    }
    proximas
        .iter()
        .map(|p| mais_proximo(p, cursor))
        .filter(|q| q.distancia(cursor) <= tolerancia)
        .min_by(|a, b| a.distancia(cursor).total_cmp(&b.distancia(cursor)))
        .map(|ponto| Achado { ponto, tipo: TipoSnap::Proximo })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn doc_com(g: Vec<Geometria>) -> Documento {
        let mut d = Documento::default();
        for x in g {
            d.adicionar_geometria(x);
        }
        d
    }

    fn linha(a: (f64, f64), b: (f64, f64)) -> Geometria {
        Geometria::Linha { a: Ponto::novo(a.0, a.1), b: Ponto::novo(b.0, b.1) }
    }

    fn so(tipo: TipoSnap) -> ModosSnap {
        let mut m = ModosSnap::default();
        for t in TipoSnap::TODOS {
            m.definir(t, t == tipo);
        }
        m
    }

    #[test]
    fn meio_da_linha() {
        let d = doc_com(vec![linha((0.0, 0.0), (10.0, 0.0))]);
        let a = achar(&d, None, Ponto::novo(5.2, 0.3), 1.0, &so(TipoSnap::Meio)).unwrap();
        assert_eq!((a.ponto, a.tipo), (Ponto::novo(5.0, 0.0), TipoSnap::Meio));
    }

    #[test]
    fn interseccao_de_duas_linhas() {
        let d = doc_com(vec![linha((0.0, 0.0), (10.0, 10.0)), linha((0.0, 10.0), (10.0, 0.0))]);
        let a = achar(&d, None, Ponto::novo(5.3, 4.8), 1.0, &so(TipoSnap::Intersecao)).unwrap();
        assert!(a.ponto.distancia(Ponto::novo(5.0, 5.0)) < 1e-9);
    }

    #[test]
    fn interseccao_de_linha_com_circulo() {
        let d = doc_com(vec![linha((-10.0, 0.0), (10.0, 0.0)), Geometria::Circulo { centro: Ponto::novo(0.0, 0.0), raio: 5.0 }]);
        let a = achar(&d, None, Ponto::novo(5.2, 0.2), 1.0, &so(TipoSnap::Intersecao)).unwrap();
        assert!(a.ponto.distancia(Ponto::novo(5.0, 0.0)) < 1e-9);
    }

    #[test]
    fn perpendicular_pede_o_ponto_anterior() {
        let d = doc_com(vec![linha((0.0, 0.0), (10.0, 0.0))]);
        assert!(achar(&d, None, Ponto::novo(3.2, 0.2), 1.0, &so(TipoSnap::Perpendicular)).is_none());
        let a = achar(&d, Some(Ponto::novo(3.0, 8.0)), Ponto::novo(3.2, 0.2), 1.0, &so(TipoSnap::Perpendicular)).unwrap();
        assert_eq!(a.ponto, Ponto::novo(3.0, 0.0));
    }

    #[test]
    fn tangente_do_ponto_ao_circulo() {
        let d = doc_com(vec![Geometria::Circulo { centro: Ponto::novo(0.0, 0.0), raio: 5.0 }]);
        // De (13, 0) a tangente toca em x = 25/13, y = ±12·5/13.
        let (x, y) = (25.0 / 13.0, 60.0 / 13.0);
        let a = achar(&d, Some(Ponto::novo(13.0, 0.0)), Ponto::novo(x + 0.1, y - 0.1), 1.0, &so(TipoSnap::Tangente)).unwrap();
        assert!(a.ponto.distancia(Ponto::novo(x, y)) < 1e-9);
    }

    #[test]
    fn quadrante_e_centro_do_arco() {
        let d = doc_com(vec![Geometria::Arco { centro: Ponto::novo(0.0, 0.0), raio: 5.0, inicio: 0.0, fim: 180.0 }]);
        let q = achar(&d, None, Ponto::novo(0.1, 5.1), 1.0, &so(TipoSnap::Quadrante)).unwrap();
        assert!(q.ponto.distancia(Ponto::novo(0.0, 5.0)) < 1e-9);
        assert!(achar(&d, None, Ponto::novo(0.0, -5.0), 1.0, &so(TipoSnap::Quadrante)).is_none(), "fora da varredura do arco");
        assert_eq!(achar(&d, None, Ponto::novo(0.2, 0.1), 1.0, &so(TipoSnap::Centro)).unwrap().ponto, Ponto::novo(0.0, 0.0));
    }

    #[test]
    fn mais_proximo_so_vale_sem_outro_e_desligado_nao_acha_nada() {
        let d = doc_com(vec![linha((0.0, 0.0), (10.0, 0.0))]);
        let a = achar(&d, None, Ponto::novo(3.7, 0.4), 1.0, &so(TipoSnap::Proximo)).unwrap();
        assert_eq!(a.ponto, Ponto::novo(3.7, 0.0));
        assert!(achar(&d, None, Ponto::novo(3.7, 0.4), 1.0, &so(TipoSnap::Extremidade)).is_none());
        let mut ligados = ModosSnap::default();
        ligados.definir(TipoSnap::Proximo, true);
        let perto_da_ponta = achar(&d, None, Ponto::novo(0.3, 0.2), 1.0, &ligados).unwrap();
        assert_eq!(perto_da_ponta.tipo, TipoSnap::Extremidade);
    }

    #[test]
    fn bulge_da_polilinha_e_arco_exato() {
        // Meio círculo de (0,0) a (10,0) (bulge 1, anti-horário: passa por baixo): centro (5,0), raio 5.
        let v = |x, y, b| Vertice { p: Ponto::novo(x, y), bulge: b };
        let d = doc_com(vec![Geometria::Polilinha { vertices: vec![v(0.0, 0.0, 1.0), v(10.0, 0.0, 0.0)], fechada: false }]);
        let m = achar(&d, None, Ponto::novo(5.1, -5.1), 1.0, &so(TipoSnap::Meio)).unwrap();
        assert!(m.ponto.distancia(Ponto::novo(5.0, -5.0)) < 1e-9, "anti-horário: o arco passa por baixo");
    }
}
