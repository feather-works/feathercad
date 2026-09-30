//! Aparar, concordância (fillet) e o contorno de uma hachura por ponto interno. Só geometria: devolve
//! as `Mudanca`s que a sessão aplica como um passo de desfazer.

use crate::documento::{Documento, Geometria, Id, Mudanca, NovaEntidade};
use crate::edicao::{caixa_da_entidade, pontos_da_polilinha};
use crate::geom::{normalizar_graus, varredura_graus, Ponto, Vertice};
use crate::regiao::regiao_em;

/// Por que a operação não aconteceu (a sessão traduz em mensagem).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recusa {
    /// Nenhuma outra forma cruza a peça: não há onde cortar.
    SemCorte,
    /// A forma não é das que este comando trata.
    NaoSuportado,
    /// As duas retas são paralelas.
    Paralelas,
    /// O raio não cabe nos trechos das retas.
    RaioGrande,
    /// Não há contorno fechado neste ponto.
    SemContorno,
}

/// Uma linha ou um arco de círculo (`varredura` = 360 é o círculo inteiro), para achar cruzamentos.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Curva {
    Seg(Ponto, Ponto),
    Arco { c: Ponto, r: f64, ini: f64, varr: f64 },
}

const EPS: f64 = 1e-9;

pub(crate) fn no_arco(c: Ponto, ini: f64, varr: f64, p: Ponto) -> bool {
    varr >= 360.0 - EPS || varredura_graus(ini, c.angulo_ate(p)) <= varr + 1e-7 || varredura_graus(ini, c.angulo_ate(p)) >= 360.0 - 1e-7
}

/// As curvas de uma forma. Polilinha com arco vira cordas.
pub(crate) fn curvas_de(geo: &Geometria) -> Vec<Curva> {
    match geo {
        Geometria::Linha { a, b } => vec![Curva::Seg(*a, *b)],
        Geometria::Circulo { centro, raio } => vec![Curva::Arco { c: *centro, r: *raio, ini: 0.0, varr: 360.0 }],
        Geometria::Arco { centro, raio, inicio, fim } => vec![Curva::Arco { c: *centro, r: *raio, ini: *inicio, varr: varredura_graus(*inicio, *fim) }],
        Geometria::Polilinha { vertices, fechada } => {
            let pts = pontos_da_polilinha(vertices, *fechada);
            let mut v: Vec<Curva> = pts.windows(2).map(|w| Curva::Seg(w[0], w[1])).collect();
            if *fechada && pts.len() > 2 {
                v.push(Curva::Seg(pts[pts.len() - 1], pts[0]));
            }
            v
        }
        _ => Vec::new(),
    }
}

/// Onde o segmento `a–b` cruza o círculo `(c, r)`.
fn seg_circulo(a: Ponto, b: Ponto, c: Ponto, r: f64) -> Vec<Ponto> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (fx, fy) = (a.x - c.x, a.y - c.y);
    let (qa, qb, qc) = (dx * dx + dy * dy, 2.0 * (fx * dx + fy * dy), fx * fx + fy * fy - r * r);
    if qa < 1e-18 {
        return Vec::new();
    }
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return Vec::new();
    }
    let raiz = disc.sqrt();
    let mut ts = vec![(-qb - raiz) / (2.0 * qa), (-qb + raiz) / (2.0 * qa)];
    if raiz < 1e-12 {
        ts.pop();
    }
    ts.into_iter().filter(|t| (-EPS..=1.0 + EPS).contains(t)).map(|t| Ponto::novo(a.x + t * dx, a.y + t * dy)).collect()
}

