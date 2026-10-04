//! `cad_modelos` e `cad_modelo`: um projeto inteiro numa chamada, a partir dos dados. O modelo `ferrovia` lê uma nuvem de
//! pontos de via férrea e entrega as pranchas: planta com detalhes das curvas críticas; gráficos (superelevação,
//! curvatura, ripagem, perfil, levante) e quadro de curvas; seção tipo e perspectiva da curva crítica; memorial; e a
//! tabela por estaca. Roda em modo projeto (pouco contexto): a resposta traz só números e avisos.

use feathercad_core::folha::InfoFolha;
use feathercad_core::viario::superelevacao::{calcular, h_em, ResultadoCurva};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::ferramentas::chamar;
use crate::fonte::Fonte;
use crate::json::{arred, numero_opcional, texto_opcional, Res};

/// Pasta dos modelos do usuário (`$FEATHERCAD_MODELOS` ou `~/.config/feathercad/modelos`): um `.json` por modelo, no formato
/// `{descricao, esquema, dados_padrao?, passos, retornar?}` — os `passos`/`retornar` são os de `cad_lote`, e `dados` do pedido
/// (sobre `dados_padrao`) vira o `dados` do lote.
fn pasta_do_usuario() -> Option<std::path::PathBuf> {
    std::env::var("FEATHERCAD_MODELOS").ok().map(std::path::PathBuf::from).or_else(|| std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".config/feathercad/modelos")))
}

/// (nome, conteúdo) de cada modelo do usuário que lê; arquivo quebrado vira um modelo com `erro` em vez de sumir.
fn do_usuario() -> Vec<(String, Value)> {
    let Some(dir) = pasta_do_usuario() else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<(String, Value)> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let nome = p.file_stem()?.to_string_lossy().into_owned();
            let conteudo = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| format!("JSON inválido: {e}")));
            Some((nome, conteudo.unwrap_or_else(|e| json!({ "erro": e }))))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

pub fn listar() -> Value {
    let mut r = listar_de_fabrica();
    if let Some(lista) = r["modelos"].as_array_mut() {
        for (nome, m) in do_usuario() {
            lista.push(json!({ "nome": nome, "origem": "usuario", "descricao": m["descricao"], "esquema": m["esquema"], "erro": m["erro"] }));
        }
    }
    r
}

fn listar_de_fabrica() -> Value {
    json!({ "modelos": [{
        "nome": "ferrovia",
        "descricao": "Projeto de correção de traçado ferroviário a partir de uma nuvem de pontos do levantamento: eixo existente, alinhamento com transições, superelevação, greide, quadros, gráficos, seção tipo, perspectiva, memorial e tabela por estaca, em folhas.",
        "esquema": {
            "nuvem": "caminho do .csv/.xyz com E,N,Z (obrigatório)", "V": "velocidade diretriz, km/h (65)", "bitola": "entre faces internas, m (1,0)",
            "regras": "nome de tabela ou objeto parcial (ferrovia_metrica)", "papel": "papel das pranchas (A0)", "escala": "denominador; omitido = a menor em que o traçado cabe",
            "inicio": "[E,N] se o rastreio do eixo precisar de ajuda", "direcao": "graus", "voltas": "rodadas do ajuste (3)",
            "carimbo": "{cliente, fase, disciplina, titulo, responsavel, numero, revisao}", "saida": "{dxf, fcdz, pdf}: caminhos a gravar no fim"
        },
        "resultado": "folhas inseridas, ripagem RMS/máxima, curvas (tabela), levante, e verificação de leiaute",
    }] })
}

pub(crate) fn passo(ag: &mut Agente, fonte: &Fonte, nome: &str, args: &Value) -> Res<Value> {
    let c = chamar(ag, fonte, nome, args).map_err(|e| format!("{nome}: {e}"))?;
    let t = c.iter().find_map(|b| b.get("text").and_then(Value::as_str)).unwrap_or("{}");
    serde_json::from_str(t).map_err(|e| format!("{nome}: resposta ilegível: {e}"))
}

/// Um passo "bonito" (1, 2, 5 × 10ⁿ) para ~`n` marcas no intervalo.
pub(crate) fn passo_bonito(intervalo: f64, n: f64) -> f64 {
    let bruto = (intervalo / n).max(1e-9);
    let mag = 10f64.powf(bruto.log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * mag).find(|p| *p >= bruto).unwrap_or(10.0 * mag)
}

pub(crate) fn arredondar_limites(lo: f64, hi: f64, passo: f64) -> (f64, f64) {
    ((lo / passo).floor() * passo, (hi / passo).ceil() * passo)
}

/// Ponto da folha: `x_mm`,`y_mm` do canto inferior esquerdo, em unidades do desenho.
pub(crate) fn na_folha(f: &InfoFolha, x_mm: f64, y_mm: f64) -> Value {
    let k = f.escala / 1000.0;
    json!([f.min.x + x_mm * k, f.min.y + y_mm * k])
}

