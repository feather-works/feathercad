//! Greide (perfil longitudinal de projeto) a partir do perfil existente: PIVs por simplificação, retas por mínimos
//! quadrados, levante preferido (nunca rebaixar além de `rebaixo_max`) e curvas verticais parabólicas de raio `Rv`.

use crate::geom::Ponto;

use super::ajuste::simplificar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regra {
    /// Eleva o greide até não haver rebaixo maior que `rebaixo_max` (a via só sobe: lastro por cima).
    SoLevante,
    /// Só os mínimos quadrados, sem levantar.
    MinimosQuadrados,
}

#[derive(Clone, Copy, Debug)]
pub struct Opcoes {
    pub regra: Regra,
    /// Rebaixo máximo tolerado, m (positivo: 0,02 = 2 cm abaixo do existente).
    pub rebaixo_max: f64,
    /// Tolerância da simplificação do perfil em cota, m (escolhe onde há PIVs).
    pub tolerancia: f64,
    /// Raio da curva vertical, m.
    pub rv: f64,
    pub comprimento: f64,
}

impl Default for Opcoes {
    fn default() -> Self {
        Self { regra: Regra::SoLevante, rebaixo_max: 0.02, tolerancia: 0.08, rv: 5000.0, comprimento: 0.0 }
    }
}

/// Um PIV com a curva vertical que o arredonda.
#[derive(Clone, Debug, PartialEq)]
pub struct Piv {
    pub s: f64,
    pub z: f64,
    /// Rampas de chegada e de saída (m/m).
    pub g1: f64,
    pub g2: f64,
    /// Comprimento da curva vertical (0 = sem curva).
    pub lv: f64,
    pub convexa: bool,
}

#[derive(Clone, Debug)]
pub struct Greide {
    pub nos_s: Vec<f64>,
    pub nos_z: Vec<f64>,
    pub pivs: Vec<Piv>,
}

