//! A região fechada que contém um ponto, formada por **linhas soltas** (um quadrado feito de quatro linhas,
//! um desenho de linhas e arcos que se fecham). Serve à hachura: no AutoCAD, clicar dentro de uma área
//! delimitada por qualquer traço a preenche, não só dentro de círculo ou polilinha fechada.
//!
//! Como: todas as curvas visíveis viram segmentos; cada segmento é cortado onde cruza ou encosta em outro;
//! isso dá um grafo plano. Tirando as pontas soltas, cada volta do grafo é uma face; a resposta é a menor
//! face que contém o ponto. Uma abertura no contorno (por menor que seja, acima da tolerância) deixa a
//! região aberta e nada é encontrado — como no AutoCAD.

use std::collections::{HashMap, HashSet};

use crate::documento::{Documento, Geometria};
use crate::edicao::pontos_da_polilinha;
use crate::geom::{amostrar_arco, varredura_graus, Ponto};

/// Mais segmentos que isto e a busca desiste (proteção; a hachura por ponto não é para desenhos gigantes).
const SEGMENTOS_MAXIMOS: usize = 20_000;

type Segmento = (Ponto, Ponto);

/// Todas as curvas visíveis como segmentos de reta (arcos e círculos em cordas finas).
fn segmentos(doc: &Documento) -> Vec<Segmento> {
    let mut v: Vec<Segmento> = Vec::new();
    for e in doc.entidades().iter().filter(|e| doc.visivel(e)) {
        match &e.geometria {
            Geometria::Linha { a, b } => v.push((*a, *b)),
            Geometria::Polilinha { vertices, fechada } => {
                let pts = pontos_da_polilinha(vertices, *fechada);
                v.extend(pts.windows(2).map(|w| (w[0], w[1])));
                if *fechada && pts.len() > 2 {
                    v.push((pts[pts.len() - 1], pts[0]));
                }
            }
            Geometria::Circulo { centro, raio } => {
                let pts = amostrar_arco(*centro, *raio, 0.0, std::f64::consts::TAU, raio / 200.0);
                v.extend(pts.windows(2).map(|w| (w[0], w[1])));
            }
            Geometria::Arco { centro, raio, inicio, fim } => {
                let pts = amostrar_arco(*centro, *raio, inicio.to_radians(), varredura_graus(*inicio, *fim).to_radians(), raio / 200.0);
                v.extend(pts.windows(2).map(|w| (w[0], w[1])));
            }
            _ => {}
        }
    }
    v.retain(|(a, b)| a.distancia(*b) > 0.0);
    v
}

/// Onde `a→b` encontra `c→d`: os parâmetros `t` (em `a→b`) e `u` (em `c→d`), se cruzam ou encostam.
fn encontro(a: Ponto, b: Ponto, c: Ponto, d: Ponto, folga: f64) -> Option<(f64, f64)> {
    let (rx, ry) = (b.x - a.x, b.y - a.y);
    let (sx, sy) = (d.x - c.x, d.y - c.y);
    let den = rx * sy - ry * sx;
    let comp_r = rx.hypot(ry);
    let comp_s = sx.hypot(sy);
    if den.abs() < 1e-12 * comp_r * comp_s {
        return None; // paralelas (a sobreposição colinear é resolvida pelas pontas, no chamador)
    }
    let (qx, qy) = (c.x - a.x, c.y - a.y);
    let t = (qx * sy - qy * sx) / den;
    let u = (qx * ry - qy * rx) / den;
    let (ft, fu) = (folga / comp_r, folga / comp_s);
    ((-ft..=1.0 + ft).contains(&t) && (-fu..=1.0 + fu).contains(&u)).then_some((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)))
}

