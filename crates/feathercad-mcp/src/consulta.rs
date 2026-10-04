//! As perguntas que a IA faz ao desenho: o que há aqui (`resumo`), quais entidades (`entidades`),
//! quanto mede (`medir`), quais áreas se fecham (`regioes`) e o que está errado (`verificar`).

use std::collections::BTreeMap;

use feathercad_core::anotacao::area_e_perimetro;
use feathercad_core::documento::{Documento, Entidade, Unidade};
use feathercad_core::edicao::{caixa_da_entidade, distancia, na_janela};
use feathercad_core::geom::Ponto;
use feathercad_core::regiao::{regiao_em, todas_as_regioes};
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{arred, janela, lista_de_pontos, numero_de, numero_opcional, par, ponto, ponto_de, texto_opcional, Res};
use crate::medidas::{centroide, descrever, tipo_de};

/// O que toda resposta lembra sobre o sistema de coordenadas.
const CONVENCAO: &str = "X cresce para a direita, Y para cima; ângulos em graus, anti-horário a partir do eixo X (0° = direita, 90° = cima); as medidas estão na unidade do desenho";

fn caixa_json(c: Option<(Ponto, Ponto)>) -> Value {
    match c {
        Some((min, max)) => json!({ "min": par(min), "max": par(max), "largura": arred(max.x - min.x), "altura": arred(max.y - min.y), "centro": par(Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0)) }),
        None => Value::Null,
    }
}

/// Visão geral: unidade, extensão, contagens por tipo e camada, e o estado do arquivo.
pub fn resumo(a: &Agente) -> Value {
    let d = &a.doc;
    let mut por_tipo: BTreeMap<&str, usize> = BTreeMap::new();
    let mut por_camada: Vec<usize> = vec![0; d.camadas().len()];
    for e in d.entidades() {
        *por_tipo.entry(tipo_de(&e.geometria)).or_default() += 1;
        if let Some(c) = por_camada.get_mut(e.camada) {
            *c += 1;
        }
    }
    let camadas: Vec<Value> = d.camadas().iter().enumerate().map(|(i, c)| json!({ "nome": c.nome, "cor": format!("#{:06X}", c.cor), "ligada": c.ligada, "congelada": c.congelada, "bloqueada": c.bloqueada, "entidades": por_camada[i], "atual": i == d.camada_atual() })).collect();
    json!({
        "unidade": d.unidade().map(Unidade::nome), "extensao": caixa_json(d.caixa()), "total_de_entidades": d.entidades().len(),
        "por_tipo": por_tipo, "camadas": camadas, "blocos": d.blocos().iter().map(|b| b.nome.clone()).collect::<Vec<_>>(),
        "arquivo": a.arquivo.as_ref().map(|p| p.display().to_string()), "alterado_desde_o_ultimo_salvar": d.sujo(),
        "pode_desfazer": d.pode_desfazer(), "pode_refazer": d.pode_refazer(), "convencao": CONVENCAO,
    })
}

/// Se a entidade passa em todos os filtros de `args` (`tipo`, `camada`, `ids`, `janela`).
fn passa(doc: &Documento, e: &Entidade, args: &Value, jan: Option<((Ponto, Ponto), bool)>) -> bool {
    if texto_opcional(args, "tipo").is_some_and(|t| t != tipo_de(&e.geometria)) {
        return false;
    }
    if texto_opcional(args, "camada").is_some_and(|c| !doc.camadas().get(e.camada).is_some_and(|k| k.nome.eq_ignore_ascii_case(c))) {
        return false;
    }
    if let Some(ids) = args.get("ids").and_then(Value::as_array) {
        if !ids.iter().any(|i| i.as_u64() == Some(e.id)) {
            return false;
        }
    }
    jan.map_or(true, |(j, cruzando)| na_janela(doc, &e.geometria, j, cruzando))
}

/// A janela de seleção dos argumentos (`janela` + `modo`: `dentro` | `toca`), se houver.
fn janela_do(args: &Value) -> Res<Option<((Ponto, Ponto), bool)>> {
    match args.get("janela") {
        Some(j) => Ok(Some((janela(j)?, texto_opcional(args, "modo") == Some("toca")))),
        None => Ok(None),
    }
}

/// Os ids que passam nos filtros `ids`, `tipo`, `camada` e `janela` de `args`.
pub fn passa_filtros(d: &Documento, args: &Value) -> Res<Vec<u64>> {
    let jan = janela_do(args)?;
    Ok(d.entidades().iter().filter(|e| passa(d, e, args, jan)).map(|e| e.id).collect())
}

