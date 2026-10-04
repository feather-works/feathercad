//! As pranchas do modelo `ferrovia` que não são plantas: o memorial de cálculo e a prancha da seção tipo com a
//! superestrutura em 3D. As medidas de leiaute estão em mm de papel para uma A0 (1189 mm de largura) e escalam com o papel.

use feathercad_core::documento::{AlinhaH, AlinhaV};
use feathercad_core::folha::InfoFolha;
use feathercad_core::geom::Ponto;
use feathercad_core::trajeto::estaca_texto;
use feathercad_core::viario::alinhamento::{Alinhamento, Curva};
use feathercad_core::viario::superelevacao::{h_em, Regras, ResultadoCurva};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::fonte::Fonte;
use crate::json::Res;
use crate::modelos::{inserir_folha, na_folha, passo, Contexto};
use crate::pecas::{aplicar, Pincel};

/// Os números do projeto que o memorial resume.
pub(crate) struct Resumo {
    pub comprimento: f64,
    pub pontos: usize,
    pub pares: usize,
    pub suspeitos: usize,
    pub rip_rms: f64,
    pub rip_max: f64,
    pub lev_medio: f64,
    pub lev_max: f64,
    pub pivs: usize,
}

fn col(titulo: &str, largura: f64, decimais: usize) -> Value {
    json!({ "titulo": titulo, "largura": largura, "alinhamento": "centro", "decimais": decimais })
}

fn texto(t: &str) -> Value {
    json!({ "tipo": "texto", "texto": t })
}

fn titulo(t: &str) -> Value {
    json!({ "tipo": "titulo", "texto": t })
}

fn formula(t: &str) -> Value {
    json!({ "tipo": "formula", "texto": t })
}

/// As seções 1 a 6 do memorial, com os valores da tabela de regras em uso.
fn premissas(r: &Regras, resumo: &Resumo, n_curvas: usize) -> Vec<Value> {
    let rmin = r.b * r.v * r.v / (127.0 * (r.h_max_mm + r.i_max_mm) / 1000.0);
    vec![
        titulo("1. DADOS E PREMISSAS"),
        texto(&format!("Critérios da tabela de regras «{}» (editável em ~/.config/feathercad/regras/). Velocidade diretriz V = {:.0} km/h; distância entre eixos dos trilhos B = {} m (bitola métrica); composições lentas a {:.0} km/h.", r.nome, r.v, r.b, r.v_lenta)),
        texto(&format!("Levantamento: {} pontos, {} pares de trilho identificados ({} descartados por cota suspeita). O eixo existente é o ponto médio de cada par. O traçado de projeto, de {:.0} m, foi ajustado ao existente com {} curvas de transição em clotoide: ripagem RMS {:.2} m, máxima {:.2} m.", resumo.pontos, resumo.pares, resumo.suspeitos, resumo.comprimento, n_curvas, resumo.rip_rms, resumo.rip_max)),
        titulo("2. SUPERELEVAÇÃO"),
        formula("he = B·V² / (127·R)"),
        texto(&format!("he é a superelevação de equilíbrio (mm). A adotada h é he arredondada para cima a múltiplos de {:.0} mm e limitada a h_max = {:.0} mm.", r.h_passo_mm, r.h_max_mm)),
        formula("I = he − h  ≤  I_max"),
        texto(&format!("I é a insuficiência de superelevação, com limite I_max = {:.0} mm.", r.i_max_mm)),
        titulo("3. RAIO MÍNIMO"),
        formula("R_min = B·V² / (127·(h_max + I_max))"),
        texto(&format!("Com V = {:.0} km/h: R_min = {rmin:.1} m. Curvas de raio menor não atendem.", r.v)),
        titulo("4. COMPRIMENTO DE TRANSIÇÃO (CLOTOIDE)"),
        formula("L_min = máx( h·v / (dh/dt) ;  h / rampa ;  a·v / ψ ;  Ls_min )"),
        texto(&format!("dh/dt = {:.0} mm/s; rampa máxima 1:{:.0}; ψ = {} m/s³; Ls_min = {:.0} m. O Ls adotado é múltiplo de {:.0} m.", r.dh_dt_mm_s, 1.0 / r.rampa, r.psi, r.ls_min, r.ls_multiplo)),
        titulo("5. ACELERAÇÃO NÃO COMPENSADA"),
        formula("a = g·I / B"),
        titulo("6. PERFIL"),
        texto(&format!("Greide de projeto por {} PIVs com curvas verticais parabólicas. Levante sobre o existente: médio {:.3} m, máximo {:.3} m.", resumo.pivs, resumo.lev_medio, resumo.lev_max)),
        json!({ "tipo": "nota", "texto": "Valores calculados pelo FeatherCAD. Confira com a norma vigente antes de emitir." }),
    ]
}