fn area_com_sinal(l: &[Ponto]) -> f64 {
    (0..l.len()).map(|i| (l[i].x * l[(i + 1) % l.len()].y - l[(i + 1) % l.len()].x * l[i].y) / 2.0).sum()
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

/// Onde cada segmento é cortado (parâmetros t em 0..1): cruzamentos, encostos em T e pontas de colineares.
/// Varre por x, para não comparar tudo com tudo.
fn cortes_dos_segmentos(segs: &[Segmento], folga: f64) -> Vec<Vec<f64>> {
    let mut ordem: Vec<usize> = (0..segs.len()).collect();
    let min_x = |s: &Segmento| s.0.x.min(s.1.x);
    let max_x = |s: &Segmento| s.0.x.max(s.1.x);
    ordem.sort_by(|&i, &j| min_x(&segs[i]).total_cmp(&min_x(&segs[j])));
    let mut cortes: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; segs.len()];
    for (k, &i) in ordem.iter().enumerate() {
        for &j in &ordem[k + 1..] {
            if min_x(&segs[j]) > max_x(&segs[i]) + folga {
                break;
            }
            let (a, b) = segs[i];
            let (c, d) = segs[j];
            if a.y.min(b.y) > c.y.max(d.y) + folga || a.y.max(b.y) < c.y.min(d.y) - folga {
                continue;
            }
            if let Some((t, u)) = encontro(a, b, c, d, folga) {
                cortes[i].push(t);
                cortes[j].push(u);
            } else {
                // Colineares que se sobrepõem: as pontas de um cortam o outro.
                for (q, (dono, o0, o1)) in [(c, (i, a, b)), (d, (i, a, b)), (a, (j, c, d)), (b, (j, c, d))] {
                    let comp = o0.distancia(o1);
                    let t = ((q.x - o0.x) * (o1.x - o0.x) + (q.y - o0.y) * (o1.y - o0.y)) / (comp * comp);
                    let perto = Ponto::novo(o0.x + t * (o1.x - o0.x), o0.y + t * (o1.y - o0.y));
                    if (0.0..=1.0).contains(&t) && perto.distancia(q) <= folga {
                        cortes[dono].push(t);
                    }
                }
            }
        }
    }
    cortes
}

/// O grafo plano: os nós (pontos únicos, por células de `celula`) e, para cada um, os vizinhos por ângulo
/// (anti-horário). As pontas soltas (grau 1), que não fecham região nenhuma, já saem podadas.
fn grafo(segs: &[Segmento], mut cortes: Vec<Vec<f64>>, celula: f64) -> (Vec<Ponto>, Vec<Vec<usize>>) {
    let mut nos: Vec<Ponto> = Vec::new();
    let mut indice: HashMap<(i64, i64), usize> = HashMap::new();
    let mut no_de = |q: Ponto| -> usize {
        let chave = ((q.x / celula).round() as i64, (q.y / celula).round() as i64);
        *indice.entry(chave).or_insert_with(|| {
            nos.push(q);
            nos.len() - 1
        })
    };
    let mut arestas: HashSet<(usize, usize)> = HashSet::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        let mut ts = std::mem::take(&mut cortes[i]);
        ts.sort_by(f64::total_cmp);
        let pontos: Vec<usize> = ts.iter().map(|t| no_de(Ponto::novo3d(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y), a.z + t * (b.z - a.z)))).collect();
        for w in pontos.windows(2) {
            if w[0] != w[1] {
                arestas.insert((w[0].min(w[1]), w[0].max(w[1])));
            }
        }
    }
    let mut vizinhos: Vec<Vec<usize>> = vec![Vec::new(); nos.len()];
    for &(u, v) in &arestas {
        vizinhos[u].push(v);
        vizinhos[v].push(u);
    }
    let mut fila: Vec<usize> = (0..nos.len()).filter(|&n| vizinhos[n].len() == 1).collect();
    while let Some(n) = fila.pop() {
        if vizinhos[n].len() != 1 {
            continue;
        }
        let m = vizinhos[n].pop().unwrap_or(n);
        vizinhos[m].retain(|&x| x != n);
        if vizinhos[m].len() == 1 {
            fila.push(m);
        }
    }
    for (n, vs) in vizinhos.iter_mut().enumerate() {
        vs.sort_by(|&a, &b| {
            let ang = |m: usize| (nos[m].y - nos[n].y).atan2(nos[m].x - nos[n].x);
            ang(a).total_cmp(&ang(b))
        });
    }
    (nos, vizinhos)
}