fn resolver(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs())).unwrap_or(c);
        a.swap(c, p);
        b.swap(c, p);
        let piv = if a[c][c].abs() < 1e-14 { 1e-14 } else { a[c][c] };
        let linha_c = a[c].clone();
        for r in c + 1..n {
            let f = a[r][c] / piv;
            if f != 0.0 {
                for (x, y) in a[r][c..].iter_mut().zip(&linha_c[c..]) {
                    *x -= f * y;
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for c in (0..n).rev() {
        let s: f64 = (c + 1..n).map(|k| a[c][k] * x[k]).sum();
        x[c] = (b[c] - s) / if a[c][c].abs() < 1e-14 { 1e-14 } else { a[c][c] };
    }
    x
}

/// Segmento e posição (0..1) de `s` entre os nós.
fn posicao(nos: &[f64], s: f64) -> (usize, f64) {
    let j = nos.partition_point(|k| *k < s).saturating_sub(1).min(nos.len() - 2);
    (j, (s - nos[j]) / (nos[j + 1] - nos[j]).max(1e-12))
}

/// `perfil`: (distância ao longo do alinhamento, cota) do existente, em qualquer ordem.
pub fn ajustar(perfil: &[(f64, f64)], o: &Opcoes) -> Result<Greide, String> {
    if perfil.len() < 3 {
        return Err("o perfil existente precisa de ao menos 3 pontos [distância, cota]".into());
    }
    let mut p = perfil.to_vec();
    p.sort_by(|a, b| a.0.total_cmp(&b.0));
    let comprimento = if o.comprimento > 0.0 { o.comprimento } else { p[p.len() - 1].0 };
    // PIVs: simplificação no espaço (distância, cota em mm), como o ajuste manual fazia.
    let mm: Vec<Ponto> = p.iter().map(|q| Ponto::novo(q.0, q.1 * 1000.0)).collect();
    let mut nos: Vec<f64> = simplificar(&mm, o.tolerancia * 1000.0).into_iter().map(|i| p[i].0).collect();
    nos[0] = 0.0;
    if let Some(u) = nos.last_mut() {
        *u = comprimento;
    }
    nos.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    if nos.len() < 2 {
        return Err("o perfil não tem extensão".into());
    }
    let n = nos.len();
    let pesos: Vec<(usize, f64)> = p.iter().map(|q| posicao(&nos, q.0)).collect();
    let mut ata = vec![vec![0.0; n]; n];
    let mut atb = vec![0.0; n];
    for ((j, t), q) in pesos.iter().zip(&p) {
        let (w0, w1) = (1.0 - t, *t);
        ata[*j][*j] += w0 * w0;
        ata[*j][j + 1] += w0 * w1;
        ata[j + 1][*j] += w0 * w1;
        ata[j + 1][j + 1] += w1 * w1;
        atb[*j] += w0 * q.1;
        atb[j + 1] += w1 * q.1;
    }
    for (i, linha) in ata.iter_mut().enumerate() {
        linha[i] += 1e-9;
    }
    let mut zk = resolver(ata, atb);
    if o.regra == Regra::SoLevante {
        for _ in 0..500 {
            let r = |(j, t): (usize, f64), z: f64, zk: &[f64]| zk[j] * (1.0 - t) + zk[j + 1] * t - z;
            let (i, rmin) = pesos.iter().zip(&p).enumerate().map(|(i, (w, q))| (i, r(*w, q.1, &zk))).fold((0, f64::MAX), |m, x| if x.1 < m.1 { x } else { m });
            if rmin >= -o.rebaixo_max {
                break;
            }
            let (j, t) = pesos[i];
            let (w0, w1) = (1.0 - t, t);
            let k = (-o.rebaixo_max - rmin) / (w0 * w0 + w1 * w1).max(1e-12);
            zk[j] += w0 * k;
            zk[j + 1] += w1 * k;
        }
    }
    let rampas: Vec<f64> = (0..n - 1).map(|i| (zk[i + 1] - zk[i]) / (nos[i + 1] - nos[i]).max(1e-12)).collect();
    let pivs = (1..n - 1)
        .map(|i| {
            let di = rampas[i] - rampas[i - 1];
            let lv = if di.abs() > 1e-4 { (o.rv * di.abs() / 10.0).ceil().mul_add(10.0, 0.0).max(20.0) } else { 0.0 };
            let lv = lv.min(0.9 * (nos[i] - nos[i - 1]).min(nos[i + 1] - nos[i]));
            Piv { s: nos[i], z: zk[i], g1: rampas[i - 1], g2: rampas[i], lv, convexa: di < 0.0 }
        })
        .collect();
    Ok(Greide { nos_s: nos, nos_z: zk, pivs })
}

impl Greide {
    /// Cota de projeto em `s` (com as curvas verticais).
    pub fn z(&self, s: f64) -> f64 {
        let (j, t) = posicao(&self.nos_s, s.clamp(self.nos_s[0], self.nos_s[self.nos_s.len() - 1]));
        let mut z = self.nos_z[j] * (1.0 - t) + self.nos_z[j + 1] * t;
        for p in &self.pivs {
            if p.lv > 0.0 && (s - p.s).abs() < p.lv / 2.0 {
                let x = s - (p.s - p.lv / 2.0);
                let pcv = p.z - p.g1 * p.lv / 2.0;
                z = pcv + p.g1 * x + (p.g2 - p.g1) * x * x / (2.0 * p.lv);
            }
        }
        z
    }

    /// Rampa (m/m) em `s`: a da reta, ou a da parábola dentro de uma curva vertical.
    pub fn rampa(&self, s: f64) -> f64 {
        for p in &self.pivs {
            if p.lv > 0.0 && (s - p.s).abs() < p.lv / 2.0 {
                return p.g1 + (p.g2 - p.g1) * (s - (p.s - p.lv / 2.0)) / p.lv;
            }
        }
        let (j, _) = posicao(&self.nos_s, s.clamp(self.nos_s[0], self.nos_s[self.nos_s.len() - 1]));
        (self.nos_z[j + 1] - self.nos_z[j]) / (self.nos_s[j + 1] - self.nos_s[j]).max(1e-12)
    }
}

/// Levante (projeto − existente) em cada ponto do perfil: (mínimo, médio, máximo).
pub fn estatistica_do_levante(g: &Greide, perfil: &[(f64, f64)]) -> (f64, f64, f64) {
    let l: Vec<f64> = perfil.iter().map(|(s, z)| g.z(*s) - z).collect();
    (l.iter().copied().fold(f64::MAX, f64::min), l.iter().sum::<f64>() / l.len().max(1) as f64, l.iter().copied().fold(f64::MIN, f64::max))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_levante_nunca_rebaixa_mais_que_o_limite_e_a_curva_vertical_suaviza() {
        // existente: sobe 1 m em 500 m, platô, desce; com ruído de ±3 cm
        let perfil: Vec<(f64, f64)> = (0..=100).map(|i| { let s = f64::from(i) * 10.0; let base = if s < 500.0 { s / 500.0 } else if s < 700.0 { 1.0 } else { 1.0 - (s - 700.0) / 300.0 }; (s, 40.0 + base + ((i * 7 % 5) as f64 - 2.0) * 0.015) }).collect();
        let g = ajustar(&perfil, &Opcoes::default()).unwrap();
        let (min, med, max) = estatistica_do_levante(&g, &perfil);
        assert!(min >= -0.05, "rebaixo {min}");
        assert!(med > 0.0 && max < 0.5, "med {med} max {max}");
        assert!(g.pivs.iter().any(|p| p.lv >= 20.0));
        let v = &g.pivs[0];
        assert!((g.z(v.s) - (v.z + (v.g2 - v.g1) * v.lv / 8.0)).abs() < 1e-9, "a parábola passa Lv/8·Δg do PIV");
        assert!((g.rampa(v.s - v.lv) - v.g1).abs() < 1e-3);
    }

    #[test]
    fn perfil_curto_demais_e_recusado() {
        assert!(ajustar(&[(0.0, 1.0), (1.0, 2.0)], &Opcoes::default()).is_err());
    }
}
