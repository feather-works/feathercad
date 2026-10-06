//! `cad_circuitos`: a **lista de circuitos como fonte única** de um quadro de distribuição. Calcula (corrente de projeto,
//! seção pelo critério da capacidade de condução e da queda de tensão, disjuntor, DR, equilíbrio das fases, alimentador)
//! e, a pedido, desenha o **quadro de cargas**, o **quadro de demanda** e o **unifilar** a partir dos mesmos números.
//!
//! Tabelas: NBR 5410 (cobre, isolação PVC 70 °C, eletroduto, método B1; fatores de agrupamento e de temperatura).
//! Limites e fatores de demanda são **premissas** devolvidas no resultado — o projetista confere na norma/concessionária.

use std::collections::BTreeMap;

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, caixa_das, fator_papel, formatar, relatorio, Pincel};

/// Seções comerciais (mm²) e capacidade de condução (A) com 2 e com 3 condutores carregados (PVC 70 °C, método B1).
const SECOES: [f64; 12] = [1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0, 50.0, 70.0, 95.0, 120.0];
const IZ_2: [f64; 12] = [17.5, 24.0, 32.0, 41.0, 57.0, 76.0, 101.0, 125.0, 151.0, 192.0, 232.0, 269.0];
const IZ_3: [f64; 12] = [15.5, 21.0, 28.0, 36.0, 50.0, 68.0, 89.0, 110.0, 134.0, 171.0, 207.0, 239.0];
/// Disjuntores termomagnéticos padronizados (A).
const DISJUNTORES: [f64; 19] = [6.0, 10.0, 16.0, 20.0, 25.0, 32.0, 40.0, 50.0, 63.0, 70.0, 80.0, 90.0, 100.0, 125.0, 150.0, 160.0, 200.0, 225.0, 250.0];
const DR_PADRAO: [f64; 5] = [25.0, 40.0, 63.0, 80.0, 100.0];
/// Resistividade do cobre a 70 °C (Ω·mm²/m): conservadora para a queda de tensão.
const RHO: f64 = 0.0225;
const LETRAS: [char; 3] = ['R', 'S', 'T'];

/// Fator de agrupamento (NBR 5410, tabela 42): circuitos no mesmo eletroduto.
fn fator_agrupamento(n: usize) -> f64 {
    match n {
        0 | 1 => 1.0,
        2 => 0.80,
        3 => 0.70,
        4 => 0.65,
        5 => 0.60,
        6 => 0.57,
        7 => 0.54,
        8 => 0.52,
        9..=11 => 0.50,
        12..=15 => 0.45,
        16..=19 => 0.41,
        _ => 0.38,
    }
}

/// Fator de temperatura (PVC 70 °C) para a temperatura ambiente em °C.
fn fator_temperatura(t: f64) -> f64 {
    const T: [(f64, f64); 9] = [(20.0, 1.12), (25.0, 1.06), (30.0, 1.0), (35.0, 0.94), (40.0, 0.87), (45.0, 0.79), (50.0, 0.71), (55.0, 0.61), (60.0, 0.50)];
    T.iter().find(|(c, _)| t <= *c).map_or(0.5, |(_, f)| *f)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tipo {
    Iluminacao,
    Tug,
    Tue,
    Outro,
}

impl Tipo {
    fn de(nome: &str) -> Self {
        match nome.to_lowercase().as_str() {
            "iluminacao" | "iluminação" | "luz" | "emergencia" | "emergência" => Self::Iluminacao,
            "tug" | "tomada" | "tomadas" => Self::Tug,
            "tue" | "especifico" | "específico" => Self::Tue,
            _ => Self::Outro,
        }
    }

    fn nome(self) -> &'static str {
        match self {
            Self::Iluminacao => "iluminação",
            Self::Tug => "TUG",
            Self::Tue => "TUE",
            Self::Outro => "outro",
        }
    }

    fn secao_minima(self) -> f64 {
        if self == Self::Iluminacao { 1.5 } else { 2.5 }
    }
}

/// As premissas do quadro (tensões, temperatura, limites).
struct Quadro {
    nome: String,
    vfn: f64,
    vff: f64,
    fases: usize,
    temperatura: f64,
    limite_dv: f64,
    fator_demanda: f64,
    dr_em_tomadas: bool,
    alimentador_m: f64,
    agrupamento: Option<u64>,
    disjuntor_minimo: f64,
}

struct Entrada {
    id: String,
    descricao: String,
    tipo: Tipo,
    potencia: f64,
    bifasico: bool,
    fp: f64,
    comprimento: f64,
    fase: Option<Vec<usize>>,
    dr: bool,
    agrupamento: usize,
    disjuntor_minimo: f64,
}

/// O resultado de um circuito.
struct Linha {
    e: Entrada,
    fases: Vec<usize>,
    ib: f64,
    disjuntor: f64,
    dr: Option<f64>,
    secao: f64,
    iz: f64,
    dv: f64,
    ok: bool,
}

