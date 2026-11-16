//! `cad_eletroduto`: o eletroduto de um projeto elétrico com os **traços de condutor** da NBR 5444.
//!
//! Desenha a linha do eletroduto (contínua no teto/parede, tracejada no piso) e, num trecho dela, um traço
//! por condutor: **fase** = traço que cruza; **neutro** = traço que cruza com uma bandeirinha; **retorno** =
//! meio traço; **terra** = meio traço com um T. Em cima vão os números dos circuitos que passam ali, embaixo
//! a seção dos condutores (só quando não é 1,5 mm², como manda a norma) e, na bandeirinha, o diâmetro Ø.

use feathercad_core::documento::{Documento, Geometria, TipoLinha};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{lista_de_pontos, numero_opcional, texto_opcional, Res};
use crate::simbolos::{gravar, mm_de_papel, Peca};

#[derive(Clone, Copy, PartialEq)]
enum Tipo {
    Fase,
    Neutro,
    Retorno,
    Terra,
}

struct Condutor {
    circuito: Option<String>,
    tipo: Tipo,
    secao: f64,
}

fn tipo_de(nome: &str) -> Res<Tipo> {
    match nome.to_lowercase().as_str() {
        "fase" | "f" => Ok(Tipo::Fase),
        "neutro" | "n" => Ok(Tipo::Neutro),
        "retorno" | "r" => Ok(Tipo::Retorno),
        "terra" | "t" | "pe" => Ok(Tipo::Terra),
        outro => Err(format!("tipo de condutor `{outro}` não vale (fase, neutro, retorno ou terra)")),
    }
}

/// Área aproximada, com isolação, de um condutor de `secao` mm² (mm²).
fn area_com_isolacao(secao: f64) -> f64 {
    match secao {
        s if s <= 1.5 => 8.0,
        s if s <= 2.5 => 11.5,
        s if s <= 4.0 => 15.0,
        s if s <= 6.0 => 20.0,
        s if s <= 10.0 => 31.0,
        s if s <= 16.0 => 46.0,
        s if s <= 25.0 => 71.0,
        _ => 95.0,
    }
}

/// O menor diâmetro nominal (mm) em que os condutores ocupam no máximo 40 % (3 ou mais condutores).
fn diametro_automatico(cs: &[Condutor]) -> u32 {
    let total: f64 = cs.iter().map(|c| area_com_isolacao(c.secao)).sum();
    [(16, 49.0), (20, 85.0), (25, 138.0), (32, 246.0), (40, 375.0), (50, 590.0)].into_iter().find(|&(_, cap)| total <= cap).map_or(50, |(d, _)| d)
}

fn numero_curto(n: f64) -> String {
    if (n - n.round()).abs() < 1e-9 { format!("{}", n as i64) } else { format!("{n}").replace('.', ",") }
}

fn condutores_de(args: &Value) -> Res<Vec<Condutor>> {
    let lista = args.get("condutores").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`condutores` precisa listar ao menos um condutor: [{circuito, tipo, secao, quantidade?}]")?;
    let mut cs = Vec::new();
    for (i, c) in lista.iter().enumerate() {
        let tipo = tipo_de(texto_opcional(c, "tipo").ok_or_else(|| format!("condutor {i}: falta `tipo`"))?).map_err(|e| format!("condutor {i}: {e}"))?;
        let secao = numero_opcional(c, "secao").filter(|s| *s > 0.0).unwrap_or(1.5);
        let circuito = match c.get("circuito") {
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        };
        let q = numero_opcional(c, "quantidade").map_or(1, |n| n.clamp(1.0, 12.0) as usize);
        for _ in 0..q {
            cs.push(Condutor { circuito: circuito.clone(), tipo, secao });
        }
    }
    if args.get("terra").and_then(Value::as_bool).unwrap_or(true) && !cs.iter().any(|c| c.tipo == Tipo::Terra) {
        let s = numero_opcional(args, "secao_terra").unwrap_or_else(|| cs.iter().map(|c| c.secao).fold(0.0, f64::max));
        cs.push(Condutor { circuito: None, tipo: Tipo::Terra, secao: s });
    }
    Ok(cs)
}