/// O texto de cada curva (do cálculo) em itens: a primeira linha vira subtítulo.
fn itens_das_curvas(memorial: &str) -> Vec<Value> {
    let mut itens = Vec::new();
    for bloco in memorial.split("\n\n") {
        let (cabeca, resto) = bloco.split_once('\n').unwrap_or((bloco, ""));
        itens.push(titulo(cabeca.trim_end_matches('.')));
        if !resto.is_empty() {
            itens.push(texto(resto));
        }
    }
    itens
}

/// Memorial de cálculo: premissas, memória de cada curva e, à direita, a verificação de todas as curvas e o resumo.
pub(crate) fn prancha_memorial(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, n: usize, dados: (&Alinhamento, &[ResultadoCurva], &Regras, &Resumo), memorial: &str) -> Res<()> {
    let (a, tabela, regras, resumo) = dados;
    let f = inserir_folha(ag, fonte, ctx, &ctx.papel, Some(ctx.escala), "Memorial de cálculo da superelevação", n)?;
    let (l, h) = (f.largura_mm, f.altura_mm);
    let u = l / 1189.0;
    let topo = h - 100.0 * u;
    let altura_max = topo - 165.0 * u;
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(&f, 0.05 * l, h - 52.0 * u), "escala": ctx.escala, "largura": 0.9 * l, "altura_texto": 6.5 * u,
        "itens": [titulo(&format!("MEMORIAL DE CÁLCULO — SUPERELEVAÇÃO E TRANSIÇÕES — V = {:.0} km/h — BITOLA MÉTRICA", regras.v))] }))?;
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(&f, 0.05 * l, topo), "escala": ctx.escala, "largura": 0.215 * l, "altura_texto": 4.1 * u, "altura_max": altura_max, "itens": premissas(regras, resumo, a.curvas.len()) }))?;
    let mut por_curva = vec![titulo("7. MEMÓRIA DE CÁLCULO POR CURVA")];
    por_curva.extend(itens_das_curvas(memorial));
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(&f, 0.29 * l, topo), "escala": ctx.escala, "largura": 0.43 * l, "altura_texto": 3.7 * u, "colunas": 2, "altura_max": altura_max, "itens": por_curva }))?;
    verificacao(ag, fonte, ctx, &f, a, tabela)?;
    resumo_do_projeto(ag, fonte, ctx, &f, resumo)
}

fn verificacao(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, f: &InfoFolha, a: &Alinhamento, tabela: &[ResultadoCurva]) -> Res<()> {
    let (l, h) = (f.largura_mm, f.altura_mm);
    let u = l / 1189.0 * 1.3;
    let ok = |b: bool| if b { "sim" } else { "NÃO" };
    let linhas: Vec<Value> = a.curvas.iter().zip(tabela).enumerate().map(|(i, (c, r))| json!([i + 1, c.raio, c.ls, r.he_mm, r.h_mm, r.i_mm, r.l_min, r.v_max, ok(r.ok)])).collect();
    passo(ag, fonte, "cad_tabela", &json!({ "origem": na_folha(f, 0.745 * l, h - 130.0 * u / 1.3), "escala": ctx.escala, "titulo": "8. VERIFICAÇÃO DE TODAS AS CURVAS", "altura_linha": 6.0 * u, "altura_texto": 3.0 * u,
        "colunas": [col("Curva", 17.0 * u, 0), col("R (m)", 20.0 * u, 0), col("Ls (m)", 20.0 * u, 0), col("he (mm)", 23.0 * u, 1), col("h (mm)", 21.0 * u, 0), col("I (mm)", 21.0 * u, 1), col("L_min (m)", 25.0 * u, 1), col("V_max", 22.0 * u, 1), col("Atende", 22.0 * u, 0)],
        "linhas": linhas })).map(|_| ())
}