struct Calculo {
    quadro: Quadro,
    linhas: Vec<Linha>,
    w_fase: [f64; 3],
    i_fase: [f64; 3],
    geral: Geral,
    avisos: Vec<String>,
}

struct Geral {
    ib: f64,
    disjuntor: f64,
    secao: f64,
    iz: f64,
    dv: f64,
}

fn proximo_disjuntor(ib: f64) -> Option<f64> {
    DISJUNTORES.iter().copied().find(|d| *d >= ib - 1e-9)
}

/// Letras de fase de um texto (`R`, `S`, `RS`, `TR`…).
fn fases_do_texto(t: &str) -> Res<Vec<usize>> {
    let mut v: Vec<usize> = Vec::new();
    for c in t.to_uppercase().chars().filter(|c| !c.is_whitespace() && *c != '+' && *c != '/') {
        let i = LETRAS.iter().position(|l| *l == c).ok_or_else(|| format!("fase `{t}` inválida (R, S, T ou pares como RS)"))?;
        if !v.contains(&i) {
            v.push(i);
        }
    }
    v.sort_unstable();
    Ok(v)
}

fn quadro_de(args: &Value) -> Res<Quadro> {
    let q = args.get("quadro").cloned().unwrap_or(Value::Null);
    let n = |c: &str, p: f64| q.get(c).and_then(Value::as_f64).filter(|x| *x > 0.0).unwrap_or(p);
    let fases = n("fases", 3.0) as usize;
    if !(1..=3).contains(&fases) {
        return Err("`quadro.fases` vale 1, 2 ou 3".into());
    }
    Ok(Quadro {
        nome: q.get("nome").and_then(Value::as_str).unwrap_or("QD1").to_string(),
        vfn: n("tensao_fn", 127.0),
        vff: n("tensao_ff", 220.0),
        fases,
        temperatura: n("temperatura", 30.0),
        limite_dv: n("limite_dv", 4.0),
        fator_demanda: n("fator_demanda", 1.0),
        dr_em_tomadas: q.get("dr_em_tomadas").and_then(Value::as_bool).unwrap_or(true),
        alimentador_m: n("alimentador_m", 10.0),
        agrupamento: q.get("agrupamento").and_then(Value::as_u64),
        disjuntor_minimo: q.get("disjuntor_minimo_a").and_then(Value::as_f64).unwrap_or(0.0),
    })
}

fn entrada_de(i: usize, c: &Value, q: &Quadro) -> Res<Entrada> {
    let ctx = |m: &str| format!("circuito {}: {m}", i + 1);
    let potencia = c.get("potencia_w").and_then(Value::as_f64).filter(|p| *p > 0.0).ok_or_else(|| ctx("falta `potencia_w` (maior que zero)"))?;
    let tipo = Tipo::de(c.get("tipo").and_then(Value::as_str).unwrap_or("outro"));
    let bifasico = match c.get("tensao") {
        Some(Value::String(s)) => matches!(s.to_lowercase().as_str(), "ff" | "bifasico" | "bifásico"),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|v| (v - q.vff).abs() < 1.0),
        _ => false,
    };
    let fase = c.get("fase").and_then(Value::as_str).map(fases_do_texto).transpose().map_err(|e| ctx(&e))?;
    if fase.as_ref().is_some_and(|f| f.len() != if bifasico { 2 } else { 1 }) {
        return Err(ctx("o nº de letras de `fase` não bate com a tensão (uma para fase-neutro, duas para entre fases)"));
    }
    if bifasico && q.fases < 2 {
        return Err(ctx("circuito entre fases num quadro de uma fase só"));
    }
    Ok(Entrada {
        id: c.get("id").and_then(Value::as_str).map_or_else(|| format!("C{}", i + 1), str::to_string),
        descricao: c.get("descricao").and_then(Value::as_str).unwrap_or("").to_string(),
        tipo,
        potencia,
        bifasico,
        fp: c.get("fp").and_then(Value::as_f64).filter(|f| *f > 0.0 && *f <= 1.0).unwrap_or(if tipo == Tipo::Iluminacao { 0.92 } else { 1.0 }),
        comprimento: c.get("comprimento_m").and_then(Value::as_f64).filter(|l| *l >= 0.0).unwrap_or(20.0),
        fase,
        dr: c.get("dr").and_then(Value::as_bool).unwrap_or(q.dr_em_tomadas && matches!(tipo, Tipo::Tug | Tipo::Tue)),
        agrupamento: c.get("agrupamento").and_then(Value::as_u64).or(q.agrupamento).unwrap_or(1) as usize,
        disjuntor_minimo: c.get("disjuntor_minimo_a").and_then(Value::as_f64).unwrap_or(q.disjuntor_minimo),
    })
}