pub(crate) fn inserir_folha(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, papel: &str, escala: Option<f64>, titulo: &str, n: usize) -> Res<InfoFolha> {
    let mut campos = ctx.carimbo.clone();
    campos["titulo"] = json!(titulo);
    campos["folha"] = json!(format!("{n:02}/{:02}", ctx.total));
    let mut a = json!({ "acao": "inserir", "papel": papel, "campos": campos });
    if let Some(e) = escala {
        a["escala"] = json!(e);
    }
    let r = passo(ag, fonte, "cad_folha", &a)?;
    let j = &r["folha"];
    let n = |v: &Value| v.as_f64().unwrap_or(0.0);
    let p = |v: &Value| feathercad_core::geom::Ponto::novo(n(&v["x"]), n(&v["y"]));
    Ok(InfoFolha {
        indice: j["indice"].as_u64().unwrap_or(0) as usize, id: j["id"].as_u64().unwrap_or(0), nome: j["nome"].as_str().unwrap_or("").to_string(), papel: j["papel"].as_str().unwrap_or("").to_string(),
        largura_mm: n(&j["largura_mm"]), altura_mm: n(&j["altura_mm"]), escala: n(&j["escala"]), escala_texto: j["escala_texto"].as_str().unwrap_or("").to_string(), min: p(&j["min"]), max: p(&j["max"]),
    })
}

pub(crate) struct Contexto {
    pub(crate) papel: String,
    pub(crate) escala: f64,
    pub(crate) carimbo: Value,
    pub(crate) total: usize,
}

fn serie_h(a: &feathercad_core::viario::alinhamento::Alinhamento, tabela: &[ResultadoCurva]) -> (Vec<Value>, Vec<Value>) {
    let n = (a.comprimento / 10.0).floor() as usize;
    let h = (0..=n).map(|i| { let s = a.progressiva_inicial + i as f64 * 10.0; json!([s, h_em(a, tabela, s)]) }).collect();
    let k = (0..=n).map(|i| { let s = a.progressiva_inicial + i as f64 * 10.0; json!([s, a.em(s).curvatura.abs() * 1000.0]) }).collect();
    (h, k)
}

/// Um gráfico empilhado: `idx` é a posição de cima para baixo na folha.
struct Painel<'a> {
    idx: usize,
    titulo: &'a str,
    rotulo_y: &'a str,
    x: (f64, f64, f64),
    y: (f64, f64, f64),
}

fn grafico(ag: &mut Agente, fonte: &Fonte, f: &InfoFolha, ctx: &Contexto, p: &Painel, series: Vec<Value>) -> Res<()> {
    let Painel { idx, titulo, rotulo_y, x, y } = *p;
    let (largura, altura, vao) = (f.largura_mm - 150.0, 80.0, 38.0);
    let y0 = f.altura_mm - 90.0 - altura - idx as f64 * (altura + vao);
    let args = json!({ "origem": na_folha(f, 90.0, y0), "escala": ctx.escala, "largura": largura, "altura": altura, "titulo": titulo, "altura_texto": 3.0,
        "eixo_x": { "de": x.0, "ate": x.1, "marcas": x.2, "rotulo": if idx == 4 { "progressiva (m)" } else { "" } },
        "eixo_y": { "de": y.0, "ate": y.1, "marcas": y.2, "rotulo": rotulo_y }, "series": series });
    passo(ag, fonte, "cad_grafico", &args).map(|_| ())
}