/// Onde as duas curvas se cruzam.
pub(crate) fn cruzamentos(x: &Curva, y: &Curva) -> Vec<Ponto> {
    match (x, y) {
        (Curva::Seg(a, b), Curva::Seg(c, d)) => {
            let (rx, ry, sx, sy) = (b.x - a.x, b.y - a.y, d.x - c.x, d.y - c.y);
            let den = rx * sy - ry * sx;
            if den.abs() < 1e-14 {
                return Vec::new();
            }
            let (t, u) = (((c.x - a.x) * sy - (c.y - a.y) * sx) / den, ((c.x - a.x) * ry - (c.y - a.y) * rx) / den);
            if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
                vec![Ponto::novo(a.x + t * rx, a.y + t * ry)]
            } else {
                Vec::new()
            }
        }
        (Curva::Seg(a, b), Curva::Arco { c, r, ini, varr }) | (Curva::Arco { c, r, ini, varr }, Curva::Seg(a, b)) => {
            seg_circulo(*a, *b, *c, *r).into_iter().filter(|p| no_arco(*c, *ini, *varr, *p)).collect()
        }
        (Curva::Arco { c: c1, r: r1, ini: i1, varr: v1 }, Curva::Arco { c: c2, r: r2, ini: i2, varr: v2 }) => {
            let d = c1.distancia(*c2);
            if d < 1e-12 || d > r1 + r2 + EPS || d < (r1 - r2).abs() - EPS {
                return Vec::new();
            }
            let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
            let h = (r1 * r1 - a * a).max(0.0).sqrt();
            let (ux, uy) = ((c2.x - c1.x) / d, (c2.y - c1.y) / d);
            let meio = Ponto::novo(c1.x + a * ux, c1.y + a * uy);
            let mut pts = vec![Ponto::novo(meio.x - h * uy, meio.y + h * ux)];
            if h > 1e-9 {
                pts.push(Ponto::novo(meio.x + h * uy, meio.y - h * ux));
            }
            pts.into_iter().filter(|p| no_arco(*c1, *i1, *v1, *p) && no_arco(*c2, *i2, *v2, *p)).collect()
        }
    }
}

/// Todos os cruzamentos de `alvo` com as outras formas visíveis do documento (as arestas de corte).
fn cruzamentos_com_o_resto(doc: &Documento, alvo: Id, curva: &Curva) -> Vec<Ponto> {
    doc.entidades()
        .iter()
        .filter(|e| e.id != alvo && doc.visivel(e))
        .flat_map(|e| curvas_de(&e.geometria))
        .flat_map(|c| cruzamentos(curva, &c))
        .collect()
}

fn ponto_no_segmento(a: Ponto, b: Ponto, t: f64) -> Ponto {
    Ponto::novo(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn parametro_no_segmento(a: Ponto, b: Ponto, p: Ponto) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy).max(1e-30)
}

/// Dos cortes `cortes` (parâmetros) e do clique `tc`, o corte imediatamente antes e o imediatamente depois.
fn vizinhos(cortes: &[f64], tc: f64) -> (Option<f64>, Option<f64>) {
    let lo = cortes.iter().copied().filter(|t| *t < tc - 1e-12).fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))));
    let hi = cortes.iter().copied().filter(|t| *t > tc + 1e-12).fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.min(t))));
    (lo, hi)
}