/// Escolhe a(s) fase(s) dos circuitos sem fase pedida: a(s) menos carregada(s), do maior para o menor.
fn distribuir_fases(entradas: &mut [Entrada], nfases: usize) -> [f64; 3] {
    let mut w = [0.0f64; 3];
    let carga = |e: &Entrada| if e.bifasico { e.potencia / 2.0 } else { e.potencia };
    for e in entradas.iter().filter(|e| e.fase.is_some()) {
        for f in e.fase.iter().flatten() {
            w[*f] += carga(e);
        }
    }
    let mut livres: Vec<usize> = (0..entradas.len()).filter(|i| entradas[*i].fase.is_none()).collect();
    livres.sort_by(|a, b| entradas[*b].potencia.total_cmp(&entradas[*a].potencia));
    for i in livres {
        let e = &mut entradas[i];
        let c = carga(e);
        let escolha: Vec<usize> = if nfases == 1 {
            vec![0]
        } else if e.bifasico {
            let pares: Vec<[usize; 2]> = (0..nfases).flat_map(|a| ((a + 1)..nfases).map(move |b| [a, b])).collect();
            let melhor = pares.into_iter().min_by(|x, y| (w[x[0]] + w[x[1]]).total_cmp(&(w[y[0]] + w[y[1]]))).unwrap_or([0, 1]);
            melhor.to_vec()
        } else {
            vec![(0..nfases).min_by(|a, b| w[*a].total_cmp(&w[*b])).unwrap_or(0)]
        };
        for f in &escolha {
            w[*f] += c;
        }
        e.fase = Some(escolha);
    }
    w
}

fn queda_pct(ib: f64, comprimento: f64, secao: f64, tensao: f64, trifasico: bool) -> f64 {
    let k = if trifasico { 3f64.sqrt() } else { 2.0 };
    k * RHO * comprimento * ib / secao / tensao * 100.0
}

/// A menor seção que atende: mínimo da norma, capacidade ≥ disjuntor (já com os fatores) e queda de tensão.
struct Pedido {
    disjuntor: f64,
    fator: f64,
    minima: f64,
    ib: f64,
    comprimento: f64,
    tensao: f64,
    tabela: &'static [f64; 12],
    trifasico: bool,
    limite_dv: f64,
}

/// (seção, capacidade corrigida, queda %, atende?)
fn escolher_secao(p: &Pedido) -> (f64, f64, f64, bool) {
    let mut ultimo = (SECOES[SECOES.len() - 1], 0.0, 0.0, false);
    for (i, s) in SECOES.iter().enumerate().filter(|(_, s)| **s >= p.minima) {
        let iz = p.tabela[i] * p.fator;
        let dv = queda_pct(p.ib, p.comprimento, *s, p.tensao, p.trifasico);
        ultimo = (*s, iz, dv, iz >= p.disjuntor && dv <= p.limite_dv);
        if ultimo.3 {
            return ultimo;
        }
    }
    ultimo
}

fn dimensionar(e: Entrada, q: &Quadro) -> Res<Linha> {
    let tensao = if e.bifasico { q.vff } else { q.vfn };
    let ib = e.potencia / (tensao * e.fp);
    let disjuntor = proximo_disjuntor(ib.max(e.disjuntor_minimo)).ok_or_else(|| format!("circuito {}: corrente de {ib:.0} A passa do maior disjuntor da tabela (250 A)", e.id))?;
    let fator = fator_temperatura(q.temperatura) * fator_agrupamento(e.agrupamento);
    let (secao, iz, dv, ok) = escolher_secao(&Pedido { disjuntor, fator, minima: e.tipo.secao_minima(), ib, comprimento: e.comprimento, tensao, tabela: &IZ_2, trifasico: false, limite_dv: q.limite_dv });
    let dr = e.dr.then(|| DR_PADRAO.iter().copied().find(|d| *d >= disjuntor)).flatten();
    let fases = e.fase.clone().unwrap_or_default();
    Ok(Linha { e, fases, ib, disjuntor, dr, secao, iz, dv, ok })
}

fn dimensionar_geral(i_fase: [f64; 3], q: &Quadro) -> Res<Geral> {
    let ib = i_fase.iter().copied().fold(0.0, f64::max) * q.fator_demanda;
    let disjuntor = proximo_disjuntor(ib).ok_or_else(|| format!("alimentador: corrente de {ib:.0} A passa do maior disjuntor da tabela (250 A)"))?;
    let trifasico = q.fases == 3;
    let (tabela, tensao) = if trifasico { (&IZ_3, q.vff) } else { (&IZ_2, q.vfn) };
    let fator = fator_temperatura(q.temperatura);
    let (secao, iz, dv, _) = escolher_secao(&Pedido { disjuntor, fator, minima: 2.5, ib, comprimento: q.alimentador_m, tensao, tabela, trifasico, limite_dv: 2.0 });
    Ok(Geral { ib, disjuntor, secao, iz, dv })
}

