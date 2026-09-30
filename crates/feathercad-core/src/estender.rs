//! Estender (EXTEND): leva a ponta clicada de uma linha, arco ou polilinha aberta (trechos retos) até a
//! primeira outra forma que encontra no caminho. As arestas-limite são todas as outras formas visíveis,
//! como no Aparar. Só geometria: devolve a `Mudanca` que a sessão aplica como um passo de desfazer.

use crate::aparar::{cruzamentos, curvas_de, no_arco, Curva, Recusa};
use crate::documento::{Documento, Geometria, Id, Mudanca};
use crate::geom::{varredura_graus, Ponto, Vertice};

const EPS: f64 = 1e-9;

/// A menor distância `t > 0` ao longo do raio `origem + t·dir` (`dir` unitário) até uma das outras formas.
fn ate_o_resto(doc: &Documento, alvo: Id, origem: Ponto, dir: (f64, f64)) -> Option<f64> {
    let cruz = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
    let mut melhor: Option<f64> = None;
    let mut candidato = |t: f64| {
        if t > EPS && melhor.map_or(true, |m| t < m) {
            melhor = Some(t);
        }
    };
    for e in doc.entidades().iter().filter(|e| e.id != alvo && doc.visivel(e)) {
        for curva in curvas_de(&e.geometria) {
            match curva {
                Curva::Seg(c, d) => {
                    let w = (d.x - c.x, d.y - c.y);
                    let den = cruz(dir, w);
                    if den.abs() < 1e-12 * w.0.hypot(w.1).max(1e-30) {
                        continue;
                    }
                    let co = (c.x - origem.x, c.y - origem.y);
                    let (t, s) = (cruz(co, w) / den, cruz(co, dir) / den);
                    if (-EPS..=1.0 + EPS).contains(&s) {
                        candidato(t);
                    }
                }
                Curva::Arco { c, r, ini, varr } => {
                    let f = (origem.x - c.x, origem.y - c.y);
                    let (qb, qc) = (2.0 * (f.0 * dir.0 + f.1 * dir.1), f.0 * f.0 + f.1 * f.1 - r * r);
                    let disc = qb * qb - 4.0 * qc;
                    if disc < 0.0 {
                        continue;
                    }
                    let raiz = disc.sqrt();
                    for t in [(-qb - raiz) / 2.0, (-qb + raiz) / 2.0] {
                        if no_arco(c, ini, varr, Ponto::novo(origem.x + t * dir.0, origem.y + t * dir.1)) {
                            candidato(t);
                        }
                    }
                }
            }
        }
    }
    melhor
}

/// A ponta `ponta`, esticada na direção de `de → ponta` até o limite mais perto.
fn ponta_esticada(doc: &Documento, alvo: Id, de: Ponto, ponta: Ponto) -> Result<Ponto, Recusa> {
    let c = de.distancia(ponta);
    if c < EPS {
        return Err(Recusa::NaoSuportado);
    }
    let dir = ((ponta.x - de.x) / c, (ponta.y - de.y) / c);
    let t = ate_o_resto(doc, alvo, ponta, dir).ok_or(Recusa::SemCorte)?;
    Ok(Ponto::novo(ponta.x + t * dir.0, ponta.y + t * dir.1))
}