fn resumo_do_projeto(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, f: &InfoFolha, r: &Resumo) -> Res<()> {
    let (l, h) = (f.largura_mm, f.altura_mm);
    let u = l / 1189.0 * 1.3;
    let linhas = vec![
        json!(["Extensão do traçado", format!("{:.0} m", r.comprimento)]),
        json!(["Pontos do levantamento", r.pontos]),
        json!(["Pares de trilho achados", r.pares]),
        json!(["Descartados (cota suspeita)", r.suspeitos]),
        json!(["Ripagem RMS / máxima", format!("{:.2} m / {:.2} m", r.rip_rms, r.rip_max)]),
        json!(["Levante médio / máximo", format!("{:.3} m / {:.3} m", r.lev_medio, r.lev_max)]),
        json!(["PIVs do greide", r.pivs]),
    ];
    let c2 = |t: &str, w: f64| json!({ "titulo": t, "largura": w * u, "alinhamento": "esquerda", "decimais": 0 });
    passo(ag, fonte, "cad_tabela", &json!({ "origem": na_folha(f, 0.745 * l, h - 130.0 * u / 1.3 - 135.0 * u), "escala": ctx.escala, "titulo": "RESUMO DO PROJETO", "altura_linha": 6.0 * u, "altura_texto": 3.0 * u,
        "colunas": [c2("Item", 100.0), c2("Valor", 91.0)], "linhas": linhas })).map(|_| ())
}

// ---------------------------------------------------------------------------------------------------- seção e 3D

/// Onde a curva é mais crítica (maior superelevação; no empate, o menor raio).
fn mais_critica(a: &Alinhamento, tabela: &[ResultadoCurva]) -> Option<usize> {
    (0..a.curvas.len()).max_by(|x, y| tabela[*x].h_mm.total_cmp(&tabela[*y].h_mm).then(a.curvas[*y].raio.total_cmp(&a.curvas[*x].raio)))
}

/// Um ponto 3D relativo ao centro: (x, y, z).
type P3 = [f64; 3];