/// Lista entidades com medidas. Filtros: `tipo`, `camada`, `ids`, `janela` (+ `modo`: dentro | toca) e
/// `perto` {ponto, raio} (traz `distancia` e `direcao` a partir do ponto, da mais perto para a mais longe).
pub fn entidades(d: &Documento, args: &Value) -> Res<Value> {
    let limite = numero_opcional(args, "limite").map_or(100, |n| n.clamp(1.0, 500.0) as usize);
    let jan = janela_do(args)?;
    let perto = match args.get("perto") {
        Some(p) => Some((ponto_de(p, "ponto")?, numero_de(p, "raio")?)),
        None => None,
    };
    let mut achadas: Vec<(f64, Value)> = Vec::new();
    for e in d.entidades().iter().filter(|e| passa(d, e, args, jan)) {
        let mut v = descrever(d, e);
        let dist = perto.map_or(0.0, |(p, _)| distancia(d, &e.geometria, p));
        if let Some((p, raio)) = perto {
            if dist > raio {
                continue;
            }
            v["distancia"] = json!(arred(dist));
            if let Some((min, max)) = caixa_da_entidade(d, &e.geometria) {
                v["direcao_do_ponto"] = json!(arred(p.angulo_ate(Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0))));
            }
        }
        achadas.push((dist, v));
    }
    achadas.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total = achadas.len();
    Ok(json!({ "total": total, "retornadas": total.min(limite), "entidades": achadas.into_iter().take(limite).map(|(_, v)| v).collect::<Vec<_>>() }))
}

/// Uma ponta de medição: um ponto `[x, y]` ou `{"id": n}` (a entidade).
enum Ponta {
    Pt(Ponto),
    Ent(u64),
}

fn ponta(v: &Value, nome: &str) -> Res<Ponta> {
    match v.get("id").and_then(Value::as_u64) {
        Some(id) => Ok(Ponta::Ent(id)),
        None => Ok(Ponta::Pt(ponto(v, nome)?)),
    }
}

/// Amostra da forma de uma entidade (pontos do contorno) para medir distâncias entre entidades.
fn amostra(d: &Documento, e: &Entidade) -> Vec<Ponto> {
    let mut v = Vec::new();
    e.geometria.para_cada_ponto_notavel(&mut |p| v.push(p));
    if let Some((min, max)) = caixa_da_entidade(d, &e.geometria) {
        for k in 0..=16 {
            let t = f64::from(k) / 16.0;
            v.push(Ponto::novo(min.x + (max.x - min.x) * t, min.y + (max.y - min.y) * t));
        }
    }
    v
}

fn entidade_de(d: &Documento, id: u64) -> Res<&Entidade> {
    d.entidade(id).ok_or_else(|| format!("não existe a entidade {id}"))
}

/// Distância e direção entre dois pontos, ou a distância mínima aproximada entre uma entidade e um ponto/outra entidade.
fn entre(d: &Documento, de: &Ponta, para: &Ponta) -> Res<Value> {
    match (de, para) {
        (Ponta::Pt(a), Ponta::Pt(b)) => Ok(json!({ "distancia": arred(a.distancia(*b)), "dx": arred(b.x - a.x), "dy": arred(b.y - a.y), "angulo": arred(a.angulo_ate(*b)) })),
        (Ponta::Pt(p), Ponta::Ent(id)) | (Ponta::Ent(id), Ponta::Pt(p)) => Ok(json!({ "distancia_minima": arred(distancia(d, &entidade_de(d, *id)?.geometria, *p)), "nota": "do ponto até a forma da entidade (0 = em cima)" })),
        (Ponta::Ent(x), Ponta::Ent(y)) => {
            let (ea, eb) = (entidade_de(d, *x)?, entidade_de(d, *y)?);
            let minima = amostra(d, ea).iter().map(|p| distancia(d, &eb.geometria, *p)).chain(amostra(d, eb).iter().map(|p| distancia(d, &ea.geometria, *p))).fold(f64::INFINITY, f64::min);
            Ok(json!({ "distancia_minima_aproximada": arred(minima), "nota": "amostrada nos pontos notáveis e na caixa; 0 = se tocam" }))
        }
    }
}

