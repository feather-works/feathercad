//! Deslocar (OFFSET): uma cópia paralela de linha, círculo, arco ou polilinha, à distância dada e do
//! lado em que o usuário clicou. Só geometria; a sessão cuida do fluxo.
//!
//! Polilinha: só de trechos retos (com arco por bulge, recusa). As quinas ficam em esquadro (encontro
//! das duas paralelas). Quando a distância faz uma polilinha fechada virar do avesso, recusa em vez de
//! devolver um contorno torto.

use crate::aparar::Recusa;
use crate::documento::Geometria;
use crate::geom::{Ponto, Vertice};

const EPS: f64 = 1e-9;

/// De que lado (esquerda = `+1`, direita = `-1`) do segmento `a→b` está `p`.
fn lado_do_segmento(a: Ponto, b: Ponto, p: Ponto) -> f64 {
    let v = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    if v >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

fn distancia_ao_segmento(p: Ponto, a: Ponto, b: Ponto) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let c2 = dx * dx + dy * dy;
    if c2 < EPS * EPS {
        return p.distancia(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / c2).clamp(0.0, 1.0);
    p.distancia(Ponto::novo(a.x + t * dx, a.y + t * dy))
}

/// O deslocamento perpendicular de `distancia` para o lado `s` (`+1` esquerda) do segmento `a→b`.
fn normal(a: Ponto, b: Ponto, s: f64, distancia: f64) -> Option<(f64, f64)> {
    let c = a.distancia(b);
    (c > EPS).then(|| (-(b.y - a.y) / c * s * distancia, (b.x - a.x) / c * s * distancia))
}

/// Onde as retas `p1→p2` e `p3→p4` se cruzam (`None` se paralelas).
fn cruzamento(p1: Ponto, p2: Ponto, p3: Ponto, p4: Ponto) -> Option<Ponto> {
    let (d1x, d1y, d2x, d2y) = (p2.x - p1.x, p2.y - p1.y, p4.x - p3.x, p4.y - p3.y);
    let den = d1x * d2y - d1y * d2x;
    if den.abs() < EPS * (d1x.hypot(d1y) * d2x.hypot(d2y)).max(1.0) {
        return None;
    }
    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / den;
    Some(Ponto::novo(p1.x + t * d1x, p1.y + t * d1y))
}

/// A cópia de `geo` a `distancia` do original, do lado onde está `lado`.
pub fn deslocar(geo: &Geometria, distancia: f64, lado: Ponto) -> Result<Geometria, Recusa> {
    if distancia <= EPS || !distancia.is_finite() {
        return Err(Recusa::NaoSuportado);
    }
    match geo {
        Geometria::Linha { a, b } => {
            let s = lado_do_segmento(*a, *b, lado);
            let (dx, dy) = normal(*a, *b, s, distancia).ok_or(Recusa::NaoSuportado)?;
            Ok(Geometria::Linha { a: Ponto::novo(a.x + dx, a.y + dy), b: Ponto::novo(b.x + dx, b.y + dy) })
        }
        Geometria::Circulo { centro, raio } => {
            let r = raio_novo(*centro, *raio, distancia, lado)?;
            Ok(Geometria::Circulo { centro: *centro, raio: r })
        }
        Geometria::Arco { centro, raio, inicio, fim } => {
            let r = raio_novo(*centro, *raio, distancia, lado)?;
            Ok(Geometria::Arco { centro: *centro, raio: r, inicio: *inicio, fim: *fim })
        }
        Geometria::Polilinha { vertices, fechada } => polilinha(vertices, *fechada, distancia, lado),
        _ => Err(Recusa::NaoSuportado),
    }
}

/// Raio do círculo/arco deslocado: para fora se o lado está além do traço, para dentro se aquém.
fn raio_novo(centro: Ponto, raio: f64, distancia: f64, lado: Ponto) -> Result<f64, Recusa> {
    let r = if centro.distancia(lado) > raio { raio + distancia } else { raio - distancia };
    if r > EPS {
        Ok(r)
    } else {
        Err(Recusa::RaioGrande)
    }
}

fn polilinha(vertices: &[Vertice], fechada: bool, distancia: f64, lado: Ponto) -> Result<Geometria, Recusa> {
    if vertices.iter().any(|v| v.bulge != 0.0) {
        return Err(Recusa::NaoSuportado);
    }
    // vértices repetidos não têm direção: somem
    let mut pts: Vec<Ponto> = Vec::with_capacity(vertices.len());
    for v in vertices {
        if pts.last().map_or(true, |u| u.distancia(v.p) > EPS) {
            pts.push(v.p);
        }
    }
    if fechada && pts.len() > 1 && pts[0].distancia(pts[pts.len() - 1]) <= EPS {
        pts.pop();
    }
    let n = pts.len();
    if n < 2 || (fechada && n < 3) {
        return Err(Recusa::NaoSuportado);
    }
    let trechos = if fechada { n } else { n - 1 };
    // o lado vale o do trecho mais perto do clique
    let mais_perto = (0..trechos)
        .min_by(|&i, &j| {
            let ((ai, bi), (aj, bj)) = (a_de_b(&pts, i), a_de_b(&pts, j));
            distancia_ao_segmento(lado, ai, bi).total_cmp(&distancia_ao_segmento(lado, aj, bj))
        })
        .ok_or(Recusa::NaoSuportado)?;
    let (ma, mb) = a_de_b(&pts, mais_perto);
    let s = lado_do_segmento(ma, mb, lado);
    // cada trecho, empurrado para o mesmo lado
    let mut paralelas = Vec::with_capacity(trechos);
    for i in 0..trechos {
        let (a, b) = a_de_b(&pts, i);
        let (dx, dy) = normal(a, b, s, distancia).ok_or(Recusa::NaoSuportado)?;
        paralelas.push((Ponto::novo(a.x + dx, a.y + dy), Ponto::novo(b.x + dx, b.y + dy)));
    }
    let junta = |(a1, b1): (Ponto, Ponto), (a2, b2): (Ponto, Ponto)| cruzamento(a1, b1, a2, b2).unwrap_or(b1);
    let mut novos = Vec::with_capacity(n);
    if fechada {
        for i in 0..n {
            novos.push(junta(paralelas[(i + n - 1) % n], paralelas[i]));
        }
    } else {
        novos.push(paralelas[0].0);
        for i in 1..trechos {
            novos.push(junta(paralelas[i - 1], paralelas[i]));
        }
        novos.push(paralelas[trechos - 1].1);
    }
    // um trecho que inverteu de direção foi engolido pela distância: o contorno sairia do avesso
    let inverteu = (0..trechos).any(|i| {
        let (a, b) = a_de_b(&pts, i);
        let (na, nb) = (novos[i], novos[(i + 1) % novos.len()]);
        (b.x - a.x) * (nb.x - na.x) + (b.y - a.y) * (nb.y - na.y) < 0.0
    });
    if inverteu {
        return Err(Recusa::RaioGrande);
    }
    Ok(Geometria::Polilinha { vertices: novos.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada })
}

fn a_de_b(pts: &[Ponto], i: usize) -> (Ponto, Ponto) {
    (pts[i], pts[(i + 1) % pts.len()])
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }
    fn v(x: f64, y: f64) -> Vertice {
        Vertice { p: p(x, y), bulge: 0.0 }
    }
    fn perto(a: Ponto, x: f64, y: f64) -> bool {
        a.distancia(p(x, y)) < 1e-9
    }

    #[test]
    fn linha_vai_para_o_lado_do_clique() {
        let l = Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) };
        let Geometria::Linha { a, b } = deslocar(&l, 2.0, p(5.0, 7.0)).unwrap() else { panic!() };
        assert!(perto(a, 0.0, 2.0) && perto(b, 10.0, 2.0));
        let Geometria::Linha { a, b } = deslocar(&l, 2.0, p(5.0, -1.0)).unwrap() else { panic!() };
        assert!(perto(a, 0.0, -2.0) && perto(b, 10.0, -2.0));
    }

    #[test]
    fn circulo_e_arco_crescem_para_fora_e_encolhem_para_dentro() {
        let c = Geometria::Circulo { centro: p(0.0, 0.0), raio: 5.0 };
        assert!(matches!(deslocar(&c, 2.0, p(9.0, 0.0)), Ok(Geometria::Circulo { raio, .. }) if (raio - 7.0).abs() < 1e-9));
        assert!(matches!(deslocar(&c, 2.0, p(1.0, 0.0)), Ok(Geometria::Circulo { raio, .. }) if (raio - 3.0).abs() < 1e-9));
        assert_eq!(deslocar(&c, 5.0, p(1.0, 0.0)), Err(Recusa::RaioGrande));
        let a = Geometria::Arco { centro: p(1.0, 1.0), raio: 4.0, inicio: 10.0, fim: 80.0 };
        assert!(matches!(deslocar(&a, 1.0, p(1.0, 1.0)), Ok(Geometria::Arco { raio, inicio, fim, .. }) if (raio - 3.0).abs() < 1e-9 && inicio == 10.0 && fim == 80.0));
    }

    #[test]
    fn polilinha_aberta_tem_quinas_em_esquadro() {
        // L: (0,0) → (10,0) → (10,10); deslocar para dentro (esquerda) 1
        let l = Geometria::Polilinha { vertices: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)], fechada: false };
        let Geometria::Polilinha { vertices, .. } = deslocar(&l, 1.0, p(5.0, 5.0)).unwrap() else { panic!() };
        assert_eq!(vertices.len(), 3);
        assert!(perto(vertices[0].p, 0.0, 1.0) && perto(vertices[1].p, 9.0, 1.0) && perto(vertices[2].p, 9.0, 10.0));
    }

    #[test]
    fn polilinha_fechada_por_dentro_e_por_fora() {
        let q = Geometria::Polilinha { vertices: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)], fechada: true };
        let Geometria::Polilinha { vertices, fechada } = deslocar(&q, 2.0, p(5.0, 5.0)).unwrap() else { panic!() };
        assert!(fechada && perto(vertices[0].p, 2.0, 2.0) && perto(vertices[2].p, 8.0, 8.0));
        let Geometria::Polilinha { vertices, .. } = deslocar(&q, 2.0, p(20.0, 5.0)).unwrap() else { panic!() };
        assert!(perto(vertices[0].p, -2.0, -2.0) && perto(vertices[2].p, 12.0, 12.0));
        // 6 de um quadrado de 10 engole o contorno
        assert_eq!(deslocar(&q, 6.0, p(5.0, 5.0)), Err(Recusa::RaioGrande));
    }

    #[test]
    fn recusa_o_que_nao_trata() {
        let com_arco = Geometria::Polilinha { vertices: vec![Vertice { p: p(0.0, 0.0), bulge: 0.5 }, v(4.0, 0.0)], fechada: false };
        assert_eq!(deslocar(&com_arco, 1.0, p(0.0, 1.0)), Err(Recusa::NaoSuportado));
        assert_eq!(deslocar(&Geometria::Ponto { p: p(0.0, 0.0) }, 1.0, p(0.0, 1.0)), Err(Recusa::NaoSuportado));
        assert_eq!(deslocar(&Geometria::Linha { a: p(0.0, 0.0), b: p(1.0, 0.0) }, 0.0, p(0.0, 1.0)), Err(Recusa::NaoSuportado));
    }
}