fn prancha_graficos(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, n: usize, tabela: &[ResultadoCurva]) -> Res<()> {
    let a = ag.alinhamentos.values().next().ok_or("sem alinhamento")?.0.clone();
    let f = inserir_folha(ag, fonte, ctx, &ctx.papel, Some(ctx.escala), "Superelevação, curvatura, ripagem, perfil e levante", n)?;
    let (h, k) = serie_h(&a, tabela);
    let passo_x = passo_bonito(a.comprimento, 12.0);
    let x = (a.progressiva_inicial, (a.progressiva_final() / passo_x).ceil() * passo_x, passo_x);
    let feic = ag.feicoes.values().next().cloned();
    let (mut rip, mut niv, mut zex, mut zpr, mut lev) = (vec![], vec![], vec![], vec![], vec![]);
    if let (Some(fe), Some(g)) = (feic, ag.greides.values().next()) {
        let t = feathercad_core::trajeto::Trajeto::novo(&a.amostrar(0.5)).ok_or("alinhamento vazio")?;
        for (i, p) in fe.pares.iter().enumerate().filter(|(i, _)| !fe.suspeitos.contains(i)) {
            let (d, s) = t.projetar(p.meio);
            let s = s + a.progressiva_inicial;
            let sinal = if a.em(s).curvatura.abs() > 1e-9 { -a.em(s).curvatura.signum() } else { 1.0 };
            rip.push(json!([s, d]));
            niv.push(json!([s, p.nivelamento * 1000.0 * sinal]));
            zex.push(json!([s, p.z]));
            lev.push(json!([s, g.z(s - a.progressiva_inicial) - p.z]));
            let _ = i;
        }
        let n = (a.comprimento / 10.0).floor() as usize;
        zpr = (0..=n).map(|i| { let s = i as f64 * 10.0; json!([a.progressiva_inicial + s, g.z(s)]) }).collect();
    }
    let faixa = |v: &[Value]| v.iter().filter_map(|p| p.get(1).and_then(Value::as_f64)).fold((f64::MAX, f64::MIN), |m, y| (m.0.min(y), m.1.max(y)));
    let (hmax, kmax) = (faixa(&h).1.max(faixa(&niv).1).max(10.0), faixa(&k).1.max(1e-3));
    let py = passo_bonito(hmax * 1.1, 5.0);
    grafico(ag, fonte, &f, ctx, &Painel { idx: 0, titulo: "Superelevação (mm): projeto em degrau, existente em cruz", rotulo_y: "h (mm)", x, y: (0.0f64.min(faixa(&niv).0.floor()), (hmax * 1.1 / py).ceil() * py, py) }, vec![json!({ "pontos": h, "estilo": "linha", "camada": "GRAFICO_PROJETO", "cor": "#FF0000" }), json!({ "pontos": niv, "estilo": "cruz", "camada": "GRAFICO_EXISTENTE", "cor": "#00B0FF" })])?;
    let pk = passo_bonito(kmax * 1.1, 4.0);
    grafico(ag, fonte, &f, ctx, &Painel { idx: 1, titulo: "Curvatura do traçado de projeto (1/R × 1000)", rotulo_y: "1000/R", x, y: (0.0, (kmax * 1.1 / pk).ceil() * pk, pk) }, vec![json!({ "pontos": k, "estilo": "linha", "camada": "GRAFICO_PROJETO", "cor": "#FF0000" })])?;
    let rmax = faixa(&rip);
    let (r0, r1) = arredondar_limites(rmax.0.min(-0.1), rmax.1.max(0.1), 0.5);
    grafico(ag, fonte, &f, ctx, &Painel { idx: 2, titulo: "Ripagem (m): existente em relação ao projeto", rotulo_y: "ripagem (m)", x, y: (r0, r1, passo_bonito(r1 - r0, 5.0)) }, vec![json!({ "pontos": rip, "estilo": "cruz", "camada": "GRAFICO_EXISTENTE", "cor": "#00B0FF" })])?;
    let (zlo, zhi) = (faixa(&zex).0.min(faixa(&zpr).0), faixa(&zex).1.max(faixa(&zpr).1));
    let pz = passo_bonito((zhi - zlo).max(1.0), 6.0);
    let (z0, z1) = arredondar_limites(zlo, zhi, pz);
    grafico(ag, fonte, &f, ctx, &Painel { idx: 3, titulo: "Perfil (m): existente em cruz, greide de projeto em linha", rotulo_y: "cota (m)", x, y: (z0, z1, pz) }, vec![json!({ "pontos": zpr, "estilo": "linha", "camada": "GRAFICO_PROJETO", "cor": "#FF0000" }), json!({ "pontos": zex, "estilo": "cruz", "camada": "GRAFICO_EXISTENTE", "cor": "#00B0FF" })])?;
    let lmax = faixa(&lev);
    let (l0, l1) = arredondar_limites(lmax.0.min(0.0), lmax.1.max(0.1), 0.1);
    grafico(ag, fonte, &f, ctx, &Painel { idx: 4, titulo: "Levante (m): projeto − existente", rotulo_y: "levante (m)", x, y: (l0, l1, passo_bonito(l1 - l0, 5.0)) }, vec![json!({ "pontos": lev, "estilo": "cruz", "camada": "GRAFICO_EXISTENTE", "cor": "#00B0FF" })])?;
    quadro_de_curvas(ag, fonte, &f, ctx, tabela)
}

fn quadro_de_curvas(ag: &mut Agente, fonte: &Fonte, f: &InfoFolha, ctx: &Contexto, tabela: &[ResultadoCurva]) -> Res<()> {
    let a = ag.alinhamentos.values().next().ok_or("sem alinhamento")?.0.clone();
    let linhas: Vec<Value> = a.curvas.iter().zip(tabela).enumerate().map(|(i, (c, r))| json!([i + 1, if c.lado > 0 { "E" } else { "D" }, c.delta_graus, c.raio, c.ls, c.lc, c.ts, feathercad_core::trajeto::estaca_texto(c.ts_marco.progressiva, 20.0), feathercad_core::trajeto::estaca_texto(c.st.progressiva, 20.0), r.he_mm, r.h_mm, r.i_mm])).collect();
    let col = |t: &str, l: f64, d: usize| json!({ "titulo": t, "largura": l, "alinhamento": "centro", "decimais": d });
    let args = json!({ "origem": na_folha(f, 90.0, 190.0), "escala": ctx.escala, "titulo": "QUADRO DE CURVAS", "altura_linha": 6, "altura_texto": 3,
        "colunas": [col("Curva", 18.0, 0), col("Lado", 14.0, 0), col("Δ (°)", 22.0, 2), col("R (m)", 22.0, 0), col("Ls (m)", 20.0, 0), col("Lc (m)", 24.0, 1), col("Ts (m)", 24.0, 1), col("TS", 30.0, 0), col("ST", 30.0, 0), col("he (mm)", 24.0, 1), col("h (mm)", 22.0, 0), col("I (mm)", 22.0, 1)],
        "linhas": linhas });
    passo(ag, fonte, "cad_tabela", &args).map(|_| ())
}