/// Trilhos, dormentes e lastro de um trecho reto de `comprimento` m centrado em `s_centro`, em 3D real (m, relativo ao centro),
/// com a superelevação `h_m` (o trilho de dentro fica ao nível do dormente mais baixo).
fn superestrutura(a: &Alinhamento, c: &Curva, s_centro: f64, h_m: f64, comprimento: f64) -> Vec<Vec<P3>> {
    let (b, dorm_c, dorm_l, dorm_h, esp) = (1.075, 2.0, 0.22, 0.20, 0.60);
    let (lastro, ombro, talude) = (0.30, 0.40, 1.5);
    let centro = a.em(s_centro).ponto;
    // u = distância transversal para a esquerda; o trilho de dentro da curva é o do lado da curvatura
    let u_dentro = f64::from(c.lado) * b / 2.0;
    let z_do = |u: f64| h_m * (u - u_dentro) / (-2.0 * u_dentro);
    let ponto = |s: f64, u: f64, z: f64| -> P3 {
        let am = a.em(s_centro + s);
        let (sn, cs) = am.direcao.to_radians().sin_cos();
        [am.ponto.x - sn * u - centro.x, am.ponto.y + cs * u - centro.y, z]
    };
    let mut l: Vec<Vec<P3>> = Vec::new();
    let passos = (comprimento / 0.3).round() as usize;
    for &ur in &[b / 2.0, -b / 2.0] {
        l.push((0..=passos).map(|i| ponto(-comprimento / 2.0 + i as f64 * 0.3, ur, z_do(ur) + 0.168)).collect());
        l.push((0..=passos).map(|i| ponto(-comprimento / 2.0 + i as f64 * 0.3, ur, z_do(ur))).collect());
    }
    let n_dorm = (comprimento / esp).floor() as usize;
    for k in 0..=n_dorm {
        let s = -comprimento / 2.0 + k as f64 * esp;
        let (sa, sb) = (s - dorm_l / 2.0, s + dorm_l / 2.0);
        let cantos = |z0: f64| [ponto(sa, dorm_c / 2.0, z0 + z_do(dorm_c / 2.0)), ponto(sb, dorm_c / 2.0, z0 + z_do(dorm_c / 2.0)), ponto(sb, -dorm_c / 2.0, z0 + z_do(-dorm_c / 2.0)), ponto(sa, -dorm_c / 2.0, z0 + z_do(-dorm_c / 2.0))];
        let (alto, baixo) = (cantos(0.0), cantos(-dorm_h));
        l.push(alto.iter().chain(std::iter::once(&alto[0])).copied().collect());
        l.push(baixo.iter().chain(std::iter::once(&baixo[0])).copied().collect());
        for i in 0..4 {
            l.push(vec![alto[i], baixo[i]]);
        }
    }
    // lastro: seção trapezoidal sob os dormentes (base horizontal, taludes 1:1,5) nas duas pontas e arestas ao longo
    let (ue, ud) = (dorm_c / 2.0 + ombro, -(dorm_c / 2.0 + ombro));
    let (z_e, z_d) = (z_do(ue) - dorm_h, z_do(ud) - dorm_h);
    let z_base = z_e.min(z_d) - lastro;
    let (t_e, t_d) = ((z_e - z_base) * talude, (z_d - z_base) * talude);
    let secao = |s: f64| -> Vec<P3> { vec![ponto(s, ue, z_e), ponto(s, ud, z_d), ponto(s, ud - t_d, z_base), ponto(s, ue + t_e, z_base), ponto(s, ue, z_e)] };
    let (ini, fim) = (-comprimento / 2.0 - 0.2, comprimento / 2.0 + 0.2);
    l.push(secao(ini));
    l.push(secao(fim));
    for k in 0..4 {
        l.push(vec![secao(ini)[k], secao(fim)[k]]);
    }
    l
}

fn projetar(ag: &mut Agente, fonte: &Fonte, linhas: &[Vec<P3>], camera: &Value, origem: Value, camada: &str) -> Res<()> {
    passo(ag, fonte, "cad_projetar", &json!({ "camera": camera, "linhas": linhas, "origem": origem, "camada": camada, "cor": "#FFFFFF" })).map(|_| ())
}

/// Trilhos com a superelevação exagerada, do TS ao ST com 60 m de cada lado.
fn esquema_3d(a: &Alinhamento, c: &Curva, tabela: &[ResultadoCurva], h_fator: f64) -> (Vec<Vec<P3>>, f64) {
    let (de, ate) = ((c.ts_marco.progressiva - 60.0).max(a.progressiva_inicial), (c.st.progressiva + 60.0).min(a.progressiva_final()));
    let b = 1.075;
    let centro = a.em((de + ate) / 2.0).ponto;
    let n = ((ate - de) / 4.0).ceil() as usize;
    let mut linhas = Vec::new();
    for lado in [-1.0, 1.0] {
        linhas.push(
            (0..=n)
                .map(|j| {
                    let s = (de + j as f64 * 4.0).min(ate);
                    let am = a.em(s);
                    let (sn, cs) = am.direcao.to_radians().sin_cos();
                    let z = h_em(a, tabela, s) / 1000.0 * if (lado > 0.0) == (c.lado < 0) { 1.0 } else { 0.0 } * h_fator;
                    [am.ponto.x - sn * lado * b / 2.0 - centro.x, am.ponto.y + cs * lado * b / 2.0 - centro.y, z]
                })
                .collect(),
        );
    }
    (linhas, ate - de)
}