/// Estende a forma `alvo` pela ponta mais perto de `clique`.
pub fn estender(doc: &Documento, alvo: Id, clique: Ponto) -> Result<Vec<Mudanca>, Recusa> {
    let e = doc.entidade(alvo).ok_or(Recusa::NaoSuportado)?;
    let nova = match &e.geometria {
        Geometria::Linha { a, b } => {
            if clique.distancia(*a) < clique.distancia(*b) {
                Geometria::Linha { a: ponta_esticada(doc, alvo, *b, *a)?, b: *b }
            } else {
                Geometria::Linha { a: *a, b: ponta_esticada(doc, alvo, *a, *b)? }
            }
        }
        Geometria::Polilinha { vertices, fechada: false } if vertices.len() >= 2 && vertices.iter().all(|v| v.bulge == 0.0) => {
            let n = vertices.len();
            let mut novos = vertices.clone();
            if clique.distancia(vertices[0].p) < clique.distancia(vertices[n - 1].p) {
                novos[0] = Vertice { p: ponta_esticada(doc, alvo, vertices[1].p, vertices[0].p)?, bulge: 0.0 };
            } else {
                novos[n - 1] = Vertice { p: ponta_esticada(doc, alvo, vertices[n - 2].p, vertices[n - 1].p)?, bulge: 0.0 };
            }
            Geometria::Polilinha { vertices: novos, fechada: false }
        }
        Geometria::Arco { centro, raio, inicio, fim } => {
            let varr = varredura_graus(*inicio, *fim);
            let s = varredura_graus(*inicio, centro.angulo_ate(clique));
            // pela ponta do fim se o clique está mais perto dela (a folga do círculo conta meio a meio)
            let pelo_fim = s > varr / 2.0 && s <= varr + (360.0 - varr) / 2.0;
            let circulo = Curva::Arco { c: *centro, r: *raio, ini: 0.0, varr: 360.0 };
            let folga = 360.0 - varr;
            let angulos: Vec<f64> = doc
                .entidades()
                .iter()
                .filter(|o| o.id != alvo && doc.visivel(o))
                .flat_map(|o| curvas_de(&o.geometria))
                .flat_map(|c| cruzamentos(&circulo, &c))
                .map(|p| centro.angulo_ate(p))
                .collect();
            // quanto falta, andando para fora da ponta, até cada cruzamento que está na folga
            let distancia = |a: f64| if pelo_fim { varredura_graus(*fim, a) } else { varredura_graus(a, *inicio) };
            let (_, angulo) = angulos
                .iter()
                .map(|a| (distancia(*a), *a))
                .filter(|(d, _)| *d > 1e-7 && *d < folga - 1e-7)
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .ok_or(Recusa::SemCorte)?;
            if pelo_fim {
                Geometria::Arco { centro: *centro, raio: *raio, inicio: *inicio, fim: angulo }
            } else {
                Geometria::Arco { centro: *centro, raio: *raio, inicio: angulo, fim: *fim }
            }
        }
        _ => return Err(Recusa::NaoSuportado),
    };
    Ok(vec![Mudanca::Trocar(alvo, nova)])
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    fn trocada(doc: &Documento, id: Id, clique: Ponto) -> Result<Geometria, Recusa> {
        estender(doc, id, clique).map(|ms| match &ms[0] {
            Mudanca::Trocar(_, g) => g.clone(),
            _ => panic!("esperava Trocar"),
        })
    }

    #[test]
    fn linha_cresce_pela_ponta_clicada_ate_a_primeira_forma() {
        let mut d = Documento::default();
        let l = d.adicionar_linha(p(0.0, 0.0), p(4.0, 0.0));
        d.adicionar_linha(p(10.0, -5.0), p(10.0, 5.0));
        d.adicionar_linha(p(20.0, -5.0), p(20.0, 5.0));
        d.adicionar_linha(p(-3.0, -5.0), p(-3.0, 5.0));
        assert_eq!(trocada(&d, l, p(3.9, 0.0)), Ok(Geometria::Linha { a: p(0.0, 0.0), b: p(10.0, 0.0) }));
        assert_eq!(trocada(&d, l, p(0.1, 0.0)), Ok(Geometria::Linha { a: p(-3.0, 0.0), b: p(4.0, 0.0) }));
    }

    #[test]
    fn sem_nada_no_caminho_recusa() {
        let mut d = Documento::default();
        let l = d.adicionar_linha(p(0.0, 0.0), p(4.0, 0.0));
        d.adicionar_linha(p(-3.0, -5.0), p(-3.0, 5.0));
        assert_eq!(trocada(&d, l, p(3.9, 0.0)), Err(Recusa::SemCorte));
        assert!(trocada(&d, l, p(-1.0, 0.0)).is_ok());
    }

    #[test]
    fn linha_chega_ao_circulo() {
        let mut d = Documento::default();
        let l = d.adicionar_linha(p(0.0, 0.0), p(2.0, 0.0));
        d.adicionar_circulo(p(10.0, 0.0), 3.0).unwrap();
        assert_eq!(trocada(&d, l, p(2.0, 0.0)), Ok(Geometria::Linha { a: p(0.0, 0.0), b: p(7.0, 0.0) }));
    }

    #[test]
    fn polilinha_aberta_mexe_so_no_vertice_da_ponta() {
        let mut d = Documento::default();
        let pl = d.adicionar_polilinha(vec![Vertice { p: p(0.0, 5.0), bulge: 0.0 }, Vertice { p: p(0.0, 0.0), bulge: 0.0 }, Vertice { p: p(4.0, 0.0), bulge: 0.0 }], false).unwrap();
        d.adicionar_linha(p(10.0, -5.0), p(10.0, 5.0));
        let Ok(Geometria::Polilinha { vertices, .. }) = trocada(&d, pl, p(4.0, 0.0)) else { panic!() };
        assert_eq!(vertices.iter().map(|v| (v.p.x, v.p.y)).collect::<Vec<_>>(), [(0.0, 5.0), (0.0, 0.0), (10.0, 0.0)]);
    }

    #[test]
    fn arco_cresce_ate_a_linha_que_cruza_o_circulo() {
        let mut d = Documento::default();
        let a = d.adicionar_arco(p(0.0, 0.0), 5.0, 0.0, 45.0).unwrap();
        d.adicionar_linha(p(-10.0, 5.0), p(10.0, 5.0));
        // a linha y = 5 toca o círculo no topo (90°): a ponta do fim vai de 45° a 90°
        let Ok(Geometria::Arco { inicio, fim, .. }) = trocada(&d, a, p(3.5, 3.5)) else { panic!() };
        assert!(inicio.abs() < 1e-9 && (fim - 90.0).abs() < 1e-4, "{inicio} {fim}");
    }

    #[test]
    fn nao_estende_o_que_nao_e_curva_aberta() {
        let mut d = Documento::default();
        let c = d.adicionar_circulo(p(0.0, 0.0), 1.0).unwrap();
        assert_eq!(trocada(&d, c, p(1.0, 0.0)), Err(Recusa::NaoSuportado));
    }
}
