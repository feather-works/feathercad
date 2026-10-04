//! Superelevação ferroviária por curva (he, h adotado, insuficiência I, aceleração não compensada, R mínimo,
//! comprimento mínimo da transição, velocidade máxima, flecha) e o perfil de `h` ao longo do eixo.
//! Os limites vêm de `Regras` (tabela de dados, não código): a norma muda, o binário não.

use serde::{Deserialize, Serialize};

use super::alinhamento::{Alinhamento, Curva};

const GRAVIDADE: f64 = 9.81;

/// Limites e parâmetros. Os valores de fábrica são os usuais de bitola métrica e **não** foram conferidos no texto
/// da ABNT NBR 16961:2021: conferir antes de tratar como norma.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Regras {
    pub nome: String,
    /// Distância entre eixos dos trilhos, m.
    #[serde(alias = "B")]
    pub b: f64,
    /// Velocidade diretriz, km/h.
    #[serde(alias = "V")]
    pub v: f64,
    /// Velocidade das composições lentas (para o excesso de superelevação), km/h.
    pub v_lenta: f64,
    pub h_max_mm: f64,
    pub i_max_mm: f64,
    /// Taxa máxima de variação da superelevação, mm/s.
    pub dh_dt_mm_s: f64,
    /// Rampa máxima de superelevação (1/600 = 0,001666…).
    pub rampa: f64,
    /// Taxa de variação da aceleração não compensada (ψ), m/s³.
    pub psi: f64,
    pub ls_min: f64,
    pub ls_multiplo: f64,
    /// Passo de arredondamento de `h` para cima, mm.
    pub h_passo_mm: f64,
    /// Corda da flecha, m.
    pub corda: f64,
    /// Comprimento mínimo da curva circular, m.
    pub lc_min: f64,
}

impl Default for Regras {
    fn default() -> Self {
        Self { nome: "ferrovia_metrica".into(), b: 1.075, v: 65.0, v_lenta: 30.0, h_max_mm: 100.0, i_max_mm: 50.0, dh_dt_mm_s: 30.0, rampa: 1.0 / 600.0, psi: 0.35, ls_min: 30.0, ls_multiplo: 10.0, h_passo_mm: 5.0, corda: 20.0, lc_min: 20.0 }
    }
}

/// O resultado de uma curva. Medidas lineares em mm onde o nome diz.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResultadoCurva {
    pub curva: usize,
    pub raio: f64,
    pub ls: f64,
    pub he_mm: f64,
    pub h_mm: f64,
    pub i_mm: f64,
    pub anc: f64,
    pub r_min: f64,
    pub v_max: f64,
    pub l_min: f64,
    pub ls_sugerido: f64,
    /// Rampa efetiva: h/Ls (adimensional); `1:n` com `n = Ls/h`.
    pub rampa_efetiva: f64,
    pub flecha_mm: f64,
    pub excesso_v_lenta_mm: f64,
    pub ok_raio: bool,
    pub ok_insuficiencia: bool,
    pub ok_transicao: bool,
    pub ok: bool,
}

fn arredondar_para_cima(v: f64, passo: f64) -> f64 {
    if passo <= 0.0 { v } else { (v / passo - 1e-9).ceil() * passo }
}

/// O `Ls` (múltiplo de `ls_multiplo`) que a transição mínima pede para um raio — para dimensionar antes de haver curva.
pub fn transicao_para_raio(raio: f64, r: &Regras) -> f64 {
    let he = r.b * r.v * r.v / (127.0 * raio) * 1000.0;
    let h = arredondar_para_cima(he, r.h_passo_mm).clamp(0.0, r.h_max_mm);
    let anc = GRAVIDADE * ((he - h).max(0.0) / 1000.0) / r.b;
    let ve = r.v / 3.6;
    let l_min = [h / 1000.0 * ve / (r.dh_dt_mm_s / 1000.0), h / 1000.0 / r.rampa, anc * ve / r.psi, r.ls_min].into_iter().fold(0.0, f64::max);
    arredondar_para_cima(l_min, r.ls_multiplo)
}

/// O menor raio que a superelevação máxima e a insuficiência máxima permitem.
pub fn raio_minimo(r: &Regras) -> f64 {
    r.b * r.v * r.v / (127.0 * (r.h_max_mm + r.i_max_mm) / 1000.0)
}