/// O índice do trecho mais comprido da polilinha.
fn trecho_mais_longo(pts: &[Ponto]) -> usize {
    (0..pts.len() - 1).max_by(|&i, &j| pts[i].distancia(pts[i + 1]).total_cmp(&pts[j].distancia(pts[j + 1]))).unwrap_or(0)
}

/// Os traços dos condutores, centrados em `m`, num trecho de direção `u` (unitário). Devolve a largura do bloco (mm de papel).
fn tracos(peca: &mut Peca, cs: &[Condutor], m: (f64, f64), u: (f64, f64)) -> f64 {
    let n = (-u.1, u.0);
    // passo de 1,8 mm e vão extra entre circuitos: com 1,2 mm a bandeira do neutro e o T do terra encostavam na marca vizinha
    let (passo, vao, meio, tampa, bandeira) = (1.8, 1.0, 1.5, 0.6, 0.8);
    let ponto = |t: f64, k: f64| (m.0 + u.0 * t + n.0 * k, m.1 + u.1 * t + n.1 * k);
    let mut pos = Vec::with_capacity(cs.len());
    let mut t = 0.0;
    for (i, c) in cs.iter().enumerate() {
        if i > 0 {
            t += passo + if c.circuito != cs[i - 1].circuito { vao } else { 0.0 };
        }
        pos.push(t);
    }
    let largura = pos.last().copied().unwrap_or(0.0);
    for (i, c) in cs.iter().enumerate() {
        let t = pos[i] - largura / 2.0;
        match c.tipo {
            Tipo::Fase => peca.linha(ponto(t, -meio), ponto(t, meio)),
            Tipo::Neutro => {
                peca.linha(ponto(t, -meio), ponto(t, meio));
                peca.linha(ponto(t, meio), ponto(t - bandeira, meio));
            }
            Tipo::Retorno => peca.linha(ponto(t, 0.0), ponto(t, meio)),
            Tipo::Terra => {
                peca.linha(ponto(t, 0.0), ponto(t, meio));
                peca.linha(ponto(t - tampa, meio), ponto(t + tampa, meio));
            }
        }
    }
    largura + 2.0 * tampa.max(bandeira)
}

/// Os textos: circuitos em cima, seções embaixo (na horizontal; ao lado do bloco se o trecho é vertical).
fn rotulos(peca: &mut Peca, cs: &[Condutor], m: (f64, f64), horizontal: bool, altura: f64) -> f64 {
    let mut circuitos: Vec<&str> = Vec::new();
    for c in cs {
        if let Some(t) = c.circuito.as_deref() {
            if !circuitos.contains(&t) {
                circuitos.push(t);
            }
        }
    }
    let mut secoes: Vec<String> = Vec::new();
    for c in cs.iter().filter(|c| (c.secao - 1.5).abs() > 1e-9) {
        let t = format!("#{}", numero_curto(c.secao));
        if !secoes.contains(&t) {
            secoes.push(t);
        }
    }
    let (em_cima, embaixo) = (circuitos.join(" "), secoes.join(" "));
    let largura = |t: &str| t.chars().count() as f64 * altura * 0.6;
    let (x_cima, y_cima, x_baixo, y_baixo) = if horizontal {
        (m.0 - largura(&em_cima) / 2.0, m.1 + 2.0, m.0 - largura(&embaixo) / 2.0, m.1 - 2.0 - altura)
    } else {
        (m.0 + 2.2, m.1 + 0.3, m.0 + 2.2, m.1 - 0.3 - altura)
    };
    if !em_cima.is_empty() {
        peca.texto((x_cima, y_cima), altura, &em_cima);
    }
    if !embaixo.is_empty() {
        peca.texto((x_baixo, y_baixo), altura, &embaixo);
    }
    if horizontal { largura(&em_cima).max(largura(&embaixo)) } else { 0.0 }
}