/// Seção tipo, bloco 3D da superestrutura, esquema 3D da curva e planta da via no trecho de transição.
pub(crate) fn prancha_secao_e_3d(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, n: usize, a: &Alinhamento, tabela: &[ResultadoCurva]) -> Res<()> {
    let f = inserir_folha(ag, fonte, ctx, &ctx.papel, Some(ctx.escala), "Seção tipo, superestrutura e planta da via", n)?;
    let Some(i) = mais_critica(a, tabela) else { return Ok(()) };
    let (c, h_mm) = (&a.curvas[i], tabela[i].h_mm);
    let (l, hh) = (f.largura_mm, f.altura_mm);
    let u = l / 1189.0;
    let e = ctx.escala;
    let rotulo = |ag: &mut Agente, x: f64, y: f64, t: &str, sub: &str| {
        passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(&f, x * u, y * u), "escala": e, "largura": 480.0 * u, "altura_texto": 4.5 * u, "itens": [titulo(t), json!({ "tipo": "nota", "texto": sub })] })).map(|_| ())
    };
    // 1) seção tipo em 1:20
    rotulo(ag, 70.0, hh / u - 55.0, &format!("SEÇÃO TRANSVERSAL TIPO EM CURVA — CURVA {}", i + 1), "Esc. 1:20 — superelevação aplicada girando a via em torno do trilho interno")?;
    passo(ag, fonte, "cad_via", &json!({ "desenho": "secao", "ponto": na_folha(&f, 290.0 * u, hh - 145.0 * u), "h_mm": h_mm, "ampliacao": e / 20.0, "altura_texto": 3.0 }))?;
    // 2) dados da curva
    let r = &tabela[i];
    let dados = format!("Curva {}: R = {:.0} m, Ls = {:.0} m, Lc = {:.1} m, Δ = {:.2}°.\nhe = {:.1} mm; h adotado = {:.0} mm; I = {:.1} mm; a = {:.3} m/s².\nTS {}  —  SC {}  —  CS {}  —  ST {}.", i + 1, c.raio, c.ls, c.lc, c.delta_graus, r.he_mm, r.h_mm, r.i_mm, r.anc, estaca_texto(c.ts_marco.progressiva, 20.0), estaca_texto(c.sc.progressiva, 20.0), estaca_texto(c.cs.progressiva, 20.0), estaca_texto(c.st.progressiva, 20.0));
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(&f, 70.0 * u, hh - 300.0 * u), "escala": e, "largura": 460.0 * u, "altura_texto": 3.6 * u, "itens": [titulo("DADOS DA CURVA CRÍTICA"), texto(&dados)] }))?;
    // 3) bloco 3D da superestrutura (1:25, superelevação real)
    let s_meio = if c.lc > 8.0 { (c.sc.progressiva + c.cs.progressiva) / 2.0 } else { (c.ts_marco.progressiva + c.st.progressiva) / 2.0 };
    rotulo(ag, 600.0, hh / u - 55.0, &format!("BLOCO 3D DA SUPERESTRUTURA — TRECHO CIRCULAR DA CURVA {}", i + 1), &format!("Perspectiva, esc. 1:25 — superelevação real h = {h_mm:.0} mm — trilhos, dormentes e lastro"))?;
    projetar(ag, fonte, &superestrutura(a, c, s_meio, h_mm / 1000.0, 7.2), &json!({ "azimute": 25.0 - a.em(s_meio).direcao, "elevacao": 28.0, "escala": e / 25.0 }), na_folha(&f, 870.0 * u, hh - 215.0 * u), "BLOCO_3D")?;
    // 4) esquema 3D da curva (superelevação exagerada)
    let (linhas, vao) = esquema_3d(a, c, tabela, 50.0);
    rotulo(ag, 70.0, 440.0, &format!("ESQUEMA 3D DA CURVA {} — TS → ST", i + 1), "Perspectiva — superelevação exagerada 50 vezes na vertical")?;
    projetar(ag, fonte, &linhas, &json!({ "azimute": a.em(s_meio).direcao.rem_euclid(360.0) - 8.0, "elevacao": 48.0, "escala": 500.0 / vao.max(1.0) * e / 1000.0 }), na_folha(&f, 330.0 * u, 300.0 * u), "ESQUEMA_3D")?;
    // 5) planta da via no trecho TS → SC (1:200), girada para ficar na horizontal
    planta_do_trecho(ag, fonte, ctx, &f, a, (i, c))
}