/// Apara a forma `alvo` no ponto `clique`: some o trecho entre os dois cortes vizinhos ao clique.
pub fn aparar(doc: &Documento, alvo: Id, clique: Ponto) -> Result<Vec<Mudanca>, Recusa> {
    let e = doc.entidade(alvo).ok_or(Recusa::NaoSuportado)?;
    let nova = |g: Geometria| NovaEntidade { camada: e.camada, cor: e.cor, geometria: g, props: Default::default() };
    match &e.geometria {
        Geometria::Linha { a, b } => {
            let curva = Curva::Seg(*a, *b);
            let cortes: Vec<f64> = cruzamentos_com_o_resto(doc, alvo, &curva).iter().map(|p| parametro_no_segmento(*a, *b, *p)).filter(|t| *t > EPS && *t < 1.0 - EPS).collect();
            let (lo, hi) = vizinhos(&cortes, parametro_no_segmento(*a, *b, clique).clamp(0.0, 1.0));
            let peca = |t0: f64, t1: f64| Geometria::Linha { a: ponto_no_segmento(*a, *b, t0), b: ponto_no_segmento(*a, *b, t1) };
            match (lo, hi) {
                (None, None) => Err(Recusa::SemCorte),
                (Some(lo), None) => Ok(vec![Mudanca::Trocar(alvo, peca(0.0, lo))]),
                (None, Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, peca(hi, 1.0))]),
                (Some(lo), Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, peca(0.0, lo)), Mudanca::Novo(nova(peca(hi, 1.0)))]),
            }
        }
        Geometria::Arco { centro, raio, inicio, fim } => {
            let varr = varredura_graus(*inicio, *fim);
            let curva = Curva::Arco { c: *centro, r: *raio, ini: *inicio, varr };
            let cortes: Vec<f64> = cruzamentos_com_o_resto(doc, alvo, &curva).iter().map(|p| varredura_graus(*inicio, centro.angulo_ate(*p))).filter(|s| *s > 1e-7 && *s < varr - 1e-7).collect();
            let (lo, hi) = vizinhos(&cortes, varredura_graus(*inicio, centro.angulo_ate(clique)).min(varr));
            let peca = |s0: f64, s1: f64| Geometria::Arco { centro: *centro, raio: *raio, inicio: normalizar_graus(inicio + s0), fim: normalizar_graus(inicio + s1) };
            match (lo, hi) {
                (None, None) => Err(Recusa::SemCorte),
                (Some(lo), None) => Ok(vec![Mudanca::Trocar(alvo, peca(0.0, lo))]),
                (None, Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, peca(hi, varr))]),
                (Some(lo), Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, peca(0.0, lo)), Mudanca::Novo(nova(peca(hi, varr)))]),
            }
        }
        Geometria::Circulo { centro, raio } => {
            let curva = Curva::Arco { c: *centro, r: *raio, ini: 0.0, varr: 360.0 };
            let mut angulos: Vec<f64> = cruzamentos_com_o_resto(doc, alvo, &curva).iter().map(|p| centro.angulo_ate(*p)).collect();
            angulos.sort_by(f64::total_cmp);
            angulos.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
            if angulos.len() < 2 {
                return Err(Recusa::SemCorte);
            }
            let ac = centro.angulo_ate(clique);
            // O vizinho de cada lado, dando a volta: o último ângulo antes do clique e o primeiro depois.
            let lo = angulos.iter().copied().filter(|a| *a < ac).fold(f64::MIN, f64::max);
            let lo = if lo == f64::MIN { *angulos.last().unwrap_or(&0.0) } else { lo };
            let hi = angulos.iter().copied().filter(|a| *a > ac).fold(f64::MAX, f64::min);
            let hi = if hi == f64::MAX { angulos[0] } else { hi };
            // Sobra o arco que vai de `hi` até `lo`, no sentido anti-horário.
            Ok(vec![Mudanca::Trocar(alvo, Geometria::Arco { centro: *centro, raio: *raio, inicio: hi, fim: lo })])
        }
        Geometria::Polilinha { vertices, fechada } => aparar_polilinha(doc, alvo, vertices, *fechada, clique, &nova),
        _ => Err(Recusa::NaoSuportado),
    }
}

/// Aparar polilinha de trechos retos. O parâmetro corre ao longo dela: `i + t` no trecho `i`.
fn aparar_polilinha(doc: &Documento, alvo: Id, vertices: &[Vertice], fechada: bool, clique: Ponto, nova: &dyn Fn(Geometria) -> NovaEntidade) -> Result<Vec<Mudanca>, Recusa> {
    if vertices.iter().any(|v| v.bulge != 0.0) || vertices.len() < 2 {
        return Err(Recusa::NaoSuportado);
    }
    let n = vertices.len();
    let trechos = if fechada { n } else { n - 1 };
    let pt = |i: usize| vertices[i % n].p;
    let (mut cortes, mut melhor) = (Vec::new(), (f64::MAX, 0.0));
    for i in 0..trechos {
        let (a, b) = (pt(i), pt(i + 1));
        for x in cruzamentos_com_o_resto(doc, alvo, &Curva::Seg(a, b)) {
            let t = parametro_no_segmento(a, b, x);
            if t > EPS && t < 1.0 - EPS {
                cortes.push(i as f64 + t);
            }
        }
        let t = parametro_no_segmento(a, b, clique).clamp(0.0, 1.0);
        let d = clique.distancia(ponto_no_segmento(a, b, t));
        if d < melhor.0 {
            melhor = (d, i as f64 + t);
        }
    }
    let ponto_em = |s: f64| {
        let i = (s.floor() as usize).min(trechos - 1);
        ponto_no_segmento(pt(i), pt(i + 1), s - i as f64)
    };
    // Os pontos de `de` até `ate` (sem dar a volta), com as pontas exatas.
    let trecho = |de: f64, ate: f64| -> Vec<Ponto> {
        let mut v = vec![ponto_em(de)];
        v.extend((0..=trechos).filter(|k| (*k as f64) > de + EPS && (*k as f64) < ate - EPS).map(pt));
        v.push(ponto_em(ate));
        v
    };
    let como_polilinha = |pts: Vec<Ponto>| Geometria::Polilinha { vertices: pts.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada: false };
    let (lo, hi) = vizinhos(&cortes, melhor.1);
    if fechada {
        // Some o trecho do clique; sobra o resto, aberto, do corte `hi` dando a volta até o corte `lo`.
        let (lo, hi) = match (lo, hi) {
            (None, None) => return Err(Recusa::SemCorte),
            (Some(l), Some(h)) => (l, h),
            (Some(l), None) => (l, cortes.iter().copied().fold(f64::MAX, f64::min)),
            (None, Some(h)) => (cortes.iter().copied().fold(f64::MIN, f64::max), h),
        };
        let mut pts = trecho(hi, trechos as f64);
        pts.pop();
        pts.extend(trecho(0.0, lo));
        pts.dedup_by(|a, b| a.distancia(*b) < 1e-9);
        return Ok(vec![Mudanca::Trocar(alvo, como_polilinha(pts))]);
    }
    match (lo, hi) {
        (None, None) => Err(Recusa::SemCorte),
        (Some(lo), None) => Ok(vec![Mudanca::Trocar(alvo, como_polilinha(trecho(0.0, lo)))]),
        (None, Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, como_polilinha(trecho(hi, trechos as f64)))]),
        (Some(lo), Some(hi)) => Ok(vec![Mudanca::Trocar(alvo, como_polilinha(trecho(0.0, lo))), Mudanca::Novo(nova(como_polilinha(trecho(hi, trechos as f64))))]),
    }
}

