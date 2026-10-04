//! Nuvem de pontos de um levantamento linear (via férrea, estrada): rastrear o eixo, achar os pares de trilho por
//! seção transversal e apontar o que é suspeito (cota fora do normal, lacunas, cotas copiadas em série).

use crate::geom::Ponto;
use crate::trajeto::Trajeto;

/// Rastreia a curva principal da nuvem: avança 10 m por vez, recentra pelo centroide dos pontos vizinhos e salta
/// lacunas por um cone à frente. `inicio`/`direcao` (graus anti-horários do eixo X) são opcionais; sem eles começa
/// na ponta da nuvem ao longo do eixo principal.
pub fn rastrear_eixo(pontos: &[Ponto], inicio: Option<Ponto>, direcao: Option<f64>) -> Vec<Ponto> {
    if pontos.len() < 3 {
        return Vec::new();
    }
    let n = pontos.len() as f64;
    let (mx, my) = (pontos.iter().map(|p| p.x).sum::<f64>() / n, pontos.iter().map(|p| p.y).sum::<f64>() / n);
    let (sxx, syy, sxy) = pontos.iter().fold((0.0, 0.0, 0.0), |a, p| (a.0 + (p.x - mx).powi(2), a.1 + (p.y - my).powi(2), a.2 + (p.x - mx) * (p.y - my)));
    let eixo_principal = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let u = (eixo_principal.cos(), eixo_principal.sin());
    let proj = |p: &Ponto| (p.x - mx) * u.0 + (p.y - my) * u.1;
    let (menor, maior) = pontos.iter().fold((pontos[0], pontos[0]), |(a, b), p| (if proj(p) < proj(&a) { *p } else { a }, if proj(p) > proj(&b) { *p } else { b }));
    let (comeco, fim) = match inicio {
        Some(i) => (i, if i.distancia(maior) > i.distancia(menor) { maior } else { menor }),
        None => (menor, maior),
    };
    let dir0 = direcao.unwrap_or_else(|| (fim.y - comeco.y).atan2(fim.x - comeco.x).to_degrees()).to_radians();
    let (mut c, mut dir) = (comeco, (dir0.cos(), dir0.sin()));
    let mut eixo = vec![c];
    for _ in 0..5000 {
        let q0 = Ponto::novo(c.x + dir.0 * 10.0, c.y + dir.1 * 10.0);
        let vizinhos = [25.0, 50.0, 90.0].into_iter().find_map(|r| {
            let v: Vec<&Ponto> = pontos.iter().filter(|p| p.distancia(q0) < r && (p.x - c.x) * dir.0 + (p.y - c.y) * dir.1 > -2.0).collect();
            (v.len() >= 3).then_some(v)
        });
        match vizinhos {
            Some(v) => {
                let cen = (v.iter().map(|p| p.x).sum::<f64>() / v.len() as f64, v.iter().map(|p| p.y).sum::<f64>() / v.len() as f64);
                let nrm = (-dir.1, dir.0);
                let lat = (cen.0 - q0.x) * nrm.0 + (cen.1 - q0.y) * nrm.1;
                let nq = Ponto::novo(q0.x + nrm.0 * lat, q0.y + nrm.1 * lat);
                let (dx, dy) = (nq.x - c.x, nq.y - c.y);
                let l = dx.hypot(dy).max(1e-12);
                let (ex, ey) = (0.5 * dir.0 + 0.5 * dx / l, 0.5 * dir.1 + 0.5 * dy / l);
                let le = ex.hypot(ey).max(1e-12);
                dir = (ex / le, ey / le);
                c = nq;
                eixo.push(c);
                if c.distancia(fim) < 8.0 {
                    break;
                }
            }
            None => {
                let salto = pontos
                    .iter()
                    .filter_map(|p| {
                        let (vx, vy) = (p.x - c.x, p.y - c.y);
                        let t = vx * dir.0 + vy * dir.1;
                        let lat = (vx * dir.1 - vy * dir.0).abs();
                        (t > 5.0 && lat < 0.5 * t + 10.0).then_some((t, *p))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                let Some((_, q)) = salto else { break };
                let l = c.distancia(q).max(1e-12);
                dir = ((q.x - c.x) / l, (q.y - c.y) / l);
                c = q;
                eixo.push(c);
            }
        }
    }
    eixo
}

/// Um par de trilhos numa seção transversal.
#[derive(Clone, Debug, PartialEq)]
pub struct Par {
    /// Distância ao longo do eixo de referência.
    pub s: f64,
    /// Ponto médio do par (o eixo existente).
    pub meio: Ponto,
    /// Cota média.
    pub z: f64,
    /// Nivelamento transversal: cota do trilho esquerdo menos a do direito.
    pub nivelamento: f64,
    /// Distância medida entre os dois pontos (eixo a eixo dos boletos).
    pub bitola: f64,
    pub esquerdo: Ponto,
    pub direito: Ponto,
}

#[derive(Clone, Debug, Default)]
pub struct Feicoes {
    pub secoes: usize,
    pub pares: Vec<Par>,
    /// `s` das seções onde não se achou par.
    pub sem_par: Vec<f64>,
}

/// Agrupa os pontos em seções (a distância ao longo do eixo muda < `agrupar` m entre vizinhos) e acha em cada uma o
/// par a `centro ± tolerancia` de distância, ambos a menos de `afast_max` do eixo, preferindo o mais próximo de
/// `centro` e o mais centrado.
pub fn pares_de_trilho(pontos: &[Ponto], eixo: &Trajeto, centro: f64, tolerancia: f64, afast_max: f64, agrupar: f64) -> Feicoes {
    let proj: Vec<(f64, f64)> = pontos.iter().map(|p| eixo.projetar(*p)).collect();
    let mut ordem: Vec<usize> = (0..pontos.len()).collect();
    ordem.sort_by(|a, b| proj[*a].1.total_cmp(&proj[*b].1));
    let mut grupos: Vec<Vec<usize>> = Vec::new();
    for i in ordem {
        match grupos.last_mut() {
            Some(g) if proj[i].1 - proj[*g.last().unwrap_or(&i)].1 < agrupar => g.push(i),
            _ => grupos.push(vec![i]),
        }
    }
    let mut f = Feicoes { secoes: grupos.len(), ..Feicoes::default() };
    for g in &grupos {
        let mut melhor: Option<(f64, usize, usize, f64)> = None;
        for (ia, &a) in g.iter().enumerate() {
            for &b in &g[ia + 1..] {
                let dd = pontos[a].distancia(pontos[b]);
                let (oa, ob) = (proj[a].0, proj[b].0);
                if (dd - centro).abs() < tolerancia && oa.abs() < afast_max && ob.abs() < afast_max {
                    let sc = (dd - centro).abs() + ((oa + ob) / 2.0).abs() * 0.05;
                    if melhor.map_or(true, |m| sc < m.0) {
                        melhor = Some((sc, a, b, dd));
                    }
                }
            }
        }
        match melhor {
            Some((_, a, b, dd)) => {
                let (esq, dir) = if proj[a].0 < proj[b].0 { (b, a) } else { (a, b) };
                f.pares.push(Par {
                    s: proj[esq].1, meio: Ponto::novo((pontos[a].x + pontos[b].x) / 2.0, (pontos[a].y + pontos[b].y) / 2.0), z: (pontos[a].z + pontos[b].z) / 2.0,
                    nivelamento: pontos[esq].z - pontos[dir].z, bitola: dd, esquerdo: pontos[esq], direito: pontos[dir],
                });
            }
            None => f.sem_par.push(proj[g[0]].1),
        }
    }
    f
}

/// Uma das bordas da plataforma numa seção: a crista (fim da parte plana) e, se a nuvem alcança, o pé do talude.
#[derive(Clone, Debug, PartialEq)]
pub struct Borda {
    /// Distância ao longo do eixo de referência.
    pub s: f64,
    /// +1 = esquerda do eixo, −1 = direita.
    pub lado: i8,
    /// Afastamento (m, sempre positivo) e ponto da crista.
    pub afastamento: f64,
    pub crista: Ponto,
    pub pe: Option<Ponto>,
    pub pe_afastamento: Option<f64>,
}

/// Parâmetros de [`bordos_de_plataforma`].
#[derive(Clone, Copy, Debug)]
pub struct ParamBordos {
    /// A plataforma é o que está a no máximo isto da cota do eixo (m).
    pub tolerancia_z: f64,
    /// Buraco máximo entre pontos seguidos do mesmo lado, em afastamento (m).
    pub buraco: f64,
    /// Afastamento máximo (m).
    pub afast_max: f64,
    /// Inclinação (dz/dd) abaixo da qual o talude "acabou" (pé).
    pub inclinacao_pe: f64,
    /// Distância entre seções (m).
    pub agrupar: f64,
}

impl Default for ParamBordos {
    fn default() -> Self {
        Self { tolerancia_z: 0.15, buraco: 3.0, afast_max: 15.0, inclinacao_pe: 0.1, agrupar: 3.0 }
    }
}

/// Em cada seção, de cada lado do eixo: caminha para fora enquanto a cota fica a `tolerancia_z` da do eixo (a crista é o
/// último ponto assim) e depois enquanto a cota desce (o pé é onde a inclinação cai abaixo de `inclinacao_pe`, ou o último
/// ponto que desce). Seção sem ponto a menos de 1 m do eixo, ou sem plataforma além dele, não dá borda desse lado.
pub fn bordos_de_plataforma(pontos: &[Ponto], eixo: &Trajeto, par: &ParamBordos) -> Vec<Borda> {
    let proj: Vec<(f64, f64)> = pontos.iter().map(|p| eixo.projetar(*p)).collect();
    let mut ordem: Vec<usize> = (0..pontos.len()).collect();
    ordem.sort_by(|a, b| proj[*a].1.total_cmp(&proj[*b].1));
    let mut grupos: Vec<Vec<usize>> = Vec::new();
    for i in ordem {
        match grupos.last_mut() {
            Some(g) if proj[i].1 - proj[*g.last().unwrap_or(&i)].1 < par.agrupar => g.push(i),
            _ => grupos.push(vec![i]),
        }
    }
    let mut bordas = Vec::new();
    for g in &grupos {
        let mut centrais: Vec<f64> = g.iter().filter(|&&i| proj[i].0.abs() <= 1.0).map(|&i| pontos[i].z).collect();
        if centrais.is_empty() {
            continue;
        }
        let z0 = mediana(&mut centrais);
        let s = g.iter().map(|&i| proj[i].1).sum::<f64>() / g.len() as f64;
        for lado in [1i8, -1] {
            let mut lado_pts: Vec<usize> = g.iter().copied().filter(|&i| f64::from(lado) * proj[i].0 > 1.0 && proj[i].0.abs() <= par.afast_max).collect();
            lado_pts.sort_by(|a, b| proj[*a].0.abs().total_cmp(&proj[*b].0.abs()));
            let (mut ultimo_d, mut crista) = (0.0, None);
            let mut resto = 0;
            for (k, &i) in lado_pts.iter().enumerate() {
                let d = proj[i].0.abs();
                if d - ultimo_d > par.buraco || (pontos[i].z - z0).abs() > par.tolerancia_z {
                    resto = k;
                    break;
                }
                crista = Some(i);
                ultimo_d = d;
                resto = k + 1;
            }
            let Some(c) = crista else { continue };
            let (mut pe, mut anterior, mut desceu) = (None, c, false);
            for &i in &lado_pts[resto..] {
                let (d, da) = (proj[i].0.abs(), proj[anterior].0.abs());
                if d - da > par.buraco || pontos[i].z > pontos[anterior].z + 0.05 {
                    break;
                }
                let inclinacao = (pontos[anterior].z - pontos[i].z) / (d - da).max(1e-9);
                if desceu && inclinacao < par.inclinacao_pe {
                    break;
                }
                desceu = desceu || pontos[c].z - pontos[i].z >= 0.3;
                if desceu {
                    pe = Some(i);
                }
                anterior = i;
            }
            bordas.push(Borda { s, lado, afastamento: proj[c].0.abs(), crista: pontos[c], pe: pe.map(|i| pontos[i]), pe_afastamento: pe.map(|i| proj[i].0.abs()) });
        }
    }
    bordas
}

/// O que há de suspeito nos pares.
#[derive(Clone, Debug, Default)]
pub struct Qualidade {
    /// (índice do par, cota, mediana local).
    pub fora_da_mediana: Vec<(usize, f64, f64)>,
    /// (de, ate) em `s` onde a distância entre pares passa do limite.
    pub lacunas: Vec<(f64, f64)>,
    /// (primeiro índice, último índice) de pares seguidos com nivelamento exatamente zero.
    pub cotas_identicas: Vec<(usize, usize)>,
    pub nivelamento_zero: usize,
    /// Pares por 100 m: (início do trecho, quantos).
    pub densidade: Vec<(f64, usize)>,
}

fn mediana(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// `desvio`: cota além disto da mediana local (±`janela` vizinhos) é suspeita; `lacuna`: m entre pares; `serie`: quantos
/// pares seguidos com nivelamento zero contam como cota copiada.
pub fn qualidade(pares: &[Par], desvio: f64, janela: usize, lacuna: f64, serie: usize) -> Qualidade {
    let mut q = Qualidade::default();
    for i in 0..pares.len() {
        let (a, b) = (i.saturating_sub(janela), (i + janela + 1).min(pares.len()));
        let mut v: Vec<f64> = (a..b).filter(|j| *j != i).map(|j| pares[j].z).collect();
        if v.len() >= 3 {
            let m = mediana(&mut v);
            if (pares[i].z - m).abs() > desvio {
                q.fora_da_mediana.push((i, pares[i].z, m));
            }
        }
    }
    for j in pares.windows(2) {
        if j[1].s - j[0].s > lacuna {
            q.lacunas.push((j[0].s, j[1].s));
        }
    }
    let zero = |p: &Par| p.nivelamento.abs() < 1e-9;
    q.nivelamento_zero = pares.iter().filter(|p| zero(p)).count();
    let mut i = 0;
    while i < pares.len() {
        if zero(&pares[i]) {
            let mut j = i;
            while j + 1 < pares.len() && zero(&pares[j + 1]) {
                j += 1;
            }
            if j - i + 1 >= serie {
                q.cotas_identicas.push((i, j));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    if let (Some(p0), Some(pn)) = (pares.first(), pares.last()) {
        let n = ((pn.s - p0.s) / 100.0).ceil().max(1.0) as usize;
        q.densidade = (0..n).map(|k| (p0.s + k as f64 * 100.0, pares.iter().filter(|p| p.s >= p0.s + k as f64 * 100.0 && p.s < p0.s + (k + 1) as f64 * 100.0).count())).collect();
    }
    q
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Uma via reta de 400 m ao longo de X com trilhos a ±0,53 m e um ponto de lixo.
    fn via_reta() -> Vec<Ponto> {
        let mut v = Vec::new();
        for i in 0..80 {
            let x = f64::from(i) * 5.0;
            v.push(Ponto::novo3d(x, 0.53, 10.0));
            v.push(Ponto::novo3d(x, -0.53, 10.0 + if i == 40 { 0.8 } else { 0.0 }));
        }
        v.push(Ponto::novo3d(200.0, 3.0, 12.0));
        v
    }

    #[test]
    fn rastreia_acha_pares_e_aponta_suspeitos() {
        let pts = via_reta();
        let eixo = Trajeto::novo(&rastrear_eixo(&pts, Some(Ponto::novo(0.0, 0.0)), Some(0.0))).unwrap();
        assert!(eixo.comprimento() > 350.0, "{}", eixo.comprimento());
        let f = pares_de_trilho(&pts, &eixo, 1.06, 0.1, 3.5, 3.0);
        assert!(f.pares.len() >= 78, "{} pares em {} seções", f.pares.len(), f.secoes);
        assert!((f.pares[10].bitola - 1.06).abs() < 1e-9);
        let q = qualidade(&f.pares, 0.3, 3, 20.0, 4);
        assert!(q.nivelamento_zero > 70 && q.cotas_identicas.len() == 2, "{:?}", q.cotas_identicas);
        assert_eq!(q.fora_da_mediana.len(), 1, "o par com +0,8 num trilho (média +0,4) passa do limite de 0,3");
        assert!(q.lacunas.is_empty());
    }

    /// Seção com plataforma plana até 3 m de cada lado e talude 1:2 descendo 2 m.
    fn secoes_com_talude() -> Vec<Ponto> {
        let mut v = Vec::new();
        for i in 0..10 {
            let x = f64::from(i) * 5.0;
            for lado in [1.0, -1.0] {
                for d in [0.5, 1.5, 2.5, 3.0] {
                    v.push(Ponto::novo3d(x, lado * d, 10.0));
                }
                for k in 1..=4 {
                    v.push(Ponto::novo3d(x, lado * (3.0 + f64::from(k)), 10.0 - f64::from(k) * 0.5));
                }
                v.push(Ponto::novo3d(x, lado * 9.0, 8.0));
            }
        }
        v
    }

    #[test]
    fn acha_crista_e_pe_do_talude_nos_dois_lados() {
        let pts = secoes_com_talude();
        let eixo = Trajeto::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(50.0, 0.0)]).unwrap();
        let b = bordos_de_plataforma(&pts, &eixo, &ParamBordos { agrupar: 1.0, ..ParamBordos::default() });
        assert_eq!(b.len(), 20, "10 seções × 2 lados");
        let e = b.iter().find(|x| x.lado == 1).unwrap();
        assert!((e.afastamento - 3.0).abs() < 1e-9, "crista a 3 m: {}", e.afastamento);
        assert!((e.pe_afastamento.unwrap() - 7.0).abs() < 1e-9, "pé onde acaba a descida (7 m, cota 8): {:?}", e.pe_afastamento);
        let d = b.iter().find(|x| x.lado == -1).unwrap();
        assert!((d.crista.y + 3.0).abs() < 1e-9);
    }

    #[test]
    fn lacuna_e_cota_fora_sao_apontadas() {
        let mut pts = via_reta();
        pts.retain(|p| !(100.0..200.0).contains(&p.x) || p.y > 1.0);
        let eixo = Trajeto::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(400.0, 0.0)]).unwrap();
        let f = pares_de_trilho(&pts, &eixo, 1.06, 0.1, 3.5, 3.0);
        let mut pares = f.pares.clone();
        pares[5].z += 2.0;
        let q = qualidade(&pares, 0.3, 3, 20.0, 4);
        assert_eq!(q.lacunas.len(), 1, "{:?}", q.lacunas);
        assert!(q.fora_da_mediana.iter().any(|(i, _, _)| *i == 5), "a cota levantada em 2 m é apontada");
        assert_eq!(q.fora_da_mediana.len(), 2, "mais o par com +0,8 num trilho que já estava na via de teste");
    }
}

#[cfg(test)]
mod teste_com_o_projeto_de_referencia {
    use super::*;

    /// O CSV do projeto ferroviário de 01/10 (1 237 pontos): 332 seções, 298 pares válidos, bitola mediana 1,075.
    /// Só roda onde o arquivo existe (a máquina do usuário).
    #[test]
    fn csv_ferroviario_de_referencia() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let caminho = std::path::Path::new(&home).join("Documentos/FeatherCAD/levantamento_topografico.csv");
        let Ok(texto) = std::fs::read_to_string(caminho) else { return };
        let pts = crate::topografia::ler_nuvem_bruta(&texto).unwrap();
        assert_eq!(pts.len(), 1237);
        let sul = pts.iter().copied().fold(pts[0], |a, p| if p.y < a.y { p } else { a });
        let bruto = rastrear_eixo(&pts, Some(sul), Some(90.0));
        let eixo = Trajeto::novo(&bruto).unwrap();
        let f = pares_de_trilho(&pts, &eixo, 1.06, 0.1, 3.5, 3.0);
        eprintln!("seções {} pares {} sem par {}", f.secoes, f.pares.len(), f.sem_par.len());
        let mut b: Vec<f64> = f.pares.iter().map(|p| p.bitola).collect();
        b.sort_by(f64::total_cmp);
        eprintln!("bitola mediana {:.3}", b[b.len() / 2]);
        assert!((f.secoes as i64 - 332).abs() <= 3, "seções {}", f.secoes);
        assert_eq!(f.pares.len(), 298);
        assert!((b[b.len() / 2] - 1.075).abs() < 0.0006);
    }
}