fn prancha_planta(ag: &mut Agente, fonte: &Fonte, ctx: &mut Contexto, n: usize, tabela: &[ResultadoCurva]) -> Res<InfoFolha> {
    let f = inserir_folha(ag, fonte, ctx, &ctx.papel.clone(), Some(ctx.escala).filter(|e| *e > 0.0), "Planta: eixo existente × traçado de projeto", n)?;
    ctx.escala = f.escala;
    let a = ag.alinhamentos.values().next().ok_or("sem alinhamento")?.0.clone();
    let (min, max) = (f.min, f.max);
    passo(ag, fonte, "cad_malha_coordenadas", &json!({ "caixa": [[min.x + 0.03 * (max.x - min.x), min.y + 0.03 * (max.y - min.y)], [max.x - 0.03 * (max.x - min.x), max.y - 0.13 * (max.y - min.y)]], "passo": passo_bonito(max.x - min.x, 8.0), "escala": f.escala, "altura_texto": 3.0 }))?;
    passo(ag, fonte, "cad_norte", &json!({ "ponto": na_folha(&f, f.largura_mm - 60.0, f.altura_mm - 75.0), "tipo": "rosa", "tamanho": 30.0, "escala": f.escala }))?;
    // detalhes das duas curvas de maior superelevação
    let mut ordem: Vec<usize> = (0..a.curvas.len()).collect();
    ordem.sort_by(|x, y| tabela[*y].h_mm.total_cmp(&tabela[*x].h_mm).then(a.curvas[*x].raio.total_cmp(&a.curvas[*y].raio)));
    for (k, &i) in ordem.iter().take(2).enumerate() {
        let c = &a.curvas[i];
        let (p0, p1) = (c.ts_marco.ponto, c.st.ponto);
        let centro = ((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
        let meia = (p0.distancia(p1) / 2.0).max(c.e * 1.5).max(40.0) * 1.1;
        let (lado_mm, kk) = (380.0, f.escala / 1000.0);
        let amp = lado_mm * kk / (2.0 * meia);
        passo(ag, fonte, "cad_detalhe", &json!({ "janela": [[centro.0 - meia, centro.1 - meia], [centro.0 + meia, centro.1 + meia]], "ponto": na_folha(&f, if k == 0 { 30.0 } else { f.largura_mm - lado_mm - 30.0 }, 190.0), "escala_detalhe": f.escala / amp, "escala_folha": f.escala,
            "titulo": format!("DETALHE {} - CURVA {}", ["A", "B"][k], i + 1), "malha": passo_bonito(2.0 * meia, 4.0), "altura_texto": 3.0 }))?;
    }
    Ok(f)
}

/// Linhas da tabela por estaca que cabem numa folha A1 (3 blocos de 52 linhas, texto de 3,5 mm).
const LINHAS_POR_FOLHA_DE_ESTACAS: usize = 156;

/// O eixo existente em trechos contínuos (quebra onde a nuvem tem lacuna de mais de 60 m ou um par suspeito).
fn eixo_existente(f: &crate::nuvem_ferramentas::Feicoes) -> Vec<Vec<feathercad_core::geom::Ponto>> {
    let mut segs: Vec<Vec<feathercad_core::geom::Ponto>> = vec![Vec::new()];
    for (i, p) in f.pares.iter().enumerate() {
        let atual = segs.last_mut().expect("sempre há um trecho");
        if f.suspeitos.contains(&i) || atual.last().is_some_and(|u| u.distancia(p.meio) > 60.0) {
            if !atual.is_empty() {
                segs.push(Vec::new());
            }
            if f.suspeitos.contains(&i) {
                continue;
            }
        }
        segs.last_mut().expect("sempre há um trecho").push(p.meio);
    }
    segs.retain(|s| s.len() > 1);
    segs
}

/// A tabela por estaca em `n_folhas` folhas A1, cada uma com 3 blocos legíveis.
fn prancha_estacas(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, primeira: usize, n_folhas: usize, a: &feathercad_core::viario::alinhamento::Alinhamento) -> Res<()> {
    let total_linhas = (a.comprimento / 20.0).floor() as usize + 1;
    let por_folha = total_linhas.div_ceil(n_folhas.max(1));
    for k in 0..n_folhas {
        let f = inserir_folha(ag, fonte, ctx, "A1", Some(ctx.escala), "Tabela por estaca", primeira + k)?;
        let (de, ate) = (a.progressiva_inicial + (k * por_folha) as f64 * 20.0, a.progressiva_inicial + (((k + 1) * por_folha) as f64 - 1.0) * 20.0);
        passo(ag, fonte, "cad_tabela_estacas", &json!({ "colunas": ["estaca", "elemento", "curva", "h_atual", "h_projeto", "flecha_projetada", "ripagem", "cota_existente", "cota_projeto", "levante"], "de": de, "ate": ate,
            "desenhar_tabela": { "origem": na_folha(&f, 40.0, f.altura_mm - 45.0), "escala": ctx.escala, "titulo": format!("TABELA POR ESTACA — FOLHA {} DE {}", k + 1, n_folhas), "blocos": 3, "altura_linha": 7.4, "altura_texto": 3.5, "largura_fator": 1.7 } }))?;
    }
    Ok(())
}

/// Roda o modelo `nome` da pasta do usuário como um `cad_lote` (um passo de desfazer, tudo ou nada).
fn modelo_do_usuario(ag: &mut Agente, fonte: &Fonte, nome: &str, args: &Value) -> Res<Value> {
    let (_, m) = do_usuario().into_iter().find(|(n, _)| n.eq_ignore_ascii_case(nome)).ok_or_else(|| format!("modelo `{nome}` não existe (veja cad_modelos; os do usuário ficam em ~/.config/feathercad/modelos/*.json)"))?;
    if let Some(e) = m.get("erro") {
        return Err(format!("modelo `{nome}`: {e}"));
    }
    let passos = m.get("passos").cloned().ok_or_else(|| format!("modelo `{nome}`: falta `passos` (os de cad_lote)"))?;
    let mut dados = m.get("dados_padrao").and_then(Value::as_object).cloned().unwrap_or_default();
    for (k, v) in args.get("dados").and_then(Value::as_object).into_iter().flatten() {
        dados.insert(k.clone(), v.clone());
    }
    let mut lote = json!({ "dados": dados, "passos": passos });
    if let Some(r) = m.get("retornar") {
        lote["retornar"] = r.clone();
    }
    let anterior = ag.projeto;
    ag.projeto = true;
    let r = passo(ag, fonte, "cad_lote", &lote);
    ag.projeto = anterior;
    r.map(|mut v| {
        v["modelo"] = json!(nome);
        v
    })
}

/// `cad_modelo {nome: "ferrovia", dados: {…}}`.
pub fn modelo(ag: &mut Agente, fonte: &Fonte, args: &Value) -> Res<Value> {
    let nome = texto_opcional(args, "nome").unwrap_or("");
    if nome != "ferrovia" {
        return modelo_do_usuario(ag, fonte, nome, args);
    }
    let d = args.get("dados").ok_or("falta `dados` (veja o `esquema` em cad_modelos)")?;
    let caminho = texto_opcional(d, "nuvem").ok_or("`dados.nuvem`: caminho do arquivo de pontos")?;
    let anterior = ag.projeto;
    ag.projeto = true;
    let r = ferrovia(ag, fonte, d, caminho);
    ag.projeto = anterior;
    r
}

/// Tudo o que o cálculo do projeto produz e as folhas leem.
struct Calculo {
    a: feathercad_core::viario::alinhamento::Alinhamento,
    tabela: Vec<ResultadoCurva>,
    regras: feathercad_core::viario::superelevacao::Regras,
    greide: feathercad_core::viario::greide::Greide,
    obs: Vec<feathercad_core::viario::criticidade::Observacao>,
    analise: feathercad_core::viario::criticidade::Analise,
    existente: crate::nuvem_ferramentas::Feicoes,
    /// Respostas das ferramentas, para o resumo final.
    secoes: Value,
    feic: Value,
    qual: Value,
    ajuste: Value,
    sup: Value,
    g: Value,
    regras_args: Value,
}

/// Os pontos do levantamento projetados no traçado: progressiva, cota e ripagem (suspeitos de fora).
fn observacoes(a: &feathercad_core::viario::alinhamento::Alinhamento, f: &crate::nuvem_ferramentas::Feicoes) -> Vec<feathercad_core::viario::criticidade::Observacao> {
    let t = feathercad_core::trajeto::Trajeto::novo(&a.amostrar(0.5));
    let mut v: Vec<_> = f
        .pares
        .iter()
        .enumerate()
        .filter(|(i, _)| !f.suspeitos.contains(i))
        .filter_map(|(_, p)| t.as_ref().map(|t| { let (d, s) = t.projetar(p.meio); feathercad_core::viario::criticidade::Observacao { s: s + a.progressiva_inicial, z: p.z, ripagem: d } }))
        .collect();
    v.sort_by(|x, y| x.s.total_cmp(&y.s));
    v
}

/// Os limiares da análise: os padrões de `Criterios` com o que vier em `dados.criterios` por cima.
fn criterios_de(d: &Value) -> feathercad_core::viario::criticidade::Criterios {
    let mut c = feathercad_core::viario::criticidade::Criterios::default();
    let Some(o) = d.get("criterios").and_then(Value::as_object) else { return c };
    let n = |k: &str| o.get(k).and_then(Value::as_f64);
    let campos: [(&str, &mut f64); 14] = [("fracao_h", &mut c.fracao_h), ("fracao_i", &mut c.fracao_i), ("folga_r", &mut c.folga_r), ("folga_v", &mut c.folga_v), ("folga_ls", &mut c.folga_ls), ("ripagem_atencao", &mut c.ripagem_atencao), ("ripagem_critica", &mut c.ripagem_critica), ("movimento_minimo", &mut c.movimento_minimo), ("movimento_atencao", &mut c.movimento_atencao), ("movimento_critico", &mut c.movimento_critico), ("rampa_atencao_pct", &mut c.rampa_atencao_pct), ("rampa_critica_pct", &mut c.rampa_critica_pct), ("tangente_minima", &mut c.tangente_minima), ("largura_plataforma", &mut c.largura_plataforma)];
    for (nome, campo) in campos {
        if let Some(v) = n(nome) {
            *campo = v;
        }
    }
    c
}

/// Nuvem → feições → traçado ajustado → superelevação → greide → análise de criticidade.
fn calcular_projeto(ag: &mut Agente, fonte: &Fonte, d: &Value, caminho: &str) -> Res<Calculo> {
    let regras_args = json!({ "regras": d.get("regras").cloned().unwrap_or(Value::Null), "V": d.get("V").cloned().unwrap_or(json!(65)) });
    passo(ag, fonte, "cad_novo", &json!({ "unidade": "m" }))?;
    let mut sec = json!({ "caminho": caminho });
    for c in ["inicio", "direcao"] {
        if let Some(v) = d.get(c).filter(|v| !v.is_null()) {
            sec[c] = v.clone();
        }
    }
    let secoes = passo(ag, fonte, "cad_nuvem_secoes", &sec)?;
    let feic = passo(ag, fonte, "cad_nuvem_feicoes", &json!({ "bitola": d.get("bitola").cloned().unwrap_or(json!(1.0)) }))?;
    let qual = passo(ag, fonte, "cad_nuvem_qualidade", &json!({ "marcar": true }))?;
    let mut aj = json!({ "voltas": d.get("voltas").cloned().unwrap_or(json!(6)), "regras": regras_args["regras"], "V": regras_args["V"] });
    if aj["regras"].is_null() {
        aj.as_object_mut().map(|o| o.remove("regras"));
    }
    let ajuste = passo(ag, fonte, "cad_alinhamento_ajustar", &aj)?;
    let sup = passo(ag, fonte, "cad_superelevacao", &regras_args)?;
    // greide `compensado` (padrão: mínimos quadrados, corta e aterra ao redor do existente) ou `so_levante` (a via só sobe)
    let regra = if texto_opcional(d, "greide") == Some("so_levante") { json!({}) } else { json!({ "regra": "minimos_quadrados" }) };
    let g = passo(ag, fonte, "cad_greide", &regra)?;
    let a = ag.alinhamentos.values().next().ok_or("sem alinhamento")?.0.clone();
    let regras = crate::regras::carregar(&regras_args)?;
    let tabela = calcular(&a, &regras);
    let existente = ag.feicoes.values().next().ok_or("sem feições")?.clone();
    let greide = ag.greides.values().next().ok_or("sem greide")?.clone();
    let obs = observacoes(&a, &existente);
    let analise = feathercad_core::viario::criticidade::analisar(&a, &tabela, &regras, &greide, &obs, &criterios_de(d));
    Ok(Calculo { a, tabela, regras, greide, obs, analise, existente, secoes, feic, qual, ajuste, sup, g, regras_args })
}

/// O eixo existente (azul) e o de projeto no desenho, que a planta geral e os detalhes copiam.
fn desenhar_eixos(ag: &mut Agente, fonte: &Fonte, c: &Calculo) -> Res<Value> {
    let f = &c.existente;
    let pontos: Vec<Value> = f.pares.iter().enumerate().filter(|(i, _)| !f.suspeitos.contains(i)).map(|(_, p)| json!([p.meio.x, p.meio.y])).collect();
    passo(ag, fonte, "cad_criar", &json!({ "formas": [{ "tipo": "polilinha", "pontos": pontos }], "camada": "EXISTENTE", "cor": "#00B0FF" }))?;
    let curvas = c.ajuste["raios"].as_array().map(|r| r.iter().zip(c.a.curvas.iter()).map(|(r, cv)| json!({ "R": r, "Ls": cv.ls })).collect::<Vec<_>>());
    let eixo = json!({ "pis": c.ajuste["pis"], "curvas": curvas, "desenhar": { "eixo": true }, "camada_base": "PROJETO", "nome": "A1" });
    passo(ag, fonte, "cad_alinhamento", &eixo)?;
    Ok(json!({ "pis": c.ajuste["pis"], "curvas": curvas }))
}

/// Quantas folhas de cada tipo o projeto terá (o carimbo mostra `n/total`).
struct Contagem {
    perfis: usize,
    plantas: usize,
    estacas: usize,
}

impl Contagem {
    fn de(a: &feathercad_core::viario::alinhamento::Alinhamento, trechos: usize) -> Self {
        Self { perfis: crate::perfil::numero_de_folhas(a.comprimento), plantas: trechos, estacas: ((a.comprimento / 20.0).floor() as usize + 1).div_ceil(LINHAS_POR_FOLHA_DE_ESTACAS) }
    }
    fn total(&self) -> usize {
        4 + self.perfis + self.plantas + self.estacas
    }
}

fn montar_folhas(ag: &mut Agente, fonte: &Fonte, d: &Value, c: &Calculo, eixo: &Value) -> Res<f64> {
    let trechos = crate::planta_a1::dividir(&c.a);
    let n = Contagem::de(&c.a, trechos.len());
    let carimbo = d.get("carimbo").cloned().unwrap_or(json!({}));
    let mut ctx = Contexto { papel: texto_opcional(d, "papel").unwrap_or("A0").to_string(), escala: numero_opcional(d, "escala").unwrap_or(0.0), carimbo, total: n.total() };
    let f1 = prancha_planta(ag, fonte, &mut ctx, 1, &c.tabela)?;
    // textos da planta já na escala certa
    let rot = json!({ "pis": eixo["pis"], "curvas": eixo["curvas"], "desenhar": { "estacas": true, "km": true, "notaveis": true, "rotulos": false }, "camada_base": "PROJETO", "escala": f1.escala, "nome": "A1", "altura_texto": 3.0, "rotular_estacas_a_cada": ((35.0 * f1.escala / 1000.0 / 20.0 / 5.0).ceil() * 5.0).max(5.0) });
    passo(ag, fonte, "cad_alinhamento", &rot)?;
    prancha_graficos(ag, fonte, &ctx, 2, &c.tabela)?;
    crate::pranchas::prancha_secao_e_3d(ag, fonte, &ctx, 3, &c.a, &c.tabela)?;
    let num = |v: &Value| v.as_f64().unwrap_or(0.0);
    let resumo = crate::pranchas::Resumo { comprimento: c.a.comprimento, pontos: c.secoes["pontos"].as_u64().unwrap_or(0) as usize, pares: c.feic["pares"].as_u64().unwrap_or(0) as usize, suspeitos: c.qual["cota_fora_da_mediana"]["total"].as_u64().unwrap_or(0) as usize,
        rip_rms: num(&c.ajuste["ripagem"]["rms"]), rip_max: num(&c.ajuste["ripagem"]["maxima"]), lev_medio: num(&c.g["levante"]["medio"]), lev_max: num(&c.g["levante"]["maximo"]), pivs: c.g["pivs"].as_array().map_or(0, Vec::len) };
    crate::pranchas::prancha_memorial(ag, fonte, &ctx, 4, (&c.a, &c.tabela, &c.regras, &resumo), c.sup["memorial"].as_str().unwrap_or(""))?;
    let projeto = crate::projeto::Projeto { a: &c.a, tabela: &c.tabela, regras: &c.regras, greide: &c.greide, analise: &c.analise, obs: &c.obs };
    crate::perfil::perfis(ag, fonte, &ctx, &projeto, 5)?;
    let existente = eixo_existente(&c.existente);
    crate::planta_a1::plantas(ag, fonte, &ctx, (&c.a, &c.tabela, &existente), 5 + n.perfis, &trechos)?;
    prancha_estacas(ag, fonte, &ctx, 5 + n.perfis + n.plantas, n.estacas, &c.a)?;
    Ok(f1.escala)
}

fn ferrovia(ag: &mut Agente, fonte: &Fonte, d: &Value, caminho: &str) -> Res<Value> {
    let c = calcular_projeto(ag, fonte, d, caminho)?;
    let eixo = desenhar_eixos(ag, fonte, &c)?;
    let escala = montar_folhas(ag, fonte, d, &c, &eixo)?;
    let leiaute = crate::leiaute::verificar_leiaute(&ag.doc);
    let mut saidas = serde_json::Map::new();
    if let Some(s) = d.get("saida") {
        for (chave, tool) in [("dxf", "cad_salvar"), ("fcdz", "cad_salvar")] {
            if let Some(cam) = texto_opcional(s, chave) {
                saidas.insert(chave.into(), passo(ag, fonte, tool, &json!({ "caminho": cam, "sobrescrever": true }))?);
            }
        }
        if let Some(cam) = texto_opcional(s, "pdf") {
            saidas.insert("pdf".into(), passo(ag, fonte, "cad_pdf", &json!({ "caminho": cam, "papel": "auto", "sobrescrever": true }))?);
        }
    }
    let niveis = |n| c.analise.contagem(n);
    Ok(json!({
        "modelo": "ferrovia", "folhas": ag.doc.folhas_inseridas().len(), "escala": escala,
        "nuvem": { "pontos": c.secoes["pontos"], "pares_de_trilho": c.feic["pares"], "suspeitos_por_cota": c.qual["cota_fora_da_mediana"]["total"] },
        "alinhamento": { "comprimento": c.ajuste["comprimento"], "curvas": c.ajuste["curvas_n"], "ripagem": c.ajuste["ripagem"] },
        "superelevacao": { "todas_atendem": c.sup["todas_atendem"] }, "greide": { "levante": c.g["levante"], "pivs": c.g["pivs"].as_array().map(Vec::len) },
        "criticidade": { "criticos": niveis(feathercad_core::viario::criticidade::Nivel::Critico), "atencao": niveis(feathercad_core::viario::criticidade::Nivel::Atencao), "normais": niveis(feathercad_core::viario::criticidade::Nivel::Normal),
            "volumes_m3": { "aterro": arred(c.analise.volumes.aterro_m3), "escavacao": arred(c.analise.volumes.escavacao_m3) } },
        "curvas": c.a.curvas.iter().zip(&c.tabela).enumerate().map(|(i, (cv, r))| json!({ "curva": i + 1, "R": cv.raio, "Ls": cv.ls, "Lc": arred(cv.lc), "h_mm": r.h_mm, "ok": r.ok })).collect::<Vec<_>>(),
        "leiaute": { "problemas": leiaute["contagem"], "textos_conferidos": leiaute["textos_conferidos"] }, "saidas": saidas,
        "proximo_passo": "cad_verificar {leiaute:true} e cad_ver {arquivo:\"x.png\", janela:…} por folha; ajuste com as ferramentas individuais",
    }))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn passo_bonito_escolhe_1_2_5() {
        assert_eq!(passo_bonito(1000.0, 10.0), 100.0);
        assert_eq!(passo_bonito(6300.0, 12.0), 1000.0);
        assert_eq!(passo_bonito(1.0, 5.0), 0.2);
    }

    #[test]
    fn modelo_ferrovia_de_ponta_a_ponta_com_uma_via_sintetica() {
        use feathercad_core::geom::Ponto;
        use feathercad_core::viario::alinhamento::{Alinhamento, EntradaCurva};
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(700.0, 0.0), Ponto::novo(700.0 + 700.0 * d.cos(), 700.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        let mut t = String::from("E,N,Z\n");
        for i in 0..=(a.comprimento / 5.0) as usize {
            let s = i as f64 * 5.0;
            let am = a.em(s);
            let r = am.direcao.to_radians();
            for lado in [0.53, -0.53] {
                t += &format!("{:.3},{:.3},{:.4}\n", am.ponto.x - r.sin() * lado, am.ponto.y + r.cos() * lado, 10.0 + s * 0.001 + (s / 60.0).sin() * 0.05);
            }
        }
        let csv = std::env::temp_dir().join(format!("fcad_modelo_{}.csv", std::process::id()));
        std::fs::write(&csv, t).unwrap();
        let mut ag = Agente::default();
        let fonte = crate::fonte::carregar();
        let r = modelo(&mut ag, &fonte, &json!({ "nome": "ferrovia", "dados": { "nuvem": csv.display().to_string(), "V": 80, "voltas": 1, "inicio": [0, 0], "direcao": 0, "carimbo": { "cliente": "Teste", "titulo": "Via sintética" } } })).unwrap();
        assert!(r["folhas"].as_u64().unwrap() >= 5, "memorial, seção, gráficos, planta geral, ao menos uma planta e a tabela: {r}");
        assert!(r["alinhamento"]["ripagem"]["rms"].as_f64().unwrap() < 1.5, "{r}");
        assert!(!ag.projeto, "o modo projeto volta ao que era");
        assert!(ag.doc.entidades().len() > 300, "{} entidades", ag.doc.entidades().len());
        let _ = std::fs::remove_file(&csv);
    }

    #[test]
    fn listar_traz_o_esquema_da_ferrovia() {
        assert_eq!(listar()["modelos"][0]["nome"], "ferrovia");
        assert!(modelo(&mut Agente::default(), &crate::fonte::carregar(), &json!({ "nome": "x", "dados": {} })).is_err());
    }

    #[test]
    fn modelo_do_usuario_roda_como_lote_com_dados_padrao() {
        let dir = std::env::temp_dir().join(format!("fcad_modelos_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("duas_linhas.json"), r#"{"descricao":"d","esquema":{"x":"fim"},"dados_padrao":{"x":10},
            "passos":[{"repetir":"=2","como":"i","faca":[{"ferramenta":"cad_criar","argumentos":{"formas":[{"tipo":"linha","a":"=[0, i]","b":"=[x, i]"}]}}]}]}"#).unwrap();
        std::fs::write(dir.join("quebrado.json"), "{").unwrap();
        std::env::set_var("FEATHERCAD_MODELOS", &dir);
        let l = listar();
        assert!(l["modelos"].as_array().unwrap().iter().any(|m| m["nome"] == "duas_linhas" && m["origem"] == "usuario"));
        assert!(l["modelos"].as_array().unwrap().iter().any(|m| m["nome"] == "quebrado" && m["erro"].is_string()));
        let mut ag = Agente::default();
        let fonte = crate::fonte::carregar();
        ag.doc = feathercad_core::documento::Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let r = modelo(&mut ag, &fonte, &json!({ "nome": "duas_linhas", "dados": { "x": 25 } }));
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(ag.doc.entidades().len(), 2);
        assert!(modelo(&mut ag, &fonte, &json!({ "nome": "quebrado" })).is_err());
        std::env::remove_var("FEATHERCAD_MODELOS");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