/// O ponto onde as retas infinitas das duas linhas se cruzam.
fn cruzamento_de_retas(a1: Ponto, b1: Ponto, a2: Ponto, b2: Ponto) -> Option<Ponto> {
    let (rx, ry, sx, sy) = (b1.x - a1.x, b1.y - a1.y, b2.x - a2.x, b2.y - a2.y);
    let den = rx * sy - ry * sx;
    if den.abs() < 1e-12 * (rx.hypot(ry) * sx.hypot(sy)).max(1e-30) {
        return None;
    }
    let t = ((a2.x - a1.x) * sy - (a2.y - a1.y) * sx) / den;
    Some(Ponto::novo(a1.x + t * rx, a1.y + t * ry))
}

/// Concordância de duas linhas com `raio` (0 = só levar as duas ao encontro). Cada `(id, clique)`
/// diz qual lado da linha fica (o do clique). Só linhas retas.
pub fn concordar(doc: &Documento, (id1, clique1): (Id, Ponto), (id2, clique2): (Id, Ponto), raio: f64) -> Result<Vec<Mudanca>, Recusa> {
    let (Some(e1), Some(e2)) = (doc.entidade(id1), doc.entidade(id2)) else { return Err(Recusa::NaoSuportado) };
    let (Geometria::Linha { a: a1, b: b1 }, Geometria::Linha { a: a2, b: b2 }) = (&e1.geometria, &e2.geometria) else { return Err(Recusa::NaoSuportado) };
    if id1 == id2 {
        return Err(Recusa::NaoSuportado);
    }
    let x = cruzamento_de_retas(*a1, *b1, *a2, *b2).ok_or(Recusa::Paralelas)?;
    // A ponta que fica: a do lado do clique em relação ao canto `x` (se as duas estão do mesmo lado, a mais longe dele).
    let lado = |a: Ponto, b: Ponto, clique: Ponto| {
        let do_lado = |p: Ponto| (p.x - x.x) * (clique.x - x.x) + (p.y - x.y) * (clique.y - x.y) >= 0.0;
        let (mais_longe, mais_perto) = if a.distancia(x) >= b.distancia(x) { (a, b) } else { (b, a) };
        if do_lado(mais_longe) { (mais_longe, mais_perto) } else { (mais_perto, mais_longe) }
    };
    let ((fica1, _), (fica2, _)) = (lado(*a1, *b1, clique1), lado(*a2, *b2, clique2));
    let unit = |p: Ponto| {
        let d = p.distancia(x).max(1e-30);
        ((p.x - x.x) / d, (p.y - x.y) / d)
    };
    let (u1, u2) = (unit(fica1), unit(fica2));
    let cos = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0);
    let meio_angulo = cos.acos() / 2.0;
    let (t1, t2, arco) = if raio <= 0.0 {
        (x, x, None)
    } else {
        let d = raio / meio_angulo.tan();
        if !d.is_finite() || d > fica1.distancia(x) + EPS || d > fica2.distancia(x) + EPS {
            return Err(Recusa::RaioGrande);
        }
        let (t1, t2) = (Ponto::novo(x.x + u1.0 * d, x.y + u1.1 * d), Ponto::novo(x.x + u2.0 * d, x.y + u2.1 * d));
        let (bx, by) = (u1.0 + u2.0, u1.1 + u2.1);
        let bn = bx.hypot(by).max(1e-30);
        let dc = raio / meio_angulo.sin();
        let centro = Ponto::novo(x.x + bx / bn * dc, x.y + by / bn * dc);
        let (a_t1, a_t2) = (centro.angulo_ate(t1), centro.angulo_ate(t2));
        // O arco curto: anti-horário de um tangente ao outro, se der menos de meia volta.
        let (inicio, fim) = if varredura_graus(a_t1, a_t2) <= 180.0 { (a_t1, a_t2) } else { (a_t2, a_t1) };
        (t1, t2, Some(Geometria::Arco { centro, raio, inicio, fim }))
    };
    let mut ms = vec![Mudanca::Trocar(id1, Geometria::Linha { a: fica1, b: t1 }), Mudanca::Trocar(id2, Geometria::Linha { a: fica2, b: t2 })];
    if let Some(g) = arco {
        ms.push(Mudanca::Novo(NovaEntidade { camada: e1.camada, cor: e1.cor, geometria: g, props: Default::default() }));
    }
    Ok(ms)
}