/// A bandeirinha do eletroduto (NBR 5444, 5.1/5.2) com o diâmetro embaixo.
fn bandeirinha(peca: &mut Peca, q: (f64, f64), diametro: u32, altura: f64) {
    peca.linha(q, (q.0 + 2.0, q.1 - 2.0));
    peca.texto((q.0 + 2.3, q.1 - 2.0 - altura), altura, &format!("Ø{diametro}"));
}

/// Curva suave (Bézier quadrática, 16 trechos) de `a` a `b`; `k` é o quanto ela se afasta da reta, em frações do comprimento (+ à esquerda).
fn curvar(a: Ponto, b: Ponto, k: f64) -> Vec<Ponto> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let c = Ponto::novo((a.x + b.x) / 2.0 - dy * k, (a.y + b.y) / 2.0 + dx * k);
    (0..=16).map(|i| {
        let t = f64::from(i) / 16.0;
        let u = 1.0 - t;
        Ponto::novo(u * u * a.x + 2.0 * u * t * c.x + t * t * b.x, u * u * a.y + 2.0 * u * t * c.y + t * t * b.y)
    }).collect()
}

/// O trecho que contém a fração `f` do comprimento total da polilinha.
fn trecho_na_fracao(pts: &[Ponto], f: f64) -> usize {
    let total: f64 = pts.windows(2).map(|w| w[0].distancia(w[1])).sum();
    let mut acc = 0.0;
    for (i, w) in pts.windows(2).enumerate() {
        acc += w[0].distancia(w[1]);
        if acc >= total * f {
            return i;
        }
    }
    pts.len() - 2
}

