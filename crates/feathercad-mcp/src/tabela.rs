//! `cad_tabela`: grade de texto a partir de dados (quadro de curvas, de pontos, de desvios…), com quebra em
//! blocos lado a lado e texto que encolhe para caber na coluna.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, caixa_das, cor_de, fator_papel, formatar, relatorio, Pincel};

struct Coluna {
    titulo: String,
    largura: f64,
    alinhamento: AlinhaH,
    decimais: Option<usize>,
}

fn colunas(args: &Value) -> Res<Vec<Coluna>> {
    let lista = args.get("colunas").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`colunas` precisa de ao menos uma coluna {titulo, largura}")?;
    lista
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let largura = c.get("largura").and_then(Value::as_f64).filter(|l| *l > 0.0).ok_or_else(|| format!("coluna {i}: falta `largura` (maior que zero)"))?;
            let alinhamento = match c.get("alinhamento").and_then(Value::as_str) {
                None | Some("esquerda" | "e") => AlinhaH::Esquerda,
                Some("centro" | "c") => AlinhaH::Centro,
                Some("direita" | "d") => AlinhaH::Direita,
                Some(o) => return Err(format!("coluna {i}: alinhamento `{o}` (esquerda, centro, direita)")),
            };
            Ok(Coluna { titulo: c.get("titulo").and_then(Value::as_str).unwrap_or("").to_string(), largura, alinhamento, decimais: c.get("decimais").and_then(Value::as_u64).map(|d| d as usize) })
        })
        .collect()
}

fn celula(v: &Value, c: &Coluna, virgula: bool) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => match (n.as_f64(), c.decimais) {
            (Some(x), Some(d)) => formatar(x, d, virgula),
            (Some(x), None) if n.is_i64() || n.is_u64() => formatar(x, 0, virgula),
            (Some(x), None) => formatar(x, 2, virgula),
            _ => n.to_string(),
        },
        Value::Bool(b) => if *b { "sim" } else { "não" }.to_string(),
        Value::Null => String::new(),
        outro => outro.to_string(),
    }
}