/// Anda pelas faces do grafo: a face à esquerda de u→v continua no vizinho de v que vem logo antes de u
/// (sentido horário em torno de v). Devolve as faces de dentro (anti-horárias, área acima de `area_minima`).
fn faces_de_dentro(nos: &[Ponto], vizinhos: &[Vec<usize>], area_minima: f64) -> Vec<(f64, Vec<Ponto>)> {
    let mut visitadas: HashSet<(usize, usize)> = HashSet::new();
    let mut faces: Vec<(f64, Vec<Ponto>)> = Vec::new();
    for u in 0..nos.len() {
        for &v in &vizinhos[u] {
            if visitadas.contains(&(u, v)) {
                continue;
            }
            let mut laco: Vec<usize> = Vec::new();
            let (mut a, mut b) = (u, v);
            loop {
                visitadas.insert((a, b));
                laco.push(a);
                let vs = &vizinhos[b];
                let Some(pos) = vs.iter().position(|&x| x == a) else { break };
                let c = vs[(pos + vs.len() - 1) % vs.len()];
                (a, b) = (b, c);
                if (a, b) == (u, v) || laco.len() > nos.len() * 2 + 2 {
                    break;
                }
            }
            let pts: Vec<Ponto> = laco.iter().map(|&n| nos[n]).collect();
            let area = area_com_sinal(&pts);
            // Anti-horário (área > 0) é face de dentro; a de fora do desenho dá área negativa.
            if area > area_minima {
                faces.push((area, pts));
            }
        }
    }
    faces
}

/// Prepara o grafo do desenho: `(nós, vizinhos, área mínima que conta como região)`; `None` se não há o que analisar.
fn preparar(doc: &Documento) -> Option<(Vec<Ponto>, Vec<Vec<usize>>, f64)> {
    let segs = segmentos(doc);
    if segs.is_empty() || segs.len() > SEGMENTOS_MAXIMOS {
        return None;
    }
    let (mut min, mut max) = (Ponto::novo(f64::MAX, f64::MAX), Ponto::novo(f64::MIN, f64::MIN));
    for (a, b) in &segs {
        for q in [a, b] {
            min = Ponto::novo(min.x.min(q.x), min.y.min(q.y));
            max = Ponto::novo(max.x.max(q.x), max.y.max(q.y));
        }
    }
    let diagonal = min.distancia(max).max(1e-9);
    let cortes = cortes_dos_segmentos(&segs, diagonal * 1e-9);
    let (nos, vizinhos) = grafo(&segs, cortes, (diagonal * 1e-7).max(1e-12));
    Some((nos, vizinhos, diagonal * diagonal * 1e-14))
}

/// A menor região fechada, feita de traços soltos, que contém `p`: o contorno em pontos (sentido anti-horário).
pub fn regiao_em(doc: &Documento, p: Ponto) -> Option<Vec<Ponto>> {
    let (nos, vizinhos, minima) = preparar(doc)?;
    faces_de_dentro(&nos, &vizinhos, minima).into_iter().filter(|(_, pts)| contem(pts, p)).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, pts)| pts)
}