/// Eletroduto com condutores. Coordenadas do desenho; o resto em milímetros de papel na `escala`.
pub fn eletroduto(doc: &mut Documento, args: &Value) -> Res<Value> {
    let mut pts = lista_de_pontos(args.get("pontos").ok_or("falta `pontos`")?, "pontos")?;
    if pts.len() < 2 {
        return Err("o eletroduto precisa de ao menos 2 pontos".into());
    }
    let curvo = numero_opcional(args, "curva").filter(|k| k.abs() > 1e-9 && pts.len() == 2);
    if let Some(k) = curvo {
        pts = curvar(pts[0], pts[1], k);
    }
    let cs = condutores_de(args)?;
    let s = mm_de_papel(doc, numero_opcional(args, "escala").filter(|e| *e > 0.0).unwrap_or(50.0))?;
    let piso = matches!(texto_opcional(args, "tipo"), Some("piso"));
    if let Some(t) = texto_opcional(args, "tipo") {
        if !matches!(t, "teto" | "parede" | "piso") {
            return Err(format!("`tipo` `{t}` não vale (teto, parede ou piso)"));
        }
    }
    let diametro = numero_opcional(args, "diametro").map_or_else(|| diametro_automatico(&cs), |d| d.max(1.0) as u32);
    let altura = numero_opcional(args, "altura_texto").filter(|h| *h > 0.0).unwrap_or(1.8);
    let rotulo = args.get("rotulo").and_then(Value::as_bool).unwrap_or(true);
    let pos = numero_opcional(args, "posicao").unwrap_or(0.5).clamp(0.05, 0.95);
    let i = if curvo.is_some() { trecho_na_fracao(&pts, pos) } else { trecho_mais_longo(&pts) };
    let (a, b) = (pts[i], pts[i + 1]);
    let comprimento = a.distancia(b);
    let u = ((b.x - a.x) / comprimento, (b.y - a.y) / comprimento);
    let ponto_em = |t: f64| (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);

    // Tudo o que é símbolo é montado em milímetros de papel, com a origem no ponto médio (`m`), e depois ampliado.
    let m = ponto_em(pos);
    let mut peca = Peca::default();
    let mut ocupado = tracos(&mut peca, &cs, (0.0, 0.0), u);
    let mut avisos: Vec<String> = Vec::new();
    if rotulo {
        ocupado = ocupado.max(rotulos(&mut peca, &cs, (0.0, 0.0), u.0.abs() >= u.1.abs(), altura));
        // o Ø vai ao lado do bloco de marcas, nunca em cima dele: do lado que tem espaço no trecho
        let bandeira_mm = 2.0 + feathercad_io::pdf::largura_do_conteudo(&format!("Ø{diametro}"), altura) + 0.3;
        let d = (ocupado / 2.0 + 1.5) * s / comprimento;
        let ponta = bandeira_mm * s / comprimento;
        let (frente, tras) = (pos + d, pos - d);
        let t_q = if frente + ponta <= 1.0 { Some(frente) } else if tras >= 0.0 { Some(tras) } else { None };
        let q = ponto_em(t_q.unwrap_or_else(|| (pos + d).min(1.0)));
        if t_q.is_none() {
            avisos.push(format!("o Ø não coube ao lado das marcas no trecho de {comprimento:.2}: ficou na ponta; use um trecho mais longo ou `posicao`"));
        }
        bandeirinha(&mut peca, ((q.0 - m.0) / s, (q.1 - m.1) / s), diametro, altura);
    }
    let precisa = ocupado * s;
    let livre = comprimento * pos.min(1.0 - pos) * 2.0;
    if precisa > livre {
        avisos.push(format!("as marcas ocupam {precisa:.2} e o trecho só tem {livre:.2} em volta da `posicao`: elas passam da ponta do trecho; use um trecho mais longo, outra `posicao` ou divida o eletroduto"));
    }
    let mut formas = crate::simbolos::colocar(&peca, s, 0.0, Ponto::novo(m.0, m.1));
    let linha = Geometria::Polilinha { vertices: pts.iter().map(|p| feathercad_core::geom::Vertice { p: *p, bulge: 0.0 }).collect(), fechada: false };
    formas.insert(0, (linha, piso.then_some(TipoLinha::Tracejada)));
    let espessura = numero_opcional(args, "espessura").filter(|e| *e > 0.0).map(|e| e as f32);
    let ids = gravar(doc, formas, args, espessura)?;
    let mut r = json!({ "eletroduto": ids.first(), "ids": ids, "diametro_mm": diametro, "condutores": cs.len(), "tipo": if piso { "piso" } else { "teto/parede" } });
    if !avisos.is_empty() {
        r["avisos"] = json!(avisos);
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::documento::Unidade;

    fn condutores(n: usize) -> Value {
        json!((1..=n).flat_map(|c| [json!({"tipo": "fase", "circuito": c, "secao": 2.5}), json!({"tipo": "neutro", "circuito": c, "secao": 2.5})]).collect::<Vec<_>>())
    }

    #[test]
    fn o_diametro_fica_fora_do_bloco_de_marcas() {
        let mut d = Documento::default().com_unidade(Some(Unidade::Metro));
        let r = eletroduto(&mut d, &json!({ "pontos": [[0.0, 0.0], [6.0, 0.0]], "condutores": condutores(4) })).unwrap();
        assert!(r.get("avisos").is_none(), "{r}");
        // o traço da bandeirinha é a única linha diagonal; o bloco de marcas fica em volta de x = 3
        let (txt, _) = crate::compacto::exportar(&d, None, false);
        let diagonal = txt.lines().filter(|l| l.starts_with("L ")).map(|l| l.split(' ').skip(1).map(|v| v.parse::<f64>().unwrap()).collect::<Vec<_>>()).find(|v| (v[0] - v[2]).abs() > 1e-6 && (v[1] - v[3]).abs() > 1e-6).expect("bandeirinha");
        let meia_largura = 0.05 * (2.0 * 4.0 * 1.8 + 3.0 * 1.0) / 2.0;
        assert!((diagonal[0] - 3.0).abs() > meia_largura, "Ø em cima das marcas: {diagonal:?}");
    }

    #[test]
    fn avisa_quando_as_marcas_nao_cabem_no_trecho() {
        let mut d = Documento::default().com_unidade(Some(Unidade::Metro));
        let r = eletroduto(&mut d, &json!({ "pontos": [[0.0, 0.0], [0.3, 0.0]], "condutores": condutores(6) })).unwrap();
        let avisos = r["avisos"].to_string();
        assert!(avisos.contains("não coube") || avisos.contains("passam da ponta"), "{r}");
    }
}