/// Medições: `{de, para}` (pontos ou `{id}`), `{pontos}` (perímetro e área do polígono) ou `{vertice, a, b}` (ângulo).
pub fn medir(d: &Documento, args: &Value) -> Res<Value> {
    if let Some(t) = texto_opcional(args, "texto") {
        let altura = numero_opcional(args, "altura").filter(|a| *a > 0.0).ok_or("`texto` precisa de `altura` maior que zero")?;
        let largura = feathercad_io::pdf::largura_do_conteudo(t, altura);
        return Ok(json!({ "largura": arred(largura), "altura": altura, "nota": "mesma métrica do PDF; para caber numa caixa de largura L use altura <= L / largura * altura" }));
    }
    if let (Some(de), Some(para)) = (args.get("de"), args.get("para")) {
        return entre(d, &ponta(de, "de")?, &ponta(para, "para")?);
    }
    if let Some(p) = args.get("pontos") {
        let pts = lista_de_pontos(p, "pontos")?;
        let (area, perimetro) = area_e_perimetro(&pts);
        return Ok(json!({ "area": arred(area), "perimetro_fechado": arred(perimetro), "centro": par(centroide(&pts)), "nota": "o polígono liga o último ponto ao primeiro" }));
    }
    if let Some(v) = args.get("vertice") {
        let (v, a, b) = (ponto(v, "vertice")?, ponto_de(args, "a")?, ponto_de(args, "b")?);
        let mut g = (v.angulo_ate(b) - v.angulo_ate(a)).rem_euclid(360.0);
        if g > 180.0 {
            g = 360.0 - g;
        }
        return Ok(json!({ "angulo_entre": arred(g), "de_a_para_b_anti_horario": arred((v.angulo_ate(b) - v.angulo_ate(a)).rem_euclid(360.0)) }));
    }
    Err("passe `de` e `para`, ou `pontos`, ou `vertice` com `a` e `b`".into())
}

/// A descrição de uma região: área, perímetro, centro, caixa, os textos que caem dentro.
fn descrever_regiao(d: &Documento, area: f64, pts: &[Ponto]) -> Value {
    let (_, perimetro) = area_e_perimetro(pts);
    let c = centroide(pts);
    let (min, max) = pts.iter().fold((pts[0], pts[0]), |(mn, mx), q| (Ponto::novo(mn.x.min(q.x), mn.y.min(q.y)), Ponto::novo(mx.x.max(q.x), mx.y.max(q.y))));
    let textos: Vec<Value> = d
        .entidades()
        .iter()
        .filter_map(|e| match &e.geometria {
            feathercad_core::documento::Geometria::Texto { ponto: p, conteudo, .. } if dentro(pts, *p) => Some(json!(conteudo)),
            _ => None,
        })
        .collect();
    json!({ "area": arred(area), "perimetro": arred(perimetro), "centro": par(c), "caixa": caixa_json(Some((min, max))), "vertices": pts.len(), "contorno": if pts.len() <= 24 { json!(pts.iter().map(|p| par(*p)).collect::<Vec<_>>()) } else { Value::Null }, "textos_dentro": textos })
}