fn avisos_de(c: &Calculo) -> Vec<String> {
    let mut a = Vec::new();
    for l in &c.linhas {
        if !l.ok {
            a.push(format!("{}: não fecha (capacidade {:.1} A contra disjuntor de {} A, queda {:.2} % contra o limite de {} %) — reduza o comprimento, divida o circuito ou informe uma seção maior", l.e.id, l.iz, l.disjuntor, l.dv, c.quadro.limite_dv));
        }
    }
    if c.quadro.fases > 1 {
        let usadas = &c.w_fase[..c.quadro.fases];
        let media = usadas.iter().sum::<f64>() / usadas.len() as f64;
        let max = usadas.iter().copied().fold(f64::MIN, f64::max);
        let min = usadas.iter().copied().fold(f64::MAX, f64::min);
        if media > 0.0 && (max - min) / media * 100.0 > 10.0 {
            a.push(format!("fases desequilibradas: {:.0} % entre a mais e a menos carregada (a prática costuma pedir até cerca de 10 %; é premissa) — redistribua com `fase` nos circuitos", (max - min) / media * 100.0));
        }
    }
    a
}

fn calcular(args: &Value) -> Res<Calculo> {
    let quadro = quadro_de(args)?;
    let lista = args.get("circuitos").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`circuitos` precisa ser uma lista com ao menos um circuito")?;
    let mut entradas: Vec<Entrada> = lista.iter().enumerate().map(|(i, c)| entrada_de(i, c, &quadro)).collect::<Res<_>>()?;
    let w_fase = distribuir_fases(&mut entradas, quadro.fases);
    let linhas: Vec<Linha> = entradas.into_iter().map(|e| dimensionar(e, &quadro)).collect::<Res<_>>()?;
    let mut i_fase = [0.0f64; 3];
    for l in &linhas {
        for f in &l.fases {
            i_fase[*f] += l.ib;
        }
    }
    let geral = dimensionar_geral(i_fase, &quadro)?;
    let mut c = Calculo { quadro, linhas, w_fase, i_fase, geral, avisos: Vec::new() };
    c.avisos = avisos_de(&c);
    Ok(c)
}

fn fases_texto(f: &[usize]) -> String {
    f.iter().map(|i| LETRAS[*i]).collect()
}

fn esquema(l: &Linha) -> &'static str {
    if l.e.bifasico { "F+F+T" } else { "F+N+T" }
}

fn linha_json(l: &Linha, c: &Calculo) -> Value {
    json!({
        "id": l.e.id, "descricao": l.e.descricao, "tipo": l.e.tipo.nome(), "esquema": esquema(l), "metodo": "B1",
        "tensao_v": if l.e.bifasico { c.quadro.vff } else { c.quadro.vfn }, "potencia_w": l.e.potencia, "fp": l.e.fp,
        "potencia_va": (l.e.potencia / l.e.fp).round(), "fase": fases_texto(&l.fases), "ib_a": round(l.ib, 2),
        "secao_mm2": l.secao, "iz_corrigida_a": round(l.iz, 1), "disjuntor_a": l.disjuntor, "dr_a": l.dr,
        "dv_parcial_pct": round(l.dv, 2), "dv_total_pct": round(l.dv + c.geral.dv, 2), "comprimento_m": l.e.comprimento,
        "agrupamento": l.e.agrupamento, "status": if l.ok { "OK" } else { "REVER" },
    })
}

fn round(v: f64, casas: i32) -> f64 {
    let k = 10f64.powi(casas);
    (v * k).round() / k
}

fn demanda(c: &Calculo, fatores: Option<&Value>) -> Vec<(String, f64, f64)> {
    let mut grupos: BTreeMap<&'static str, f64> = BTreeMap::new();
    for l in &c.linhas {
        *grupos.entry(l.e.tipo.nome()).or_insert(0.0) += l.e.potencia;
    }
    grupos
        .into_iter()
        .map(|(t, p)| {
            let f = fatores.and_then(|o| o.get(t)).and_then(Value::as_f64).filter(|f| *f > 0.0 && *f <= 1.0).unwrap_or(1.0);
            (t.to_string(), p, f)
        })
        .collect()
}