/// Calcula uma curva do alinhamento.
pub fn calcular_curva(c: &Curva, indice: usize, r: &Regras) -> ResultadoCurva {
    let ve = r.v / 3.6;
    let he = r.b * r.v * r.v / (127.0 * c.raio) * 1000.0;
    let h = arredondar_para_cima(he, r.h_passo_mm).clamp(0.0, r.h_max_mm);
    let i = (he - h).max(0.0);
    let anc = GRAVIDADE * (i / 1000.0) / r.b;
    let r_min = r.b * r.v * r.v / (127.0 * (r.h_max_mm + r.i_max_mm) / 1000.0);
    let v_max = (127.0 * c.raio * (r.h_max_mm + r.i_max_mm) / 1000.0 / r.b).sqrt();
    let l_min = [h / 1000.0 * ve / (r.dh_dt_mm_s / 1000.0), h / 1000.0 / r.rampa, anc * ve / r.psi, r.ls_min].into_iter().fold(0.0, f64::max);
    let ls_sugerido = arredondar_para_cima(l_min, r.ls_multiplo);
    let he_lenta = r.b * r.v_lenta * r.v_lenta / (127.0 * c.raio) * 1000.0;
    let (ok_raio, ok_insuficiencia, ok_transicao) = (c.raio + 1e-9 >= r_min, i <= r.i_max_mm + 1e-9, c.ls + 1e-9 >= l_min);
    ResultadoCurva {
        curva: indice, raio: c.raio, ls: c.ls, he_mm: he, h_mm: h, i_mm: i, anc, r_min, v_max, l_min, ls_sugerido,
        rampa_efetiva: if c.ls > 0.0 { h / 1000.0 / c.ls } else { 0.0 }, flecha_mm: r.corda * r.corda / (8.0 * c.raio) * 1000.0,
        excesso_v_lenta_mm: h - he_lenta, ok_raio, ok_insuficiencia, ok_transicao, ok: ok_raio && ok_insuficiencia && ok_transicao,
    }
}

/// Todas as curvas, na ordem do alinhamento.
pub fn calcular(a: &Alinhamento, r: &Regras) -> Vec<ResultadoCurva> {
    a.curvas.iter().enumerate().map(|(i, c)| calcular_curva(c, i, r)).collect()
}

/// A superelevação (mm) na progressiva `s`: 0 nas tangentes, rampa linear nas transições, `h` na circular.
/// Positiva = trilho externo mais alto (curva à esquerda ou à direita, o sinal é do `lado` da curva só no relatório).
pub fn h_em(a: &Alinhamento, resultados: &[ResultadoCurva], s: f64) -> f64 {
    for (c, r) in a.curvas.iter().zip(resultados) {
        if s < c.ts_marco.progressiva || s > c.st.progressiva {
            continue;
        }
        if s <= c.sc.progressiva {
            return r.h_mm * (s - c.ts_marco.progressiva) / (c.sc.progressiva - c.ts_marco.progressiva).max(1e-12);
        }
        if s <= c.cs.progressiva {
            return r.h_mm;
        }
        return r.h_mm * (c.st.progressiva - s) / (c.st.progressiva - c.cs.progressiva).max(1e-12);
    }
    0.0
}