fn dentro(laco: &[Ponto], p: Ponto) -> bool {
    let mut d = false;
    for i in 0..laco.len() {
        let (a, b) = (laco[i], laco[(i + 1) % laco.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            d = !d;
        }
    }
    d
}

/// As regiões fechadas que os traços formam. Com `ponto`, só a menor que o contém (o que uma hachura preencheria).
/// Sem ele, todas (da menor para a maior); `area_util` já desconta as regiões que ficam dentro (vãos).
pub fn regioes(d: &Documento, args: &Value) -> Res<Value> {
    if let Some(p) = args.get("ponto") {
        let p = ponto(p, "ponto")?;
        return Ok(match regiao_em(d, p) {
            Some(pts) => json!({ "encontrada": true, "regiao": descrever_regiao(d, feathercad_core::anotacao::area_e_perimetro(&pts).0, &pts) }),
            None => json!({ "encontrada": false, "nota": "o ponto não está numa área fechada (pode haver uma abertura no contorno; use cad_verificar)" }),
        });
    }
    let todas = todas_as_regioes(d);
    let limite = numero_opcional(args, "limite").map_or(60, |n| n.clamp(1.0, 200.0) as usize);
    let centros: Vec<Ponto> = todas.iter().map(|(_, pts)| centroide(pts)).collect();
    let lista: Vec<Value> = todas
        .iter()
        .enumerate()
        .take(limite)
        .map(|(i, (area, pts))| {
            let filhas: f64 = todas.iter().enumerate().filter(|(j, (a2, _))| *j != i && a2 < area && dentro(pts, centros[*j])).map(|(_, (a2, _))| a2).sum();
            let mut v = descrever_regiao(d, *area, pts);
            v["area_util"] = json!(arred((area - filhas).max(0.0)));
            v
        })
        .collect();
    Ok(json!({ "total": todas.len(), "retornadas": lista.len(), "regioes": lista }))
}

fn checklist_eletrica(d: &Documento) -> Vec<Value> {
    let mut itens = Vec::new();
    let tem_qdc = d.entidades().iter().any(|e| {
        if let feathercad_core::documento::Geometria::Texto { conteudo, .. } = &e.geometria {
            let u = conteudo.to_uppercase();
            u.contains("QDC") || u.contains("QUADRO")
        } else if let feathercad_core::documento::Geometria::Insercao { bloco, .. } = &e.geometria {
            let u = d.blocos().get(*bloco).map(|b| b.nome.to_uppercase()).unwrap_or_default();
            u.contains("QUADRO") || u.contains("QDC")
        } else {
            false
        }
    });
    itens.push(json!({
        "item": "Quadro de Distribuição (QDC)",
        "passou": tem_qdc,
        "detalhes": if tem_qdc { "Quadro de distribuição/QDC identificado no desenho." } else { "Falta incluir ao menos um Quadro de Distribuição (QDC)." }
    }));

    let tem_luz = d.entidades().iter().any(|e| {
        let cname = d.camadas().get(e.camada).map(|c| c.nome.to_uppercase()).unwrap_or_default();
        cname.contains("ELE") || cname.contains("LUZ") || matches!(e.geometria, feathercad_core::documento::Geometria::Circulo { .. })
    });
    itens.push(json!({
        "item": "Pontos de Iluminação",
        "passou": tem_luz,
        "detalhes": if tem_luz { "Pontos de iluminação presentes no desenho." } else { "Falta incluir pontos de iluminação na camada ELE_PONTOS." }
    }));

    let tem_eletrodutos = d.entidades().iter().any(|e| {
        let cname = d.camadas().get(e.camada).map(|c| c.nome.to_uppercase()).unwrap_or_default();
        cname.contains("ELETRODUTO") || cname.contains("ELE")
    });
    itens.push(json!({
        "item": "Traçado de Eletrodutos",
        "passou": tem_eletrodutos,
        "detalhes": if tem_eletrodutos { "Eletrodutos e tubulações identificadas." } else { "Falta traçado de eletrodutos interligando os pontos." }
    }));
    itens
}

fn checklist_arquitetura(d: &Documento) -> Vec<Value> {
    let mut itens = Vec::new();
    let tem_paredes = d.entidades().iter().any(|e| {
        let cname = d.camadas().get(e.camada).map(|c| c.nome.to_uppercase()).unwrap_or_default();
        cname.contains("PAREDE") || cname.contains("ARQ")
    });
    itens.push(json!({
        "item": "Alvenaria e Paredes",
        "passou": tem_paredes,
        "detalhes": if tem_paredes { "Camadas de alvenaria/paredes presentes." } else { "Falta alvenaria/paredes na camada ARQ_PAREDES." }
    }));

    let tem_niveis = d.entidades().iter().any(|e| {
        if let feathercad_core::documento::Geometria::Texto { conteudo, .. } = &e.geometria {
            let u = conteudo.to_uppercase();
            u.contains("N.P.") || u.contains("N.A.") || u.contains("Z:") || u.contains("NÍVEL")
        } else {
            false
        }
    });
    itens.push(json!({
        "item": "Indicação de Níveis (Z) e Cotas",
        "passou": tem_niveis,
        "detalhes": if tem_niveis { "Indicação de níveis Z / cotas identificada." } else { "Falta indicação de níveis Z (N.P. / N.A.) em COTAS_TEXTOS." }
    }));

    let tem_folha = d.folha_campos(0).is_ok() || d.entidades().iter().any(|e| {
        if let feathercad_core::documento::Geometria::Texto { conteudo, .. } = &e.geometria {
            let u = conteudo.to_uppercase();
            u.contains("PRANCHETA") || u.contains("FOLHA") || u.contains("ESCALA")
        } else {
            false
        }
    });
    itens.push(json!({
        "item": "Prancheta / Folha de Projeto",
        "passou": tem_folha,
        "detalhes": if tem_folha { "Folha/Prancheta de projeto identificada." } else { "Falta inserir ao menos uma Folha/Prancheta (cad_folha)." }
    }));
    itens
}

fn checklist_hidraulica(d: &Documento) -> Vec<Value> {
    let mut itens = Vec::new();
    let tem_caixa = d.entidades().iter().any(|e| {
        if let feathercad_core::documento::Geometria::Texto { conteudo, .. } = &e.geometria {
            let u = conteudo.to_uppercase();
            u.contains("RESERVATÓRIO") || u.contains("CAIXA") || u.contains("AF-1")
        } else {
            false
        }
    });
    itens.push(json!({
        "item": "Reservatório / Caixa d'Água",
        "passou": tem_caixa,
        "detalhes": if tem_caixa { "Reservatório/caixa d'água identificado." } else { "Falta incluir o reservatório de água fria." }
    }));

    let tem_esgoto = d.entidades().iter().any(|e| {
        let cname = d.camadas().get(e.camada).map(|c| c.nome.to_uppercase()).unwrap_or_default();
        cname.contains("ESGOTO") || cname.contains("HID")
    });
    itens.push(json!({
        "item": "Instalações de Esgoto e Caixas (CG/CI/CS)",
        "passou": tem_esgoto,
        "detalhes": if tem_esgoto { "Caixas e rede de esgoto presentes." } else { "Falta rede de esgoto e caixas sifonadas/gordura." }
    }));
    itens
}

/// Validação de checklist de requisitos mínimos por domínio (eletrica, arquitetura, hidraulica).
pub fn checklist_do_dominio(d: &Documento, dom: &str) -> Value {
    let dom_str = dom.to_lowercase();
    let itens = match dom_str.as_str() {
        "eletrica" | "elétrica" => checklist_eletrica(d),
        "arquitetura" => checklist_arquitetura(d),
        "hidraulica" | "hidráulica" => checklist_hidraulica(d),
        _ => {
            let tem_entidades = !d.entidades().is_empty();
            vec![json!({
                "item": "Entidades de Desenho",
                "passou": tem_entidades,
                "detalhes": if tem_entidades { "Desenho possui entidades." } else { "O desenho está em branco." }
            })]
        }
    };

    let tudo_ok = itens.iter().all(|i| i["passou"].as_bool().unwrap_or(false));
    json!({ "dominio": dom, "aprovado": tudo_ok, "checklist": itens })
}

/// Erros de desenho comuns: comprimento zero, duplicadas, extremidades que **quase** se tocam e checklist de domínio.
pub fn verificar(d: &Documento, args: &Value) -> Value {
    let diag = d.caixa().map_or(1.0, |(a, b)| a.distancia(b).max(1e-9));
    let tol = numero_opcional(args, "tolerancia").unwrap_or(diag * 0.002);
    let mut problemas: Vec<Value> = Vec::new();
    zeros_e_duplicadas(d, &mut problemas);
    if d.entidades().len() <= 3000 {
        folgas(d, tol, &mut problemas);
    }
    let mut res = json!({ "ok": problemas.is_empty(), "tolerancia_usada": arred(tol), "problemas": problemas, "nota": "folga = extremidade que fica perto de outra forma sem tocar; dentro da tolerância, provavelmente era para fechar" });
    if args.get("leiaute").and_then(Value::as_bool).unwrap_or(false) {
        res["leiaute"] = crate::leiaute::verificar_leiaute(d);
    }
    if let Some(dom) = texto_opcional(args, "dominio") {
        res["checklist_dominio"] = checklist_do_dominio(d, dom);
    }
    res
}

fn zeros_e_duplicadas(d: &Documento, fora: &mut Vec<Value>) {
    let es = d.entidades();
    for (i, e) in es.iter().enumerate() {
        let vazia = caixa_da_entidade(d, &e.geometria).is_some_and(|(a, b)| a.distancia(b) < 1e-9);
        if vazia && !matches!(e.geometria, feathercad_core::documento::Geometria::Ponto { .. } | feathercad_core::documento::Geometria::Texto { .. }) {
            fora.push(json!({ "tipo": "comprimento_zero", "entidade": e.id, "forma": tipo_de(&e.geometria) }));
        }
        if es[..i].iter().any(|o| o.geometria == e.geometria && o.camada == e.camada) {
            fora.push(json!({ "tipo": "duplicada", "entidade": e.id, "forma": tipo_de(&e.geometria) }));
        }
    }
}

/// Extremidades de linhas, arcos e polilinhas abertas que ficam a `0 < dist <= tol` de outra forma.
fn folgas(d: &Documento, tol: f64, fora: &mut Vec<Value>) {
    for e in d.entidades().iter().filter(|e| matches!(tipo_de(&e.geometria), "linha" | "arco" | "polilinha")) {
        let mut pontas: Vec<Ponto> = Vec::new();
        e.geometria.para_cada_ponto_notavel(&mut |p| pontas.push(p));
        for p in pontas {
            let mais_perto = d.entidades().iter().filter(|o| o.id != e.id).map(|o| (distancia(d, &o.geometria, p), o.id)).min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((dist, outro)) = mais_perto.filter(|(dist, _)| *dist > 1e-9 && *dist <= tol) {
                fora.push(json!({ "tipo": "folga", "entidade": e.id, "extremidade": par(p), "distancia": arred(dist), "perto_de": outro }));
            }
        }
    }
}