/// Todas as regiões fechadas do desenho (cada volta do grafo de traços), da menor para a maior, com a área.
/// Uma região que tem outra dentro aparece inteira: a área "com o vão" é a dela menos as de dentro.
pub fn todas_as_regioes(doc: &Documento) -> Vec<(f64, Vec<Ponto>)> {
    let Some((nos, vizinhos, minima)) = preparar(doc) else { return Vec::new() };
    let mut faces = faces_de_dentro(&nos, &vizinhos, minima);
    faces.sort_by(|a, b| a.0.total_cmp(&b.0));
    faces
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    fn quadrado(d: &mut Documento, x0: f64, y0: f64, x1: f64, y1: f64) {
        d.adicionar_linha(p(x0, y0), p(x1, y0));
        d.adicionar_linha(p(x1, y0), p(x1, y1));
        d.adicionar_linha(p(x1, y1), p(x0, y1));
        d.adicionar_linha(p(x0, y1), p(x0, y0));
    }

    fn area(l: &[Ponto]) -> f64 {
        area_com_sinal(l).abs()
    }

    #[test]
    fn quadrado_de_quatro_linhas_soltas_e_uma_regiao() {
        let mut d = Documento::default();
        quadrado(&mut d, 0.0, 0.0, 10.0, 10.0);
        let r = regiao_em(&d, p(5.0, 5.0)).expect("região");
        assert!((area(&r) - 100.0).abs() < 1e-6);
        assert!(regiao_em(&d, p(50.0, 5.0)).is_none(), "fora não é região");
    }

    #[test]
    fn linhas_que_se_cruzam_e_encostam_no_meio_fecham_a_regiao() {
        // Lados que passam do canto (X) e um lado que só encosta no meio de outro (T).
        let mut d = Documento::default();
        d.adicionar_linha(p(-2.0, 0.0), p(12.0, 0.0));
        d.adicionar_linha(p(10.0, -2.0), p(10.0, 12.0));
        d.adicionar_linha(p(12.0, 10.0), p(0.0, 10.0));
        d.adicionar_linha(p(0.0, 12.0), p(0.0, -2.0));
        let r = regiao_em(&d, p(5.0, 5.0)).expect("região");
        assert!((area(&r) - 100.0).abs() < 1e-6, "{}", area(&r));
        let mut t = Documento::default();
        quadrado(&mut t, 0.0, 0.0, 10.0, 10.0);
        t.adicionar_linha(p(5.0, 0.0), p(5.0, 10.0)); // divide ao meio, encostando nas linhas de cima e de baixo
        let r = regiao_em(&t, p(2.0, 5.0)).expect("metade");
        assert!((area(&r) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn a_menor_regiao_ganha_e_pontas_soltas_nao_atrapalham() {
        let mut d = Documento::default();
        quadrado(&mut d, 0.0, 0.0, 10.0, 10.0);
        quadrado(&mut d, 2.0, 2.0, 4.0, 4.0);
        d.adicionar_linha(p(6.0, 6.0), p(9.0, 9.0)); // ponta solta dentro do grande
        assert!((area(&regiao_em(&d, p(3.0, 3.0)).unwrap()) - 4.0).abs() < 1e-6);
        assert!(regiao_em(&d, p(8.0, 2.0)).is_some(), "o grande, com a ponta solta ignorada");
    }

    #[test]
    fn todas_as_regioes_lista_cada_volta_fechada() {
        let mut d = Documento::default();
        quadrado(&mut d, 0.0, 0.0, 10.0, 10.0);
        d.adicionar_linha(p(5.0, 0.0), p(5.0, 10.0)); // duas metades
        quadrado(&mut d, 20.0, 0.0, 22.0, 3.0);
        let r = todas_as_regioes(&d);
        let areas: Vec<f64> = r.iter().map(|(a, _)| (a * 1000.0).round() / 1000.0).collect();
        assert_eq!(areas, vec![6.0, 50.0, 50.0], "duas metades e o retângulo separado");
    }

    #[test]
    fn contorno_aberto_nao_fecha_regiao() {
        let mut d = Documento::default();
        d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
        d.adicionar_linha(p(10.0, 0.0), p(10.0, 10.0));
        d.adicionar_linha(p(10.0, 10.0), p(0.0, 10.0));
        d.adicionar_linha(p(0.0, 9.0), p(0.0, 0.0)); // falta o pedaço de 9 a 10
        assert!(regiao_em(&d, p(5.0, 5.0)).is_none());
    }

    #[test]
    fn linhas_e_arco_fecham_uma_regiao_de_forma_livre() {
        let mut d = Documento::default();
        d.adicionar_linha(p(-5.0, 0.0), p(5.0, 0.0));
        d.adicionar_arco(p(0.0, 0.0), 5.0, 0.0, 180.0);
        let r = regiao_em(&d, p(0.0, 2.0)).expect("meia-lua");
        // O arco vira cordas inscritas, então a área sai um pouco menor que a exata (π·25/2 ≈ 39,27): vale 1%.
        let exata = std::f64::consts::PI * 25.0 / 2.0;
        assert!((area(&r) - exata).abs() < exata * 0.01, "{}", area(&r));
    }
}
