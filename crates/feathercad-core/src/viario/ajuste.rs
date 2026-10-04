//! Ajuste do alinhamento de projeto ao eixo existente: PIs iniciais por simplificação do eixo (Douglas–Peucker),
//! raios por ajuste de círculo, e depois busca por coordenadas nos PIs e nos raios minimizando a ripagem
//! (média dos quadrados + 5 % do pior desvio ao quadrado). Cada raio leva o `Ls` mínimo da superelevação.

use crate::geom::Ponto;
use crate::trajeto::Trajeto;

use super::alinhamento::{Alinhamento, EntradaCurva, Falha};
use super::superelevacao::{raio_minimo, transicao_para_raio, Regras};

/// Douglas–Peucker: índices dos vértices que ficam (distância perpendicular > `tol`).
pub fn simplificar(p: &[Ponto], tol: f64) -> Vec<usize> {
    if p.len() < 3 {
        return (0..p.len()).collect();
    }
    let mut manter = vec![false; p.len()];
    manter[0] = true;
    manter[p.len() - 1] = true;
    let mut pilha = vec![(0usize, p.len() - 1)];
    while let Some((a, b)) = pilha.pop() {
        if b <= a + 1 {
            continue;
        }
        let (pa, pb) = (p[a], p[b]);
        let (dx, dy) = (pb.x - pa.x, pb.y - pa.y);
        let l = dx.hypot(dy).max(1e-12);
        let (mut i_max, mut d_max) = (a, 0.0);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let d = ((q.x - pa.x) * dy - (q.y - pa.y) * dx).abs() / l;
            if d > d_max {
                (i_max, d_max) = (i, d);
            }
        }
        if d_max >= tol {
            manter[i_max] = true;
            pilha.push((a, i_max));
            pilha.push((i_max, b));
        }
    }
    manter.iter().enumerate().filter(|(_, m)| **m).map(|(i, _)| i).collect()
}

fn direcao(a: Ponto, b: Ponto) -> f64 {
    (b.y - a.y).atan2(b.x - a.x)
}