/// O memorial de uma curva: cada conta com os números, em português.
pub fn memorial(c: &Curva, r: &ResultadoCurva, regras: &Regras) -> String {
    let ve = regras.v / 3.6;
    format!(
        "Curva {n} (R = {R:.0} m, Ls = {ls:.0} m, Δ = {d:.2}°).\n\
         Superelevação de equilíbrio: he = B·V²/(127·R) = {b}·{v}²/(127·{R:.0}) = {he:.1} mm.\n\
         Superelevação adotada (arredondada a {p} mm para cima, limitada a {hm} mm): h = {h:.0} mm.\n\
         Insuficiência: I = he − h = {i:.1} mm (limite {im} mm) — {oki}.\n\
         Aceleração não compensada: a = g·I/B = {g}·{ib:.4}/{b} = {anc:.3} m/s².\n\
         Raio mínimo: R_min = B·V²/(127·(h_max + I_max)) = {rmin:.1} m — {okr}.\n\
         Velocidade máxima na curva: V_max = √(127·R·(h_max + I_max)/B) = {vmax:.1} km/h.\n\
         Transição mínima: L_min = máx(h·v/(dh/dt) = {l1:.1} m; h/rampa = {l2:.1} m; a·v/ψ = {l3:.1} m; Ls_min = {lsm:.0} m) = {lmin:.1} m → Ls sugerido {lss:.0} m; adotado {ls:.0} m — {okt}.\n\
         Rampa efetiva de superelevação: 1:{rampa:.0}. Flecha na corda de {corda:.0} m: {fl:.1} mm. Excesso a {vl:.0} km/h: {ex:.1} mm.",
        n = r.curva + 1, R = c.raio, ls = c.ls, d = c.delta_graus, b = regras.b, v = regras.v, he = r.he_mm, p = regras.h_passo_mm, hm = regras.h_max_mm, h = r.h_mm,
        i = r.i_mm, im = regras.i_max_mm, oki = ok(r.ok_insuficiencia), g = GRAVIDADE, ib = r.i_mm / 1000.0, anc = r.anc, rmin = r.r_min, okr = ok(r.ok_raio),
        vmax = r.v_max, l1 = r.h_mm / 1000.0 * ve / (regras.dh_dt_mm_s / 1000.0), l2 = r.h_mm / 1000.0 / regras.rampa, l3 = r.anc * ve / regras.psi, lsm = regras.ls_min,
        lmin = r.l_min, lss = r.ls_sugerido, okt = ok(r.ok_transicao), rampa = if r.rampa_efetiva > 0.0 { 1.0 / r.rampa_efetiva } else { 0.0 }, corda = regras.corda,
        fl = r.flecha_mm, vl = regras.v_lenta, ex = r.excesso_v_lenta_mm,
    )
}

fn ok(b: bool) -> &'static str {
    if b { "atende" } else { "NÃO atende" }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::geom::Ponto;
    use crate::viario::alinhamento::EntradaCurva;

    pub fn c6() -> Alinhamento {
        let d = 93.92f64.to_radians();
        Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(2000.0, 0.0), Ponto::novo(2000.0 + 2000.0 * d.cos(), 2000.0 * d.sin())], &[Some(EntradaCurva { raio: 350.0, ls: 70.0 })], 0.0).unwrap()
    }

    #[test]
    fn c6_reproduz_a_tabela_de_referencia() {
        let a = c6();
        let r = &calcular(&a, &Regras::default())[0];
        assert!((r.he_mm - 102.2).abs() < 0.05, "he {}", r.he_mm);
        assert_eq!(r.h_mm, 100.0);
        assert!((r.i_mm - 2.2).abs() < 0.05, "I {}", r.i_mm);
        assert!((r.anc - 0.020).abs() < 0.0005, "anc {}", r.anc);
        assert!((r.r_min - 238.4).abs() < 0.05, "Rmin {}", r.r_min);
        assert!((r.l_min - 60.2).abs() < 0.05, "Lmin {}", r.l_min);
        assert_eq!(r.ls_sugerido, 70.0);
        assert!((r.v_max - 78.8).abs() < 0.05, "Vmax {}", r.v_max);
        assert!((r.flecha_mm - 142.9).abs() < 0.05, "flecha {}", r.flecha_mm);
        assert!(r.ok);
        let m = memorial(&a.curvas[0], r, &Regras::default());
        assert!(m.contains("he = ") && m.contains("102.2") && m.contains("atende"), "{m}");
    }

    #[test]
    fn perfil_de_h_sobe_na_transicao_e_zera_fora() {
        let a = c6();
        let r = calcular(&a, &Regras::default());
        let c = &a.curvas[0];
        assert_eq!(h_em(&a, &r, c.ts_marco.progressiva - 1.0), 0.0);
        assert!((h_em(&a, &r, (c.ts_marco.progressiva + c.sc.progressiva) / 2.0) - 50.0).abs() < 1e-9);
        assert_eq!(h_em(&a, &r, (c.sc.progressiva + c.cs.progressiva) / 2.0), 100.0);
        assert_eq!(h_em(&a, &r, c.st.progressiva + 1.0), 0.0);
    }
}
