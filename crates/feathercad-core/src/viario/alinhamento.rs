//! Alinhamento horizontal por PIs, com curvas de transição (clotoide simétrica) e circulares.
//!
//! Para cada PI interno vêm `R` e `Ls`. Fórmulas (θs em rad): θs = Ls/(2R); xs, ys pelas séries da clotoide;
//! p = ys − R(1 − cos θs); k = xs − R sen θs; Ts = (R + p)·tg(Δ/2) + k; E = (R + p)/cos(Δ/2) − R;
//! Lc = R(Δ − 2θs); A = √(R·Ls). Com `Ls = 0` a curva é circular simples.
//! Ângulos em graus, anti-horário a partir do eixo X ("direção"); `azimute` = a direção de topografia (do norte, horário).

use crate::geom::Ponto;

/// O que o usuário pede para um PI interno.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntradaCurva {
    pub raio: f64,
    pub ls: f64,
}

/// Um ponto notável (TS, SC, CS, ST): progressiva em metros e coordenadas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marco {
    pub progressiva: f64,
    pub ponto: Ponto,
}

/// Os números de uma curva.
#[derive(Clone, Debug, PartialEq)]
pub struct Curva {
    /// Índice do PI (0 = primeiro ponto da lista; só os internos têm curva).
    pub pi: usize,
    pub raio: f64,
    pub ls: f64,
    pub delta_graus: f64,
    /// +1 = para a esquerda, −1 = para a direita.
    pub lado: i8,
    pub lc: f64,
    pub ts: f64,
    pub e: f64,
    pub theta_s_graus: f64,
    pub a: f64,
    pub p: f64,
    pub k: f64,
    pub xs: f64,
    pub ys: f64,
    pub ts_marco: Marco,
    pub sc: Marco,
    pub cs: Marco,
    pub st: Marco,
    pub pi_progressiva: f64,
    /// Coordenadas do PI.
    pub pi_ponto: Ponto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Reta,
    Espiral,
    Circular,
}

impl Tipo {
    pub fn letra(self) -> &'static str {
        match self {
            Tipo::Reta => "T",
            Tipo::Espiral => "E",
            Tipo::Circular => "C",
        }
    }
}

/// Um trecho contínuo com curvatura linear de `k0` a `k1` (1/m, positiva = esquerda).
#[derive(Clone, Debug)]
struct Elemento {
    tipo: Tipo,
    curva: Option<usize>,
    s0: f64,
    comprimento: f64,
    origem: Ponto,
    direcao0: f64,
    k0: f64,
    k1: f64,
}

/// Onde o alinhamento está numa progressiva.
#[derive(Clone, Copy, Debug)]
pub struct Amostra {
    pub ponto: Ponto,
    /// Direção da tangente, graus anti-horários do eixo X.
    pub direcao: f64,
    /// Direção de topografia: graus a partir do norte (eixo Y), no sentido horário.
    pub azimute: f64,
    /// Curvatura (1/m), positiva = esquerda.
    pub curvatura: f64,
    pub tipo: Tipo,
    pub curva: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Alinhamento {
    pub pis: Vec<Ponto>,
    /// O que foi pedido para cada PI interno (para refazer com outro `Ls`).
    pub entradas: Vec<Option<EntradaCurva>>,
    pub curvas: Vec<Curva>,
    pub progressiva_inicial: f64,
    pub comprimento: f64,
    elementos: Vec<Elemento>,
}

fn direcao(a: Ponto, b: Ponto) -> f64 {
    (b.y - a.y).atan2(b.x - a.x).to_degrees()
}

fn normalizar(g: f64) -> f64 {
    let m = g.rem_euclid(360.0);
    if m > 180.0 { m - 360.0 } else { m }
}

/// Série da clotoide para θs em rad: (xs, ys) / Ls.
fn serie(theta: f64) -> (f64, f64) {
    let t2 = theta * theta;
    (1.0 - t2 / 10.0 + t2 * t2 / 216.0 - t2 * t2 * t2 / 9360.0, theta / 3.0 - theta * t2 / 42.0 + theta * t2 * t2 / 1320.0 - theta * t2 * t2 * t2 / 75600.0)
}

/// Avança `l` metros num trecho de curvatura linear (Simpson em passos de até 0,5 m).
/// Devolve (ponto, direção em graus).
fn avancar(origem: Ponto, direcao0: f64, k0: f64, k1: f64, comprimento: f64, l: f64) -> (Ponto, f64) {
    let h0 = direcao0.to_radians();
    let rumo = |s: f64| h0 + k0 * s + (k1 - k0) * s * s / (2.0 * comprimento.max(1e-12));
    let n = (((l / 0.5).ceil() as usize).max(1) + 1) & !1;
    let passo = l / n as f64;
    let (mut sx, mut sy) = (0.0, 0.0);
    for i in 0..=n {
        let peso = if i == 0 || i == n { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 };
        let r = rumo(i as f64 * passo);
        sx += peso * r.cos();
        sy += peso * r.sin();
    }
    (Ponto::novo(origem.x + sx * passo / 3.0, origem.y + sy * passo / 3.0), rumo(l).to_degrees())
}

/// Por que um alinhamento não fecha (para quem otimiza saber qual curva mexer). Cada variante traz a mensagem para o usuário.
#[derive(Clone, Debug, PartialEq)]
pub enum Falha {
    /// Δ pequeno demais para o R e o Ls (Lc negativo) no PI.
    CurvaCurta(usize, String),
    /// A tangente entre a curva anterior e a do PI ficou negativa.
    Tangente(usize, String),
    Invalida(String),
}

impl std::fmt::Display for Falha {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Falha::CurvaCurta(_, m) | Falha::Tangente(_, m) | Falha::Invalida(m) => f.write_str(m),
        }
    }
}