fn resultado(c: &Calculo, fatores: Option<&Value>) -> Value {
    let total: f64 = c.linhas.iter().map(|l| l.e.potencia).sum();
    let va: f64 = c.linhas.iter().map(|l| l.e.potencia / l.e.fp).sum();
    let dem = demanda(c, fatores);
    let dem_w: f64 = dem.iter().map(|(_, p, f)| p * f).sum();
    json!({
        "quadro": { "nome": c.quadro.nome, "tensao_fn": c.quadro.vfn, "tensao_ff": c.quadro.vff, "fases": c.quadro.fases },
        "circuitos": c.linhas.iter().map(|l| linha_json(l, c)).collect::<Vec<_>>(),
        "totais": { "potencia_w": total, "potencia_va": va.round(), "por_fase_w": { "R": c.w_fase[0].round(), "S": c.w_fase[1].round(), "T": c.w_fase[2].round() },
                    "corrente_por_fase_a": { "R": round(c.i_fase[0], 1), "S": round(c.i_fase[1], 1), "T": round(c.i_fase[2], 1) } },
        "alimentador": { "ib_a": round(c.geral.ib, 1), "disjuntor_geral_a": c.geral.disjuntor, "secao_mm2": c.geral.secao, "iz_corrigida_a": round(c.geral.iz, 1),
                         "dv_pct": round(c.geral.dv, 2), "comprimento_m": c.quadro.alimentador_m, "condutores": if c.quadro.fases == 3 { "3F+N+T" } else { "F+N+T" } },
        "demanda": { "grupos": dem.iter().map(|(t, p, f)| json!({ "tipo": t, "instalada_w": p, "fator": f, "demanda_w": round(p * f, 0) })).collect::<Vec<_>>(), "total_w": round(dem_w, 0), "total_kva": round(dem_w / (total / va), 2) },
        "premissas": ["cobre, isolação PVC 70 °C, eletroduto, método B1 (NBR 5410 tab. 36/37), condutores carregados: 2 nos circuitos e 3 no alimentador trifásico",
                      format!("temperatura ambiente {} °C, agrupamento por circuito (campo `agrupamento`)", c.quadro.temperatura),
                      format!("queda de tensão parcial no circuito ≤ {} % (e ≤ 2 % no alimentador), resistividade do cobre a 70 °C = {RHO} Ω·mm²/m, sem reatância", c.quadro.limite_dv),
                      "disjuntor = menor padronizado ≥ Ib (e ≥ `disjuntor_minimo_a`, p.ex. 10 A para circuitos de LED por causa da corrente de partida); seção = menor que dá capacidade ≥ disjuntor e queda dentro do limite, nunca abaixo de 1,5 (iluminação) ou 2,5 mm² (demais)",
                      "fatores de demanda valem 1,0 se não informados em `fatores_demanda` — não são da norma",
                      "DR de 30 mA nas tomadas e circuitos específicos (padrão; `dr` no circuito muda)"],
        "avisos": c.avisos,
    })
}

// ---------- desenho ----------

fn texto_n(v: f64, d: usize) -> String {
    formatar(v, d, true)
}

fn colunas_do_quadro() -> Vec<Value> {
    let c = |t: &str, l: f64, a: &str| json!({ "titulo": t, "largura": l, "alinhamento": a });
    vec![
        c("Circ.", 12.0, "centro"), c("Descrição", 64.0, "esquerda"), c("Esquema", 15.0, "centro"), c("Tensão (V)", 13.0, "centro"),
        c("Pot. (W)", 14.0, "direita"), c("FP", 9.0, "centro"), c("Fase", 11.0, "centro"), c("R (W)", 12.0, "direita"), c("S (W)", 12.0, "direita"),
        c("T (W)", 12.0, "direita"), c("Ib (A)", 11.0, "direita"), c("Seção (mm²)", 14.0, "centro"), c("Iz corr. (A)", 13.0, "direita"),
        c("Disj. (A)", 11.0, "centro"), c("DR (A)", 10.0, "centro"), c("dV parc. (%)", 13.0, "direita"), c("dV total (%)", 13.0, "direita"), c("Status", 12.0, "centro"),
    ]
}

fn celulas_do_circuito(l: &Linha, c: &Calculo) -> Value {
    let mut w = [String::new(), String::new(), String::new()];
    for f in &l.fases {
        w[*f] = format!("{:.0}", if l.e.bifasico { l.e.potencia / 2.0 } else { l.e.potencia });
    }
    json!([l.e.id, l.e.descricao, esquema(l), format!("{:.0}", if l.e.bifasico { c.quadro.vff } else { c.quadro.vfn }), format!("{:.0}", l.e.potencia),
           texto_n(l.e.fp, 2), fases_texto(&l.fases), w[0], w[1], w[2], texto_n(l.ib, 1), texto_n(l.secao, 1), texto_n(l.iz, 1), format!("{:.0}", l.disjuntor),
           l.dr.map_or("-".to_string(), |d| format!("{d:.0}")), texto_n(l.dv, 2), texto_n(l.dv + c.geral.dv, 2), if l.ok { "OK" } else { "REVER" }])
}

fn linhas_do_quadro(c: &Calculo) -> Vec<Value> {
    let mut linhas: Vec<Value> = c.linhas.iter().map(|l| celulas_do_circuito(l, c)).collect();
    let total: f64 = c.linhas.iter().map(|l| l.e.potencia).sum();
    let usadas: String = (0..c.quadro.fases).map(|i| LETRAS[i]).collect();
    linhas.push(json!(["TOTAL", "", "", "", format!("{total:.0}"), "", usadas, format!("{:.0}", c.w_fase[0]), format!("{:.0}", c.w_fase[1]), format!("{:.0}", c.w_fase[2]), "", "", "", "", "", "", "", ""]));
    linhas.push(json!(["GERAL", format!("Alimentador do {} (L = {} m, {} condutores)", c.quadro.nome, texto_n(c.quadro.alimentador_m, 0), if c.quadro.fases == 3 { "3F+N+T" } else { "F+N+T" }),
                       "", "", "", "", "", "", "", "", texto_n(c.geral.ib, 1), texto_n(c.geral.secao, 1), texto_n(c.geral.iz, 1), format!("{:.0}", c.geral.disjuntor), "", texto_n(c.geral.dv, 2), "", "OK"]));
    linhas
}