pub fn tabela(doc: &mut Documento, args: &Value) -> Res<Value> {
    let origem = ponto_de(args, "origem")?;
    let k = fator_papel(doc, args)?;
    let cols = colunas(args)?;
    let linhas = args.get("linhas").and_then(Value::as_array).ok_or("falta `linhas` (lista de listas)")?;
    let hl = numero_opcional(args, "altura_linha").unwrap_or(6.0) * k;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(3.0) * k;
    if hl <= 0.0 || ht <= 0.0 {
        return Err("`altura_linha` e `altura_texto` precisam ser maiores que zero".into());
    }
    let blocos = args.get("blocos").and_then(Value::as_u64).unwrap_or(1).clamp(1, 20) as usize;
    let grade = texto_opcional(args, "grade").unwrap_or("todas");
    if !["todas", "5em5", "externa"].contains(&grade) {
        return Err("`grade`: todas, 5em5 ou externa".into());
    }
    let virgula = args.get("virgula").and_then(Value::as_bool).unwrap_or(false);
    let camada = texto_opcional(args, "camada").unwrap_or("TABELA");
    let cor = cor_de(args, "cor")?;
    let por_bloco = linhas.len().div_ceil(blocos).max(1);
    let blocos_usados = linhas.len().div_ceil(por_bloco).max(1);
    let wb: f64 = cols.iter().map(|c| c.largura * k).sum();
    let largura_total = wb * blocos_usados as f64;
    let titulo = texto_opcional(args, "titulo");
    let tem_titulo = titulo.is_some();
    let topo_cabecalho = origem.y - if tem_titulo { hl } else { 0.0 };
    let fundo = topo_cabecalho - hl * (1 + por_bloco) as f64;
    let margem = 0.4 * ht;

    let mut p = Pincel::novo(camada);
    p.cor = cor;
    p.retangulo(Ponto::novo(origem.x, fundo), Ponto::novo(origem.x + largura_total, origem.y));
    if let Some(t) = titulo {
        p.linha(Ponto::novo(origem.x, topo_cabecalho), Ponto::novo(origem.x + largura_total, topo_cabecalho));
        p.texto(Ponto::novo(origem.x + largura_total / 2.0, origem.y - hl / 2.0), ht * 1.2, 0.0, t, AlinhaH::Centro, AlinhaV::Meio);
    }
    let mut avisos: Vec<String> = Vec::new();
    let mut escreve = |p: &mut Pincel, texto: &str, x0: f64, largura: f64, y_meio: f64, al: AlinhaH, onde: &str| {
        let util = (largura - 2.0 * margem).max(1e-9);
        let natural = feathercad_io::pdf::largura_do_conteudo(texto, ht);
        let h = if natural > util { (ht * util / natural).max(ht * 0.55) } else { ht };
        if feathercad_io::pdf::largura_do_conteudo(texto, h) > util * 1.001 {
            avisos.push(format!("{onde}: `{texto}` não cabe nem com a letra reduzida"));
        }
        let x = match al {
            AlinhaH::Esquerda => x0 + margem,
            AlinhaH::Centro => x0 + largura / 2.0,
            AlinhaH::Direita => x0 + largura - margem,
        };
        p.texto(Ponto::novo(x, y_meio), h, 0.0, texto, al, AlinhaV::Meio);
    };
    for b in 0..blocos_usados {
        let x0 = origem.x + b as f64 * wb;
        // cabeçalho
        let mut x = x0;
        for (j, c) in cols.iter().enumerate() {
            let l = c.largura * k;
            escreve(&mut p, &c.titulo, x, l, topo_cabecalho - hl / 2.0, AlinhaH::Centro, &format!("cabeçalho {}", j + 1));
            if j > 0 {
                p.linha(Ponto::novo(x, topo_cabecalho), Ponto::novo(x, fundo));
            }
            x += l;
        }
        if b > 0 {
            p.linha(Ponto::novo(x0, topo_cabecalho), Ponto::novo(x0, fundo));
        }
        p.linha(Ponto::novo(x0, topo_cabecalho - hl), Ponto::novo(x0 + wb, topo_cabecalho - hl));
        let (ini, fim) = (b * por_bloco, ((b + 1) * por_bloco).min(linhas.len()));
        for (r, linha) in linhas[ini..fim].iter().enumerate() {
            let topo = topo_cabecalho - hl * (1 + r) as f64;
            let valores = linha.as_array().ok_or_else(|| format!("linha {}: precisa ser uma lista de valores", ini + r))?;
            let mut x = x0;
            for (j, c) in cols.iter().enumerate() {
                let l = c.largura * k;
                if let Some(v) = valores.get(j) {
                    let t = celula(v, c, virgula);
                    escreve(&mut p, &t, x, l, topo - hl / 2.0, c.alinhamento, &format!("linha {} coluna {}", ini + r + 1, j + 1));
                }
                x += l;
            }
            let ultima = r + 1 == por_bloco;
            let traco = match grade {
                "todas" => !ultima,
                "5em5" => (r + 1) % 5 == 0 && !ultima,
                _ => false,
            };
            if traco {
                p.linha(Ponto::novo(x0, topo - hl), Ponto::novo(x0 + wb, topo - hl));
            }
        }
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["linhas_por_bloco"] = json!(por_bloco);
    r["blocos"] = json!(blocos_usados);
    r["altura_total"] = json!(origem.y - fundo);
    r["largura_total"] = json!(largura_total);
    r["avisos"] = json!(avisos);
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn duzentas_linhas_em_quatro_blocos_cabem_na_caixa_e_alinham() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let linhas: Vec<Value> = (0..204).map(|i| json!([i, 12.5 + f64::from(i), "ok"])).collect();
        let r = tabela(&mut d, &json!({ "origem": [0, 0], "escala": 1000, "titulo": "Desvios", "altura_linha": 4, "altura_texto": 2,
            "colunas": [{ "titulo": "N", "largura": 10 }, { "titulo": "Valor", "largura": 20, "alinhamento": "direita", "decimais": 1 }, { "titulo": "Sit.", "largura": 12, "alinhamento": "centro" }],
            "linhas": linhas, "blocos": 4 })).unwrap();
        assert_eq!(r["linhas_por_bloco"], 51);
        assert_eq!(r["blocos"], 4);
        // 4 blocos de 42 mm de papel × 1000 m/mm... (escala 1:1000 em metros: 1 mm de papel = 1 m)
        assert!((r["largura_total"].as_f64().unwrap() - 168.0).abs() < 1e-6, "{r}");
        assert!((r["altura_total"].as_f64().unwrap() - 4.0 * 53.0).abs() < 1e-6, "{r}");
        assert!(r["avisos"].as_array().unwrap().is_empty(), "{r}");
    }

    #[test]
    fn texto_que_nao_cabe_encolhe_e_avisa_quando_nao_basta() {
        let mut d = Documento::default();
        let r = tabela(&mut d, &json!({ "origem": [0, 0], "colunas": [{ "titulo": "A", "largura": 10 }], "linhas": [["texto muito comprido para a coluna"]] })).unwrap();
        assert_eq!(r["avisos"].as_array().unwrap().len(), 1);
    }
}