impl Alinhamento {
    /// `pis`: do ponto inicial ao final; `curvas`: uma por PI interno (`None` = vértice sem curva).
    pub fn novo(pis: &[Ponto], curvas: &[Option<EntradaCurva>], progressiva_inicial: f64) -> Result<Self, String> {
        Self::calcular(pis, curvas, progressiva_inicial).map_err(|f| f.to_string())
    }

    /// Como `novo`, com a falha estruturada.
    pub fn calcular(pis: &[Ponto], curvas: &[Option<EntradaCurva>], progressiva_inicial: f64) -> Result<Self, Falha> {
        if pis.len() < 2 {
            return Err(Falha::Invalida("o alinhamento precisa de ao menos 2 pontos".into()));
        }
        if curvas.len() != pis.len().saturating_sub(2) {
            return Err(Falha::Invalida(format!("{} PIs internos pedem {} curvas (uma por PI, `null` onde não há curva); vieram {}", pis.len().saturating_sub(2), pis.len().saturating_sub(2), curvas.len())));
        }
        let mut lidas: Vec<Curva> = Vec::new();
        for (j, pedido) in curvas.iter().enumerate() {
            let i = j + 1;
            let Some(c) = pedido else { continue };
            if c.raio <= 0.0 || c.ls < 0.0 {
                return Err(Falha::Invalida(format!("PI {i}: `R` precisa ser maior que zero e `Ls` não negativo")));
            }
            let (entra, sai) = (direcao(pis[i - 1], pis[i]), direcao(pis[i], pis[i + 1]));
            let delta_com_sinal = normalizar(sai - entra);
            if delta_com_sinal.abs() < 1e-7 {
                continue;
            }
            let (delta, lado) = (delta_com_sinal.abs().to_radians(), if delta_com_sinal > 0.0 { 1 } else { -1 });
            let theta = c.ls / (2.0 * c.raio);
            let (fx, fy) = serie(theta);
            let (xs, ys) = (c.ls * fx, c.ls * fy);
            let p = ys - c.raio * (1.0 - theta.cos());
            let k = xs - c.raio * theta.sin();
            let lc = c.raio * (delta - 2.0 * theta);
            if lc < -1e-9 {
                return Err(Falha::CurvaCurta(i, format!("PI {i}: Δ = {:.2}° é pequeno demais para R = {} e Ls = {} (Lc ficaria {:.2} m); reduza Ls ou R", delta.to_degrees(), c.raio, c.ls, lc)));
            }
            let ts = (c.raio + p) * (delta / 2.0).tan() + k;
            lidas.push(Curva {
                pi: i, raio: c.raio, ls: c.ls, delta_graus: delta.to_degrees(), lado, lc: lc.max(0.0), ts, e: (c.raio + p) / (delta / 2.0).cos() - c.raio,
                theta_s_graus: theta.to_degrees(), a: (c.raio * c.ls).sqrt(), p, k, xs, ys,
                ts_marco: Marco { progressiva: 0.0, ponto: pis[i] }, sc: Marco { progressiva: 0.0, ponto: pis[i] }, cs: Marco { progressiva: 0.0, ponto: pis[i] }, st: Marco { progressiva: 0.0, ponto: pis[i] }, pi_progressiva: 0.0, pi_ponto: pis[i],
            });
        }
        // Distâncias entre PIs e as progressivas (a tangente entre curvas não pode ser negativa).
        let dist: Vec<f64> = pis.windows(2).map(|j| j[0].distancia(j[1])).collect();
        let mut elementos: Vec<Elemento> = Vec::new();
        let mut inicio_reta = pis[0];
        let mut s_local = 0.0;
        let mut anterior_st_ts = 0.0; // Ts da curva anterior (distância já consumida no segmento corrente)
        let mut ultimo_pi = 0usize;
        for (idx, c) in lidas.iter_mut().enumerate() {
            let segmento: f64 = dist[ultimo_pi..c.pi].iter().sum();
            let reta = segmento - anterior_st_ts - c.ts;
            if reta < -1e-6 {
                return Err(Falha::Tangente(c.pi, format!("PI {}: a tangente entre as curvas ficou negativa ({reta:.2} m); as curvas se sobrepõem — reduza R, Ls ou afaste os PIs", c.pi)));
            }
            let reta = reta.max(0.0);
            let entra = direcao(pis[c.pi - 1], pis[c.pi]);
            let sai = direcao(pis[c.pi], pis[c.pi + 1]);
            let u_entra = (entra.to_radians().cos(), entra.to_radians().sin());
            let ts_ponto = Ponto::novo(pis[c.pi].x - u_entra.0 * c.ts, pis[c.pi].y - u_entra.1 * c.ts);
            if reta > 0.0 {
                elementos.push(Elemento { tipo: Tipo::Reta, curva: None, s0: s_local, comprimento: reta, origem: inicio_reta, direcao0: entra, k0: 0.0, k1: 0.0 });
            }
            s_local += reta;
            let lado = f64::from(c.lado);
            let k_circ = lado / c.raio;
            let lc_el = |tipo: Tipo, comp: f64, origem: Ponto, dir0: f64, k0: f64, k1: f64, s: &mut f64, el: &mut Vec<Elemento>| -> (Ponto, f64) {
                if comp > 0.0 {
                    el.push(Elemento { tipo, curva: Some(idx), s0: *s, comprimento: comp, origem, direcao0: dir0, k0, k1 });
                    *s += comp;
                    avancar(origem, dir0, k0, k1, comp, comp)
                } else {
                    (origem, dir0)
                }
            };
            let marco = |s: f64, p: Ponto| Marco { progressiva: progressiva_inicial + s, ponto: p };
            let ts_m = marco(s_local, ts_ponto);
            let (sc_p, dir_sc) = lc_el(Tipo::Espiral, c.ls, ts_ponto, entra, 0.0, k_circ, &mut s_local, &mut elementos);
            let sc_m = marco(s_local, sc_p);
            let (cs_p, dir_cs) = lc_el(Tipo::Circular, c.lc, sc_p, dir_sc, k_circ, k_circ, &mut s_local, &mut elementos);
            let cs_m = marco(s_local, cs_p);
            let (_st_integrado, _) = lc_el(Tipo::Espiral, c.ls, cs_p, dir_cs, k_circ, 0.0, &mut s_local, &mut elementos);
            let u_sai = (sai.to_radians().cos(), sai.to_radians().sin());
            let st_p = Ponto::novo(pis[c.pi].x + u_sai.0 * c.ts, pis[c.pi].y + u_sai.1 * c.ts);
            let st_m = marco(s_local, st_p);
            let pi_prog = ts_m.progressiva + c.ts;
            c.ts_marco = ts_m;
            c.sc = sc_m;
            c.cs = cs_m;
            c.st = st_m;
            c.pi_progressiva = pi_prog;
            inicio_reta = st_p;
            anterior_st_ts = c.ts;
            ultimo_pi = c.pi;
        }
        let ultimo: f64 = dist[ultimo_pi..].iter().sum::<f64>() - anterior_st_ts;
        if ultimo < -1e-6 {
            return Err(Falha::Invalida("a última curva passa do ponto final".into()));
        }
        let ultimo = ultimo.max(0.0);
        let dir_final = direcao(pis[pis.len() - 2], pis[pis.len() - 1]);
        if ultimo > 0.0 {
            elementos.push(Elemento { tipo: Tipo::Reta, curva: None, s0: s_local, comprimento: ultimo, origem: inicio_reta, direcao0: dir_final, k0: 0.0, k1: 0.0 });
        }
        Ok(Self { pis: pis.to_vec(), entradas: curvas.to_vec(), curvas: lidas, progressiva_inicial, comprimento: s_local + ultimo, elementos })
    }