/// Desenha o quadro de cargas com `cad_tabela`.
fn desenhar_quadro(doc: &mut Documento, c: &Calculo, a: &Value) -> Res<Value> {
    let titulo = format!("QUADRO DE CARGAS - {} ({:.0}/{:.0} V, {})", c.quadro.nome, c.quadro.vff, c.quadro.vfn, if c.quadro.fases == 3 { "3F+N+T" } else { "F+N+T" });
    let mut args = json!({ "origem": a.get("origem").cloned().unwrap_or(Value::Null), "titulo": titulo, "colunas": colunas_do_quadro(), "linhas": linhas_do_quadro(c),
        "altura_linha": 4.5, "altura_texto": 2.0, "virgula": true, "camada": "DI-Tabelas" });
    copiar(a, &mut args, &["escala", "camada", "altura_linha", "altura_texto", "cor"]);
    crate::tabela::tabela(doc, &args)
}

fn copiar(de: &Value, para: &mut Value, chaves: &[&str]) {
    for k in chaves {
        if let Some(v) = de.get(*k) {
            para[*k] = v.clone();
        }
    }
}

/// Desenha o quadro de demanda com `cad_tabela`.
fn desenhar_demanda(doc: &mut Documento, c: &Calculo, a: &Value, fatores: Option<&Value>) -> Res<Value> {
    let dem = demanda(c, fatores);
    let total_p: f64 = dem.iter().map(|(_, p, _)| p).sum();
    let total_d: f64 = dem.iter().map(|(_, p, f)| p * f).sum();
    let mut linhas: Vec<Value> = dem.iter().map(|(t, p, f)| json!([t, format!("{p:.0}"), texto_n(*f, 2), format!("{:.0}", p * f)])).collect();
    linhas.push(json!(["TOTAL", format!("{total_p:.0}"), "", format!("{total_d:.0}")]));
    let mut args = json!({ "origem": a.get("origem").cloned().unwrap_or(Value::Null), "titulo": format!("QUADRO DE DEMANDA - {}", c.quadro.nome),
        "colunas": [{ "titulo": "Tipo de carga", "largura": 40.0 }, { "titulo": "Instalada (W)", "largura": 28.0, "alinhamento": "direita" }, { "titulo": "Fator de demanda", "largura": 30.0, "alinhamento": "centro" }, { "titulo": "Demanda (W)", "largura": 28.0, "alinhamento": "direita" }],
        "linhas": linhas, "altura_linha": 4.5, "altura_texto": 2.0, "virgula": true, "camada": "DI-Tabelas" });
    copiar(a, &mut args, &["escala", "camada", "altura_linha", "altura_texto", "cor"]);
    crate::tabela::tabela(doc, &args)
}

/// O desenho do unifilar em coordenadas locais (mm de papel, y para baixo a partir da origem).
struct Folha<'a> {
    p: Pincel,
    origem: Ponto,
    k: f64,
    h: f64,
    camadas: &'a Camadas,
}

struct Camadas {
    linhas: String,
    disjuntor: String,
    textos: String,
}

impl Folha<'_> {
    fn pt(&self, x: f64, y: f64) -> Ponto {
        Ponto::novo(self.origem.x + x * self.k, self.origem.y - y * self.k)
    }

    fn linha(&mut self, a: (f64, f64), b: (f64, f64)) {
        let (pa, pb) = (self.pt(a.0, a.1), self.pt(b.0, b.1));
        self.p.em(&self.camadas.linhas, None).linha(pa, pb);
    }

    fn texto(&mut self, x: f64, y: f64, t: &str, al: AlinhaH, fator: f64) {
        let ponto = self.pt(x, y);
        let altura = self.h * fator * self.k;
        self.p.em(&self.camadas.textos, None).texto(ponto, altura, 0.0, t, al, AlinhaV::Meio);
    }

    fn retangulo(&mut self, a: (f64, f64), b: (f64, f64), camada_dos_disjuntores: bool) {
        let (pa, pb) = (self.pt(a.0, a.1.max(b.1)), self.pt(b.0, a.1.min(b.1)));
        let camada = if camada_dos_disjuntores { self.camadas.disjuntor.clone() } else { self.camadas.linhas.clone() };
        self.p.em(&camada, None).retangulo(pa, pb);
    }

    /// Disjuntor: contato com a lâmina inclinada, entre `x` e `x + 6`.
    fn disjuntor(&mut self, x: f64, y: f64) {
        let (pa, pb) = (self.pt(x, y), self.pt(x + 5.0, y - 3.2));
        let camada = self.camadas.disjuntor.clone();
        self.p.em(&camada, None).linha(pa, pb);
    }

    /// Traços de condutor cruzando a linha em `x`.
    fn marcas(&mut self, x: f64, y: f64, n: usize) {
        for i in 0..n {
            let dx = x + i as f64 * 1.1;
            self.linha((dx - 0.6, y + 1.8), (dx + 0.6, y - 1.8));
        }
    }

    fn terra(&mut self, x: f64, y: f64) {
        self.linha((x, y), (x, y + 4.0));
        for (i, meia) in [3.0, 2.0, 1.0].iter().enumerate() {
            self.linha((x - meia, y + 4.0 + i as f64 * 1.2), (x + meia, y + 4.0 + i as f64 * 1.2));
        }
    }
}