fn deflexao(a: Ponto, b: Ponto, c: Ponto) -> f64 {
    (direcao(b, c) - direcao(a, b) + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI
}

fn interseccao(p1: Ponto, d1: (f64, f64), p2: Ponto, d2: (f64, f64)) -> Option<Ponto> {
    let det = d1.0 * -d2.1 - -d2.0 * d1.1;
    if det.abs() < 1e-12 {
        return None;
    }
    let (bx, by) = (p2.x - p1.x, p2.y - p1.y);
    let t = (bx * -d2.1 - -d2.0 * by) / det;
    Some(Ponto::novo(p1.x + d1.0 * t, p1.y + d1.1 * t))
}

/// PIs iniciais: simplifica o eixo (`tol` m), junta vértices seguidos de mesmo sentido ligados por segmento curto
/// (`juntar` m) no PI da interseção das tangentes externas, e remove os PIs de deflexão menor que `min_graus`.
pub fn pis_iniciais(eixo: &[Ponto], tol: f64, juntar: f64, min_graus: f64) -> Vec<Ponto> {
    let v: Vec<Ponto> = simplificar(eixo, tol).into_iter().map(|i| eixo[i]).collect();
    if v.len() < 3 {
        return v;
    }
    let seg: Vec<f64> = v.windows(2).map(|j| j[0].distancia(j[1])).collect();
    let df: Vec<f64> = (1..v.len() - 1).map(|i| deflexao(v[i - 1], v[i], v[i + 1])).collect();
    let mut grupos: Vec<Vec<usize>> = vec![vec![1]];
    for i in 2..v.len() - 1 {
        if df[i - 1].signum() == df[i - 2].signum() && seg[i - 1] < juntar {
            if let Some(g) = grupos.last_mut() {
                g.push(i);
            }
        } else {
            grupos.push(vec![i]);
        }
    }
    let mut pis = vec![v[0]];
    for g in &grupos {
        let (a, b) = (g[0], g[g.len() - 1]);
        if a == b {
            pis.push(v[a]);
            continue;
        }
        let (d1, d2) = ((v[a].x - v[a - 1].x, v[a].y - v[a - 1].y), (v[b + 1].x - v[b].x, v[b + 1].y - v[b].y));
        pis.push(interseccao(v[a], d1, v[b], d2).unwrap_or(v[a]));
    }
    pis.push(v[v.len() - 1]);
    while pis.len() > 3 {
        let (j, menor) = (1..pis.len() - 1).map(|i| (i, deflexao(pis[i - 1], pis[i], pis[i + 1]).abs().to_degrees())).fold((0, f64::MAX), |m, x| if x.1 < m.1 { x } else { m });
        if menor < min_graus {
            pis.remove(j);
        } else {
            break;
        }
    }
    pis
}

/// Raio de cada PI interno pelo ajuste de círculo (Kasa) nos pontos do eixo que estão na zona da curva: perto do PI
/// e a mais de `2 m` das duas retas do polígono. Sem pontos suficientes, 800 m. Limitado a [R_min, 5000].
pub fn raios_iniciais(eixo: &[Ponto], pis: &[Ponto], regras: &Regras) -> Vec<f64> {
    let rmin = raio_minimo(regras);
    (1..pis.len().saturating_sub(1))
        .map(|i| {
            let rho = 0.5 * pis[i].distancia(pis[i - 1]).min(pis[i].distancia(pis[i + 1]));
            let dist_reta = |q: Ponto, a: Ponto, b: Ponto| {
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let t = (((q.x - a.x) * dx + (q.y - a.y) * dy) / (dx * dx + dy * dy).max(1e-12)).clamp(0.0, 1.0);
                q.distancia(Ponto::novo(a.x + dx * t, a.y + dy * t))
            };
            let zona: Vec<Ponto> = eixo.iter().copied().filter(|q| q.distancia(pis[i]) < rho && dist_reta(*q, pis[i - 1], pis[i]) > 2.0 && dist_reta(*q, pis[i], pis[i + 1]) > 2.0).collect();
            let r = if zona.len() >= 3 { kasa(&zona).unwrap_or(800.0) } else { 800.0 };
            r.clamp(rmin + 1.0, 5000.0)
        })
        .collect()
}

/// Raio do círculo de mínimos quadrados (Kasa).
fn kasa(q: &[Ponto]) -> Option<f64> {
    let n = q.len() as f64;
    let (mx, my) = (q.iter().map(|p| p.x).sum::<f64>() / n, q.iter().map(|p| p.y).sum::<f64>() / n);
    let (mut suu, mut svv, mut suv, mut suuu, mut svvv, mut suvv, mut svuu) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in q {
        let (u, v) = (p.x - mx, p.y - my);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
    }
    let det = suu * svv - suv * suv;
    if det.abs() < 1e-12 {
        return None;
    }
    let (b1, b2) = (0.5 * (suuu + suvv), 0.5 * (svvv + svuu));
    let (uc, vc) = ((b1 * svv - b2 * suv) / det, (b2 * suu - b1 * suv) / det);
    Some((uc * uc + vc * vc + (suu + svv) / n).sqrt())
}

/// Uma proposta de alinhamento e quanto ela custa.
struct Proposta {
    custo: f64,
}

fn montar(pis: &[Ponto], raios: &[f64], regras: &Regras) -> Result<Alinhamento, Falha> {
    let entradas: Vec<Option<EntradaCurva>> = raios.iter().map(|r| Some(EntradaCurva { raio: *r, ls: transicao_para_raio(*r, regras) })).collect();
    let a = Alinhamento::calcular(pis, &entradas, 0.0)?;
    if let Some(c) = a.curvas.iter().find(|c| c.lc < regras.lc_min) {
        return Err(Falha::CurvaCurta(c.pi, format!("PI {}: Lc = {:.1} m abaixo do mínimo de {} m", c.pi, c.lc, regras.lc_min)));
    }
    Ok(a)
}

fn custo(eixo: &[Ponto], pis: &[Ponto], raios: &[f64], regras: &Regras) -> Proposta {
    let Ok(a) = montar(pis, raios, regras) else { return Proposta { custo: 1e9 } };
    let Some(t) = Trajeto::novo(&a.amostrar(2.0)) else { return Proposta { custo: 1e9 } };
    let d: Vec<f64> = eixo.iter().map(|q| t.projetar(*q).0).collect();
    let (soma, maior) = d.iter().fold((0.0, 0.0f64), |(s, m), x| (s + x * x, m.max(x.abs())));
    Proposta { custo: soma / d.len().max(1) as f64 + 0.05 * maior * maior }
}

/// O resultado do ajuste.
pub struct Ajuste {
    pub pis: Vec<Ponto>,
    pub raios: Vec<f64>,
    pub alinhamento: Alinhamento,
    /// (distância ao longo do projeto, afastamento assinado do existente; positivo = existente à esquerda) por ponto.
    pub ripagem: Vec<(f64, f64)>,
    pub rms: f64,
    pub maxima: f64,
}

/// Torna viável (tangentes ≥ 0 e Lc ≥ mínimo) reduzindo/aumentando raios.
fn viabilizar(pis: &[Ponto], raios: &mut [f64], regras: &Regras) -> bool {
    let rmin = raio_minimo(regras);
    for _ in 0..200 {
        let entradas: Vec<Option<EntradaCurva>> = raios.iter().map(|r| Some(EntradaCurva { raio: *r, ls: transicao_para_raio(*r, regras) })).collect();
        match Alinhamento::calcular(pis, &entradas, 0.0) {
            Err(Falha::CurvaCurta(i, _)) => raios[i - 1] *= 1.05,
            Err(Falha::Tangente(i, _)) => {
                raios[i - 1] = (raios[i - 1] * 0.95).max(rmin + 1.0);
                if i >= 2 {
                    raios[i - 2] = (raios[i - 2] * 0.95).max(rmin + 1.0);
                }
            }
            Err(Falha::Invalida(_)) => return false,
            Ok(a) => match a.curvas.iter().find(|c| c.lc < regras.lc_min) {
                Some(c) => raios[c.pi - 1] *= 1.05,
                None => return true,
            },
        }
    }
    false
}

/// Ajusta PIs e raios ao eixo existente. `voltas`: rodadas da busca grossa (5 no projeto de referência);
/// `arredondar`: passo do arredondamento do raio abaixo de 1000 m (acima, 10×).
pub fn ajustar(eixo: &[Ponto], pis0: &[Ponto], raios0: Option<&[f64]>, regras: &Regras, voltas: usize, arredondar: f64) -> Result<Ajuste, String> {
    if pis0.len() < 3 {
        return Err("preciso de ao menos 3 PIs (início, um vértice e fim)".into());
    }
    let rmin = raio_minimo(regras);
    let mut pis = pis0.to_vec();
    let mut raios: Vec<f64> = raios0.map_or_else(|| raios_iniciais(eixo, &pis, regras), <[f64]>::to_vec);
    if raios.len() != pis.len() - 2 {
        return Err(format!("{} PIs internos pedem {} raios; vieram {}", pis.len() - 2, pis.len() - 2, raios.len()));
    }
    if !viabilizar(&pis, &mut raios, regras) {
        return Err("não consegui um traçado viável com esses PIs (curvas se sobrepõem): afaste ou funda PIs próximos".into());
    }
    let mut melhor = custo(eixo, &pis, &raios, regras).custo;
    let mover = |pis: &mut Vec<Ponto>, raios: &[f64], melhor: &mut f64, passos: &[f64]| {
        for &passo in passos {
            for i in 1..pis.len() - 1 {
                for (dx, dy) in [(passo, 0.0), (-passo, 0.0), (0.0, passo), (0.0, -passo)] {
                    let mut q = pis.clone();
                    q[i] = Ponto::novo(q[i].x + dx, q[i].y + dy);
                    let c = custo(eixo, &q, raios, regras).custo;
                    if c < *melhor {
                        (*melhor, *pis) = (c, q);
                    }
                }
            }
        }
    };
    // raios candidatos (varredura grossa por curva, como a busca manual fazia)
    let candidatos: Vec<f64> = [236.0, 250.0, 275.0, 300.0, 325.0, 350.0, 375.0, 400.0, 450.0, 500.0, 550.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1200.0, 1500.0, 2000.0, 2500.0, 3000.0, 4000.0, 5000.0].into_iter().filter(|r| *r >= rmin).collect();
    for _ in 0..voltas {
        for j in 0..raios.len() {
            for &r in &candidatos {
                let mut t = raios.clone();
                t[j] = r;
                let c = custo(eixo, &pis, &t, regras).custo;
                if c < melhor {
                    (melhor, raios) = (c, t);
                }
            }
        }
        for fr in [0.08, 0.03, 0.01] {
            for j in 0..raios.len() {
                for f in [1.0 + fr, 1.0 - fr] {
                    let mut t = raios.clone();
                    t[j] = (raios[j] * f).max(rmin + 0.5);
                    let c = custo(eixo, &pis, &t, regras).custo;
                    if c < melhor {
                        (melhor, raios) = (c, t);
                    }
                }
            }
        }
        mover(&mut pis, &raios, &mut melhor, &[8.0, 3.0, 1.5, 0.5, 0.2]);
    }
    // raios redondos e refinamento fino
    let redondo = |r: f64| if r < 1000.0 { (r / arredondar).round() * arredondar } else { (r / (arredondar * 10.0)).round() * arredondar * 10.0 };
    raios = raios.iter().map(|r| redondo(*r).max(rmin.ceil())).collect();
    viabilizar(&pis, &mut raios, regras);
    melhor = custo(eixo, &pis, &raios, regras).custo;
    for _ in 0..voltas.min(4) {
        mover(&mut pis, &raios, &mut melhor, &[2.0, 0.8, 0.3, 0.1]);
        for j in 0..raios.len() {
            let passo = if raios[j] < 1000.0 { arredondar } else { arredondar * 10.0 };
            for dr in [passo, -passo] {
                let mut t = raios.clone();
                t[j] += dr;
                if t[j] < rmin {
                    continue;
                }
                let c = custo(eixo, &pis, &t, regras).custo;
                if c < melhor {
                    (melhor, raios) = (c, t);
                }
            }
        }
    }
    let alinhamento = montar(&pis, &raios, regras).map_err(|f| f.to_string())?;
    let t = Trajeto::novo(&alinhamento.amostrar(0.5)).ok_or("o traçado ficou vazio")?;
    let ripagem: Vec<(f64, f64)> = eixo.iter().map(|q| { let (d, s) = t.projetar(*q); (s, d) }).collect();
    let rms = (ripagem.iter().map(|(_, d)| d * d).sum::<f64>() / ripagem.len().max(1) as f64).sqrt();
    let maxima = ripagem.iter().fold(0.0f64, |m, (_, d)| m.max(d.abs()));
    Ok(Ajuste { pis, raios, alinhamento, ripagem, rms, maxima })
}

/// Ajuste sem PIs prévios: testa vários limites de junção de vértices (mais PIs = mais curvas, ajuste mais justo) e fica
/// com o traçado de menos curvas cuja ripagem RMS cabe em `meta_rms`; se nenhum cabe, o de menor RMS.
/// Devolve também o limite de junção escolhido.
pub fn ajustar_auto(eixo: &[Ponto], regras: &Regras, voltas: usize, arredondar: f64, meta_rms: f64, tol_dp: f64) -> Result<(Ajuste, f64), String> {
    let mut melhor: Option<(f64, Ajuste)> = None;
    for juntar in [160.0, 120.0, 100.0, 80.0, 60.0] {
        let pis = pis_iniciais(eixo, tol_dp, juntar, 1.5);
        if pis.len() < 3 {
            continue;
        }
        let Ok(a) = ajustar(eixo, &pis, None, regras, 2.min(voltas), arredondar) else { continue };
        let cabe = a.rms <= meta_rms;
        let ganha = melhor.as_ref().map_or(true, |(_, m)| a.rms < m.rms);
        if cabe || ganha {
            let fim = cabe;
            melhor = Some((juntar, a));
            if fim {
                break;
            }
        }
    }
    let (juntar, previa) = melhor.ok_or("não consegui ajustar nenhum traçado ao eixo existente")?;
    if voltas <= 2 {
        return Ok((previa, juntar));
    }
    let a = ajustar(eixo, &previa.pis, Some(&previa.raios), regras, voltas, arredondar)?;
    Ok((a, juntar))
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Eixo existente = curva clotoide + circular conhecida, com um pequeno ruído determinístico.
    fn eixo_sintetico() -> (Vec<Ponto>, Alinhamento) {
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(900.0, 0.0), Ponto::novo(900.0 + 900.0 * d.cos(), 900.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        let t = Trajeto::novo(&a.amostrar(5.0)).unwrap();
        let n = (t.comprimento() / 5.0) as usize;
        let eixo = (0..=n).map(|i| { let (p, ang) = t.em(i as f64 * 5.0); let ruido = ((i * 37 % 11) as f64 - 5.0) * 0.01; let r = ang.to_radians(); Ponto::novo(p.x - r.sin() * ruido, p.y + r.cos() * ruido) }).collect();
        (eixo, a)
    }

    #[test]
    fn dp_acha_os_vertices_e_os_pis_saem_perto_dos_originais() {
        let (eixo, _) = eixo_sintetico();
        let pis = pis_iniciais(&eixo, 1.5, 160.0, 1.5);
        assert!(pis.len() >= 3 && pis.len() <= 6, "{} PIs", pis.len());
        let ideal = Ponto::novo(900.0, 0.0);
        assert!(pis.iter().any(|p| p.distancia(ideal) < 40.0), "{pis:?}");
    }

    #[test]
    fn ajuste_recupera_o_raio_e_a_ripagem_fica_pequena() {
        let (eixo, _) = eixo_sintetico();
        let r = Regras { v: 80.0, ..Regras::default() };
        let pis = [eixo[0], Ponto::novo(900.0, 0.0), *eixo.last().unwrap()];
        let a = ajustar(&eixo, &pis, Some(&[350.0]), &r, 2, 5.0).unwrap();
        assert!((a.raios[0] - 400.0).abs() <= 50.0, "R {}", a.raios[0]);
        assert!(a.rms < 1.0 && a.maxima < 3.0, "rms {} max {}", a.rms, a.maxima);
    }

    #[test]
    fn kasa_acha_o_raio_de_um_arco() {
        let q: Vec<Ponto> = (0..30).map(|i| { let t = f64::from(i) * 0.02; Ponto::novo(500.0 * t.cos(), 500.0 * t.sin()) }).collect();
        assert!((kasa(&q).unwrap() - 500.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod teste_com_o_projeto_de_referencia {
    use super::*;
    use crate::nuvem::{pares_de_trilho, rastrear_eixo};

    /// Projeto de 01/10: ripagem RMS 0,182 m e máxima 0,643 m, 12 curvas. Lento (rode `--release -- --ignored`).
    #[test]
    #[ignore = "lento: rode com --release -- --ignored"]
    fn ajuste_do_csv_ferroviario_de_referencia() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let Ok(texto) = std::fs::read_to_string(std::path::Path::new(&home).join("Documentos/FeatherCAD/levantamento_topografico.csv")) else { return };
        let pts = crate::topografia::ler_nuvem_bruta(&texto).unwrap();
        let sul = pts.iter().copied().fold(pts[0], |a, p| if p.y < a.y { p } else { a });
        let eixo0 = Trajeto::novo(&rastrear_eixo(&pts, Some(sul), Some(90.0))).unwrap();
        let f = pares_de_trilho(&pts, &eixo0, 1.06, 0.1, 3.5, 3.0);
        let q = crate::nuvem::qualidade(&f.pares, 0.3, 3, 20.0, 4);
        eprintln!("suspeitos por cota: {:?}", q.fora_da_mediana.iter().map(|(i, z, m)| (f.pares[*i].s.round(), *z, *m)).collect::<Vec<_>>());
        let ruins: std::collections::BTreeSet<usize> = q.fora_da_mediana.iter().map(|x| x.0).collect();
        let existente: Vec<Ponto> = f.pares.iter().enumerate().filter(|(i, _)| !ruins.contains(i)).map(|(_, p)| p.meio).collect();
        let t0 = std::time::Instant::now();
        let (a, juntar) = ajustar_auto(&existente, &Regras::default(), 5, 5.0, 0.25, 1.5).unwrap();
        eprintln!("ajuste em {:?} (juntar {juntar}): {} curvas, comprimento {:.1}, rms {:.3}, max {:.3}, R {:?}", t0.elapsed(), a.raios.len(), a.alinhamento.comprimento, a.rms, a.maxima, a.raios);
        assert!(a.rms < 0.30 && a.maxima < 0.90, "rms {} max {}", a.rms, a.maxima);
    }
}