/// A planta da via (trilhos e dormentes) no trecho do TS até 30 m depois do SC, em 1:200.
fn planta_do_trecho(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, f: &InfoFolha, a: &Alinhamento, (i, c): (usize, &Curva)) -> Res<()> {
    let u = f.largura_mm / 1189.0;
    let e = ctx.escala;
    let (s0, s1) = (c.ts_marco.progressiva, (c.sc.progressiva + 30.0).min(a.progressiva_final()));
    let (ponto0, giro) = (a.em(s0).ponto, -a.em((s0 + s1) / 2.0).direcao);
    let para = na_folha(f, 600.0 * u, 300.0 * u);
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": na_folha(f, 600.0 * u, 440.0 * u), "escala": e, "largura": 520.0 * u, "altura_texto": 4.5 * u,
        "itens": [titulo(&format!("PLANTA DA VIA NO TRECHO TS → SC DA CURVA {}", i + 1)), json!({ "tipo": "nota", "texto": "Esc. 1:200 — trilhos e dormentes (a cada 0,60 m)" })] }))?;
    passo(ag, fonte, "cad_via", &json!({ "desenho": "planta", "de": s0 - a.progressiva_inicial, "ate": s1 - a.progressiva_inicial, "destino": { "de": [ponto0.x, ponto0.y], "para": para, "ampliacao": e / 200.0, "giro": giro } }))?;
    // marcas sob a planta: TS, SC e o fim do trecho, na mesma transformação da via
    let (sn, cs) = giro.to_radians().sin_cos();
    let (k, origem) = (e / 200.0, Ponto::novo(para[0].as_f64().unwrap_or(0.0), para[1].as_f64().unwrap_or(0.0)));
    let mut p = Pincel::novo("PLANTA_VIA_MARCAS");
    for (nome, s) in [("TS", s0), ("SC", c.sc.progressiva), ("SC + 30 m", s1)] {
        let q = a.em(s).ponto;
        let (dx, dy) = (q.x - ponto0.x, q.y - ponto0.y);
        let m = Ponto::novo(origem.x + (dx * cs - dy * sn) * k, origem.y + (dx * sn + dy * cs) * k);
        let ht = 3.2 * u * e / 1000.0;
        p.linha(Ponto::novo(m.x, m.y - 8.0 * u * e / 1000.0), Ponto::novo(m.x, m.y - 14.0 * u * e / 1000.0));
        p.texto(Ponto::novo(m.x, m.y - 15.0 * u * e / 1000.0), ht, 0.0, &format!("{nome} — est. {}", estaca_texto(s, 20.0)), AlinhaH::Centro, AlinhaV::Topo);
    }
    aplicar(&mut ag.doc, p.pecas).map(|_| ())
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::viario::alinhamento::EntradaCurva;

    #[test]
    fn superestrutura_tem_trilhos_dormentes_e_lastro() {
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(700.0, 0.0), Ponto::novo(700.0 + 700.0 * d.cos(), 700.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        let c = &a.curvas[0];
        let l = superestrutura(&a, c, (c.sc.progressiva + c.cs.progressiva) / 2.0, 0.1, 7.2);
        // 4 linhas de trilho + 13 dormentes x 6 + 2 seções + 4 arestas
        assert_eq!(l.len(), 4 + 13 * 6 + 2 + 4);
        let z_max = l.iter().flatten().map(|p| p[2]).fold(f64::MIN, f64::max);
        assert!((z_max - (0.1 + 0.168)).abs() < 1e-9, "o trilho de fora fica h + altura do trilho acima do de dentro: {z_max}");
    }

    #[test]
    fn itens_das_curvas_separa_o_subtitulo() {
        let v = itens_das_curvas("Curva 1 (R = 1100 m).\nlinha a\nlinha b\n\nCurva 2 (R = 380 m).\nlinha c");
        assert_eq!(v.len(), 4);
        assert_eq!(v[0]["texto"], "Curva 1 (R = 1100 m)");
    }
}