    /// O ponto, a direção e a curvatura na progressiva `s` (metros, preso às pontas).
    pub fn em(&self, progressiva: f64) -> Amostra {
        let s = (progressiva - self.progressiva_inicial).clamp(0.0, self.comprimento);
        let el = self.elementos.iter().rev().find(|e| e.s0 <= s + 1e-12).unwrap_or(&self.elementos[0]);
        let l = (s - el.s0).clamp(0.0, el.comprimento);
        let (ponto, dir) = if el.tipo == Tipo::Reta {
            let r = el.direcao0.to_radians();
            (Ponto::novo(el.origem.x + r.cos() * l, el.origem.y + r.sin() * l), el.direcao0)
        } else {
            avancar(el.origem, el.direcao0, el.k0, el.k1, el.comprimento, l)
        };
        let curvatura = el.k0 + (el.k1 - el.k0) * l / el.comprimento.max(1e-12);
        Amostra { ponto, direcao: dir, azimute: (90.0 - dir).rem_euclid(360.0), curvatura, tipo: el.tipo, curva: el.curva }
    }

    /// Pontos do eixo (passo de até `passo` m nas curvas; retas só nas pontas), prontos para uma polilinha.
    /// Integra elemento por elemento (custo linear no comprimento), sem reiniciar a conta a cada ponto.
    pub fn amostrar(&self, passo: f64) -> Vec<Ponto> {
        let passo = passo.max(0.05);
        let mut v: Vec<Ponto> = Vec::new();
        for e in &self.elementos {
            if v.is_empty() {
                v.push(e.origem);
            }
            if e.tipo == Tipo::Reta {
                let r = e.direcao0.to_radians();
                v.push(Ponto::novo(e.origem.x + r.cos() * e.comprimento, e.origem.y + r.sin() * e.comprimento));
                continue;
            }
            let n = (e.comprimento / passo).ceil().max(2.0) as usize;
            let h = e.comprimento / n as f64;
            let (mut p, mut dir) = (e.origem, e.direcao0);
            let dk = (e.k1 - e.k0) / n as f64;
            for i in 0..n {
                let (k0, k1) = (e.k0 + dk * i as f64, e.k0 + dk * (i + 1) as f64);
                let (q, d) = avancar(p, dir, k0, k1, h, h);
                p = q;
                dir = d;
                v.push(p);
            }
        }
        v
    }