const PASSO: f64 = 13.0;
const X_BARRA: f64 = 62.0;
const X_QUADRO_FIM: f64 = 128.0;

fn circuito_no_unifilar(f: &mut Folha, l: &Linha, i: usize) {
    let y = i as f64 * PASSO;
    f.linha((X_BARRA, y), (70.0, y));
    f.disjuntor(70.0, y);
    f.texto(73.0, y - 5.0, &format!("{:.0} A", l.disjuntor), AlinhaH::Centro, 0.9);
    f.linha((76.0, y), (X_QUADRO_FIM, y));
    if let Some(dr) = l.dr {
        f.retangulo((85.0, y - 3.2), (95.0, y + 3.2), true);
        f.texto(90.0, y, "DR", AlinhaH::Centro, 0.9);
        f.texto(90.0, y - 5.0, &format!("{dr:.0} A / 30 mA"), AlinhaH::Centro, 0.9);
    }
    f.linha((X_QUADRO_FIM, y), (150.0, y));
    f.marcas(135.0, y, 2);
    f.texto(137.0, y + 3.6, &texto_n(l.secao, 1), AlinhaH::Centro, 0.9);
    f.texto(146.0, y - 2.6, &fases_texto(&l.fases), AlinhaH::Centro, 0.9);
    f.texto(152.0, y - 0.2, &format!("({:.0} W) {} {}", l.e.potencia, l.e.id, l.e.descricao), AlinhaH::Esquerda, 1.0);
}

/// O unifilar: alimentação, disjuntor geral, quadro com barramento e uma linha por circuito (disjuntor, DR, seção, fase, carga).
fn desenhar_unifilar(doc: &mut Documento, c: &Calculo, a: &Value) -> Res<Value> {
    let origem = ponto_de(a, "origem")?;
    let k = fator_papel(doc, a)?;
    let camadas = Camadas { linhas: texto_opcional(a, "camada").unwrap_or("DI-Linhas do diagrama").to_string(), disjuntor: "DI-Disjuntor".into(), textos: "DI-Textos do diagrama".into() };
    let h = numero_opcional(a, "altura_texto").unwrap_or(2.5);
    let mut f = Folha { p: Pincel::novo(&camadas.linhas), origem, k, h, camadas: &camadas };
    let n = c.linhas.len();
    let fim = (n.saturating_sub(1)) as f64 * PASSO;
    let meio = fim / 2.0;
    f.retangulo((48.0, -9.0), (X_QUADRO_FIM, fim + 8.0), false);
    let total: f64 = c.linhas.iter().map(|l| l.e.potencia).sum();
    f.texto(50.0, -5.0, &c.quadro.nome, AlinhaH::Esquerda, 1.1);
    f.texto(X_QUADRO_FIM - 2.0, -5.0, &format!("{total:.0} W"), AlinhaH::Direita, 1.0);
    f.linha((0.0, meio), (48.0, meio));
    f.disjuntor(12.0, meio);
    f.texto(15.0, meio - 5.0, &format!("{:.0} A", c.geral.disjuntor), AlinhaH::Centro, 0.9);
    f.marcas(32.0, meio, if c.quadro.fases == 3 { 4 } else { 2 });
    f.texto(34.0, meio + 4.2, &format!("{} #{} mm²", if c.quadro.fases == 3 { "3F+N+T" } else { "F+N+T" }, texto_n(c.geral.secao, 1)), AlinhaH::Centro, 0.9);
    f.linha((X_BARRA, 0.0), (X_BARRA, fim));
    f.retangulo((X_BARRA - 0.5, 0.0), (X_BARRA + 0.5, fim), true);
    f.linha((48.0, meio), (X_BARRA, meio));
    for (i, l) in c.linhas.iter().enumerate() {
        circuito_no_unifilar(&mut f, l, i);
    }
    f.terra(85.0, fim + 8.0);
    let caixa = caixa_das(&f.p.pecas);
    let ids = aplicar(doc, f.p.pecas)?;
    Ok(relatorio(&ids, caixa))
}