/// O contorno fechado de uma forma (círculo, polilinha fechada), como laço de vértices com bulge.
fn laco_da_forma(g: &Geometria) -> Option<Vec<Vertice>> {
    match g {
        Geometria::Circulo { centro, raio } => Some(vec![Vertice { p: Ponto::novo(centro.x - raio, centro.y), bulge: 1.0 }, Vertice { p: Ponto::novo(centro.x + raio, centro.y), bulge: 1.0 }]),
        Geometria::Polilinha { vertices, fechada: true } if vertices.len() >= 3 => Some(vertices.clone()),
        _ => None,
    }
}

fn area_do_laco(l: &[Ponto]) -> f64 {
    (0..l.len()).map(|i| (l[i].x * l[(i + 1) % l.len()].y - l[(i + 1) % l.len()].x * l[i].y) / 2.0).sum::<f64>().abs()
}

fn contem(laco: &[Ponto], p: Ponto) -> bool {
    let mut dentro = false;
    for i in 0..laco.len() {
        let (a, b) = (laco[i], laco[(i + 1) % laco.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            dentro = !dentro;
        }
    }
    dentro
}

/// A hachura do menor contorno fechado (círculo ou polilinha fechada) que contém `p`; os contornos
/// fechados que ficam dentro dele viram ilhas (par-ímpar). `solida` decide sólida ou linhas a 45°.
pub fn hachura_no_ponto(doc: &Documento, p: Ponto, solida: bool) -> Result<Geometria, Recusa> {
    hachura_com_padrao(doc, p, if solida { crate::padroes::SOLIDA } else { crate::padroes::PADRAO_DE_FABRICA }, 1.0)
}

/// Como `hachura_no_ponto`, com o padrão da biblioteca (`padroes::NOMES`, ou `SOLID`) e a escala dele.
pub fn hachura_com_padrao(doc: &Documento, p: Ponto, nome: &str, escala: f64) -> Result<Geometria, Recusa> {
    let mut candidatos: Vec<(Vec<Vertice>, Vec<Ponto>)> = doc
        .entidades()
        .iter()
        .filter(|e| doc.visivel(e))
        .filter_map(|e| laco_da_forma(&e.geometria))
        .map(|v| {
            let pts = pontos_da_polilinha(&v, true);
            (v, pts)
        })
        .collect();
    // Além de círculo e polilinha fechada, vale a área fechada por traços soltos (quatro linhas formando
    // um quadrado, linhas e arcos): entra se for menor que o menor contorno fechado que já contém o ponto.
    let menor_fechado = candidatos.iter().filter(|(_, pts)| contem(pts, p)).map(|(_, pts)| area_do_laco(pts)).min_by(f64::total_cmp);
    if let Some(pts) = regiao_em(doc, p) {
        if menor_fechado.map_or(true, |m| area_do_laco(&pts) < m * (1.0 - 1e-6)) {
            candidatos.push((pts.iter().map(|q| Vertice { p: *q, bulge: 0.0 }).collect(), pts));
        }
    }
    let (externo, pts_externo) = candidatos
        .iter()
        .filter(|(_, pts)| contem(pts, p))
        .min_by(|a, b| area_do_laco(&a.1).total_cmp(&area_do_laco(&b.1)))
        .ok_or(Recusa::SemContorno)?;
    let mut contornos = vec![externo.clone()];
    contornos.extend(candidatos.iter().filter(|(v, pts)| v != externo && pts.first().is_some_and(|q| contem(pts_externo, *q)) && area_do_laco(pts) < area_do_laco(pts_externo)).map(|(v, _)| v.clone()));
    let (padrao, nome) = crate::padroes::para_contornos(nome, escala, &contornos);
    Ok(Geometria::Hachura { contornos, padrao, nome })
}

/// A caixa de uma seleção de ids (para enquadrar/mostrar).
pub fn caixa_dos(doc: &Documento, ids: &[Id]) -> Option<(Ponto, Ponto)> {
    ids.iter().filter_map(|id| doc.entidade(*id)).filter_map(|e| caixa_da_entidade(doc, &e.geometria)).reduce(|(a, b), (c, d)| (Ponto::novo(a.x.min(c.x), a.y.min(c.y)), Ponto::novo(b.x.max(d.x), b.y.max(d.y))))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    fn geometrias(d: &Documento) -> Vec<Geometria> {
        d.entidades().iter().map(|e| e.geometria.clone()).collect()
    }

    #[test]
    fn aparar_linha_no_trecho_clicado_entre_dois_cortes_e_na_ponta() {
        let mut d = Documento::default();
        let alvo = d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
        d.adicionar_linha(p(3.0, -1.0), p(3.0, 1.0));
        d.adicionar_linha(p(7.0, -1.0), p(7.0, 1.0));
        // No meio (entre 3 e 7): sobram [0,3] e [7,10].
        let ms = aparar(&d, alvo, p(5.0, 0.0)).unwrap();
        d.aplicar_mudancas(ms);
        let g = geometrias(&d);
        assert!(g.contains(&Geometria::Linha { a: p(0.0, 0.0), b: p(3.0, 0.0) }) && g.contains(&Geometria::Linha { a: p(7.0, 0.0), b: p(10.0, 0.0) }), "{g:?}");
        assert_eq!(d.entidades().len(), 4);
        assert!(d.desfazer());
        assert_eq!(d.entidades().len(), 3, "um Ctrl+Z desfaz o aparar inteiro");
        // Na ponta (depois do 7): sobra [0,7].
        let ms = aparar(&d, alvo, p(9.0, 0.0)).unwrap();
        d.aplicar_mudancas(ms);
        assert!(geometrias(&d).contains(&Geometria::Linha { a: p(0.0, 0.0), b: p(7.0, 0.0) }));
    }

    #[test]
    fn aparar_sem_nada_que_cruze_recusa_e_circulo_vira_arco() {
        let mut d = Documento::default();
        let l = d.adicionar_linha(p(0.0, 0.0), p(1.0, 0.0));
        assert_eq!(aparar(&d, l, p(0.5, 0.0)), Err(Recusa::SemCorte));
        let mut d = Documento::default();
        let c = d.adicionar_circulo(p(0.0, 0.0), 5.0).unwrap();
        d.adicionar_linha(p(-6.0, 0.0), p(6.0, 0.0));
        // A linha corta o círculo em 0° e 180°; clicando no alto (90°) some a metade de cima e sobra a de baixo.
        let ms = aparar(&d, c, p(0.0, 5.0)).unwrap();
        d.aplicar_mudancas(ms);
        let Geometria::Arco { inicio, fim, .. } = d.entidade(c).unwrap().geometria else { panic!() };
        assert!((inicio - 180.0).abs() < 1e-6 && (fim - 0.0).abs().min((fim - 360.0).abs()) < 1e-6, "{inicio} {fim}");
    }

    #[test]
    fn aparar_polilinha_aberta_e_fechada() {
        let mut d = Documento::default();
        let ret = d.adicionar_polilinha(vec![Vertice { p: p(0.0, 0.0), bulge: 0.0 }, Vertice { p: p(4.0, 0.0), bulge: 0.0 }, Vertice { p: p(4.0, 4.0), bulge: 0.0 }, Vertice { p: p(0.0, 4.0), bulge: 0.0 }], true).unwrap();
        d.adicionar_linha(p(2.0, -1.0), p(2.0, 5.0));
        // O retângulo cortado em x = 2 (embaixo e em cima): clicar na lateral direita tira o trecho entre os cortes (o lado direito todo).
        let ms = aparar(&d, ret, p(4.0, 2.0)).unwrap();
        d.aplicar_mudancas(ms);
        let Geometria::Polilinha { vertices, fechada } = &d.entidade(ret).unwrap().geometria else { panic!() };
        assert!(!fechada);
        let pts: Vec<(f64, f64)> = vertices.iter().map(|v| (v.p.x, v.p.y)).collect();
        assert_eq!(pts, [(2.0, 4.0), (0.0, 4.0), (0.0, 0.0), (2.0, 0.0)], "sobra a metade esquerda em U");
    }

    #[test]
    fn concordancia_com_raio_zero_encontra_no_canto_e_com_raio_cria_o_arco_tangente() {
        let mut d = Documento::default();
        let a = d.adicionar_linha(p(0.0, 0.0), p(8.0, 0.0));
        let b = d.adicionar_linha(p(10.0, 2.0), p(10.0, 10.0));
        let ms = concordar(&d, (a, p(1.0, 0.0)), (b, p(10.0, 9.0)), 0.0).unwrap();
        d.aplicar_mudancas(ms);
        assert_eq!(d.entidade(a).unwrap().geometria, Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) });
        assert_eq!(d.entidade(b).unwrap().geometria, Geometria::Linha { a: p(10.0, 10.0), b: p(10.0, 0.0) });
        let mut d = Documento::default();
        let a = d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
        let b = d.adicionar_linha(p(10.0, 0.0), p(10.0, 10.0));
        let ms = concordar(&d, (a, p(1.0, 0.0)), (b, p(10.0, 9.0)), 2.0).unwrap();
        d.aplicar_mudancas(ms);
        let g = geometrias(&d);
        assert!(g.contains(&Geometria::Linha { a: p(0.0, 0.0), b: p(8.0, 0.0) }), "{g:?}");
        let Geometria::Arco { centro, raio, inicio, fim } = g[2] else { panic!("{g:?}") };
        assert!((centro.x - 8.0).abs() < 1e-9 && (centro.y - 2.0).abs() < 1e-9 && raio == 2.0);
        assert!((inicio - 270.0).abs() < 1e-6 && (fim - 0.0).abs().min((fim - 360.0).abs()) < 1e-6, "{inicio} {fim}");
        assert_eq!(concordar(&d, (a, p(1.0, 0.0)), (b, p(10.0, 9.0)), 50.0), Err(Recusa::RaioGrande));
        let mut d = Documento::default();
        let (a, b) = (d.adicionar_linha(p(0.0, 0.0), p(1.0, 0.0)), d.adicionar_linha(p(0.0, 1.0), p(1.0, 1.0)));
        assert_eq!(concordar(&d, (a, p(0.0, 0.0)), (b, p(0.0, 1.0)), 0.0), Err(Recusa::Paralelas));
    }

    #[test]
    fn hachura_no_ponto_pega_o_menor_contorno_e_ilhas() {
        let mut d = Documento::default();
        d.adicionar_circulo(p(0.0, 0.0), 10.0);
        d.adicionar_circulo(p(0.0, 0.0), 3.0);
        assert_eq!(hachura_no_ponto(&d, p(50.0, 0.0), true), Err(Recusa::SemContorno));
        // Entre os círculos: contorno externo r = 10, ilha r = 3.
        let Geometria::Hachura { contornos, padrao, .. } = hachura_no_ponto(&d, p(6.0, 0.0), false).unwrap() else { panic!() };
        assert_eq!(contornos.len(), 2);
        assert_eq!(padrao.len(), 1);
        // Dentro do menor: só ele.
        let Geometria::Hachura { contornos, padrao, .. } = hachura_no_ponto(&d, p(1.0, 0.0), true).unwrap() else { panic!() };
        assert_eq!((contornos.len(), padrao.len()), (1, 0));
    }
}