    pub fn progressiva_final(&self) -> f64 {
        self.progressiva_inicial + self.comprimento
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    /// Três PIs com Δ = 93,92° à esquerda, como a curva C6 do projeto de referência (R 350, Ls 70).
    fn c6() -> Alinhamento {
        let d = 93.92f64.to_radians();
        Alinhamento::novo(&[p(0.0, 0.0), p(2000.0, 0.0), p(2000.0 + 2000.0 * d.cos(), 2000.0 * d.sin())], &[Some(EntradaCurva { raio: 350.0, ls: 70.0 })], 0.0).unwrap()
    }

    #[test]
    fn curva_c6_reproduz_os_numeros_de_referencia() {
        let a = c6();
        let c = &a.curvas[0];
        assert_eq!(c.lado, 1);
        assert!((c.theta_s_graus - 5.730).abs() < 0.001, "θs {}", c.theta_s_graus);
        assert!((c.lc - 503.75).abs() < 0.05, "Lc {}", c.lc);
        assert!((c.ts - 410.44).abs() < 0.1, "Ts {}", c.ts);
        assert!((c.e - 163.68).abs() < 0.1, "E {}", c.e);
        assert!((c.k - 34.988).abs() < 0.005, "k {}", c.k);
        assert!((c.p - 0.583).abs() < 0.005, "p {}", c.p);
        assert!((c.a - 156.52).abs() < 0.01, "A {}", c.a);
    }

    #[test]
    fn a_integracao_fecha_no_st_e_o_comprimento_bate() {
        let a = c6();
        let c = &a.curvas[0];
        let fim = a.em(c.st.progressiva).ponto;
        assert!(fim.distancia(c.st.ponto) < 1e-3, "ST integrado a {} m do analítico", fim.distancia(c.st.ponto));
        // TS + 2 Ls + Lc = ST; progressivas em sequência
        assert!((c.st.progressiva - c.ts_marco.progressiva - (2.0 * c.ls + c.lc)).abs() < 1e-6);
        let esperado = 2000.0 - c.ts + (2.0 * c.ls + c.lc) + 2000.0 - c.ts;
        assert!((a.comprimento - esperado).abs() < 1e-6, "{} x {}", a.comprimento, esperado);
        let amostra = a.em(c.cs.progressiva - 10.0);
        assert_eq!(amostra.tipo, Tipo::Circular);
        assert!((amostra.curvatura - 1.0 / 350.0).abs() < 1e-9);
    }

    #[test]
    fn erros_dizem_o_motivo() {
        let e = Alinhamento::novo(&[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)], &[Some(EntradaCurva { raio: 350.0, ls: 70.0 })], 0.0).unwrap_err();
        assert!(e.contains("tangente"), "{e}");
        assert!(Alinhamento::novo(&[p(0.0, 0.0), p(1.0, 0.0)], &[Some(EntradaCurva { raio: 1.0, ls: 0.0 })], 0.0).is_err());
        let sem_curva = Alinhamento::novo(&[p(0.0, 0.0), p(100.0, 0.0), p(200.0, 0.0)], &[Some(EntradaCurva { raio: 100.0, ls: 10.0 })], 0.0).unwrap();
        assert!(sem_curva.curvas.is_empty() && (sem_curva.comprimento - 200.0).abs() < 1e-9);
    }
}