/// `cad_circuitos`: calcula e, se pedido em `desenhar`, desenha `quadro_de_cargas`, `demanda` e `unifilar`.
pub fn circuitos(ag: &mut Agente, args: &Value) -> Res<Value> {
    let c = calcular(args)?;
    let fatores = args.get("fatores_demanda");
    let mut r = resultado(&c, fatores);
    if let Some(d) = args.get("desenhar").filter(|d| d.is_object()) {
        let mut desenhado = serde_json::Map::new();
        if let Some(a) = d.get("quadro_de_cargas") {
            desenhado.insert("quadro_de_cargas".into(), desenhar_quadro(&mut ag.doc, &c, a)?);
        }
        if let Some(a) = d.get("demanda") {
            desenhado.insert("demanda".into(), desenhar_demanda(&mut ag.doc, &c, a, fatores)?);
        }
        if let Some(a) = d.get("unifilar") {
            desenhado.insert("unifilar".into(), desenhar_unifilar(&mut ag.doc, &c, a)?);
        }
        r["desenhado"] = Value::Object(desenhado);
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dados() -> Value {
        json!({
            "quadro": { "nome": "QDL", "fases": 3, "alimentador_m": 10 },
            "circuitos": [
                { "id": "C1", "descricao": "Fileira 1", "tipo": "iluminacao", "potencia_w": 750, "comprimento_m": 30 },
                { "id": "C2", "descricao": "Fileira 2", "tipo": "iluminacao", "potencia_w": 750, "comprimento_m": 30 },
                { "id": "C3", "descricao": "Fileira 3", "tipo": "iluminacao", "potencia_w": 750, "comprimento_m": 30 },
                { "id": "C4", "descricao": "Tomadas", "tipo": "tug", "potencia_w": 1200, "comprimento_m": 15 },
            ]
        })
    }

    #[test]
    fn corrente_secao_e_disjuntor_do_circuito_de_iluminacao() {
        let c = calcular(&dados()).unwrap();
        let l = &c.linhas[0];
        assert!((l.ib - 750.0 / (127.0 * 0.92)).abs() < 1e-9);
        assert!((l.disjuntor - 10.0).abs() < 1e-9);
        // 1,5 mm² daria ~4,5 % em 30 m: passa para 2,5 mm² (queda ~2,7 %)
        assert!((l.secao - 2.5).abs() < 1e-9, "{}", l.secao);
        assert!(l.dv < 4.0 && l.ok);
    }

    #[test]
    fn fases_sao_distribuidas_e_o_alimentador_sai_do_pior_caso() {
        let c = calcular(&dados()).unwrap();
        // a maior carga (tomadas, 1200 W) entra primeiro na fase R; as demais completam S e T antes de repetir
        assert_eq!(c.linhas[3].fases, vec![0]);
        assert!(c.w_fase.iter().all(|w| *w > 0.0), "{:?}", c.w_fase);
        assert!((c.w_fase.iter().sum::<f64>() - 3450.0).abs() < 1e-9);
        assert!(c.geral.disjuntor >= c.geral.ib);
        assert!(c.linhas[3].dr.is_some() && c.linhas[0].dr.is_none());
    }

    #[test]
    fn fase_pedida_vale_e_letras_erradas_sao_recusadas() {
        let mut d = dados();
        d["circuitos"][0]["fase"] = json!("T");
        assert_eq!(calcular(&d).unwrap().linhas[0].fases, vec![2]);
        d["circuitos"][0]["fase"] = json!("RS");
        assert!(calcular(&d).is_err());
        d["circuitos"][0]["fase"] = json!("X");
        assert!(calcular(&d).is_err());
    }

    #[test]
    fn circuito_entre_fases_divide_a_potencia_nas_duas() {
        let mut d = dados();
        d["circuitos"] = json!([{ "id": "C1", "tipo": "tue", "tensao": "ff", "potencia_w": 5400, "fase": "RS", "comprimento_m": 10 }]);
        let c = calcular(&d).unwrap();
        assert!((c.w_fase[0] - 2700.0).abs() < 1e-9 && (c.w_fase[1] - 2700.0).abs() < 1e-9 && c.w_fase[2] == 0.0);
        assert!((c.linhas[0].ib - 5400.0 / 220.0).abs() < 1e-9);
        assert!(c.avisos.iter().any(|a| a.contains("desequilibradas")));
    }

    #[test]
    fn disjuntor_minimo_sobe_o_disjuntor_sem_mudar_a_corrente() {
        let mut d = dados();
        d["quadro"]["disjuntor_minimo_a"] = json!(16);
        let c = calcular(&d).unwrap();
        assert!((c.linhas[0].disjuntor - 16.0).abs() < 1e-9);
        assert!((c.linhas[0].ib - 750.0 / (127.0 * 0.92)).abs() < 1e-9);
    }

    #[test]
    fn desenha_quadro_demanda_e_unifilar_numa_chamada() {
        let mut ag = Agente { doc: Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro)), ..Agente::default() };
        let mut d = dados();
        d["desenhar"] = json!({ "quadro_de_cargas": { "origem": [0, 100], "escala": 100 }, "demanda": { "origem": [0, 50], "escala": 100 }, "unifilar": { "origem": [0, 0], "escala": 100 } });
        let r = circuitos(&mut ag, &d).unwrap();
        for k in ["quadro_de_cargas", "demanda", "unifilar"] {
            assert!(r["desenhado"][k]["criadas"].as_u64().unwrap() > 10, "{k}: {r}");
        }
        assert!(r["desenhado"]["quadro_de_cargas"]["avisos"].as_array().unwrap().is_empty(), "{r}");
    }
}
