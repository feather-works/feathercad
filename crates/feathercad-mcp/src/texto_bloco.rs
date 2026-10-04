//! `cad_texto_bloco`: parágrafos com quebra automática pela largura (memorial, notas), em uma ou mais colunas.
//! Devolve onde terminou (`y_final`) para encadear o próximo bloco.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{numero_de, numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, caixa_das, cor_de, fator_papel, relatorio, Pincel};

/// Quebra `texto` em linhas de no máximo `largura` (palavras inteiras; `\n` força quebra).
pub fn quebrar(texto: &str, altura: f64, largura: f64) -> Vec<String> {
    let mut linhas = Vec::new();
    for paragrafo in texto.split('\n') {
        let mut atual = String::new();
        for palavra in paragrafo.split_whitespace() {
            let tentativa = if atual.is_empty() { palavra.to_string() } else { format!("{atual} {palavra}") };
            if !atual.is_empty() && feathercad_io::pdf::largura_do_conteudo(&tentativa, altura) > largura {
                linhas.push(std::mem::take(&mut atual));
                atual = palavra.to_string();
            } else {
                atual = tentativa;
            }
        }
        linhas.push(atual);
    }
    linhas
}

pub fn texto_bloco(doc: &mut Documento, args: &Value) -> Res<Value> {
    let origem = ponto_de(args, "origem")?;
    let k = fator_papel(doc, args)?;
    let largura = numero_de(args, "largura")? * k;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(3.0) * k;
    let entrelinha = numero_opcional(args, "entrelinha").unwrap_or(1.4);
    let colunas = args.get("colunas").and_then(Value::as_u64).unwrap_or(1).clamp(1, 6) as usize;
    let altura_max = numero_opcional(args, "altura_max").map(|a| a * k);
    if colunas > 1 && altura_max.is_none() {
        return Err("com `colunas` > 1 diga a `altura_max` (onde o texto passa para a coluna seguinte)".into());
    }
    if largura <= 0.0 || ht <= 0.0 {
        return Err("`largura` e `altura_texto` precisam ser maiores que zero".into());
    }
    let calha = 0.04 * largura;
    let larg_col = (largura - calha * (colunas - 1) as f64) / colunas as f64;
    let itens = args.get("itens").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`itens` precisa ser uma lista de {tipo, texto}")?;
    let camada = texto_opcional(args, "camada").unwrap_or("TEXTO");
    let mut p = Pincel::novo(camada);
    p.cor = cor_de(args, "cor")?;

    let (mut col, mut y, mut linhas_total) = (0usize, origem.y, 0usize);
    let (mut y_min, mut estourou) = (origem.y, false);
    for (i, item) in itens.iter().enumerate() {
        let tipo = item.get("tipo").and_then(Value::as_str).unwrap_or("texto");
        let texto = item.get("texto").and_then(Value::as_str).ok_or_else(|| format!("item {i}: falta `texto`"))?;
        let (altura, recuo, prefixo) = match tipo {
            "titulo" => (ht * 1.3, 0.0, ""),
            "texto" => (ht, 0.0, ""),
            "formula" => (ht, 1.5 * ht, ""),
            "nota" => (ht * 0.85, 0.0, ""),
            "lista" => (ht, 1.2 * ht, ""),
            outro => return Err(format!("item {i}: tipo `{outro}` (titulo, texto, formula, nota, lista)")),
        };
        let mut linhas = quebrar(&format!("{prefixo}{texto}"), altura, larg_col - recuo);
        if tipo == "titulo" && y < origem.y {
            y -= 0.6 * ht;
        }
        for (n, l) in linhas.drain(..).enumerate() {
            let passo = altura * entrelinha;
            if let Some(max) = altura_max {
                if y - passo < origem.y - max && col + 1 < colunas {
                    col += 1;
                    y = origem.y;
                } else if y - passo < origem.y - max - 1e-9 {
                    estourou = true;
                }
            }
            y -= passo;
            let x = origem.x + col as f64 * (larg_col + calha);
            if tipo == "lista" && n == 0 {
                p.texto(Ponto::novo(x, y), altura, 0.0, "•", AlinhaH::Esquerda, AlinhaV::Base);
            }
            p.texto(Ponto::novo(x + recuo, y), altura, 0.0, &l, AlinhaH::Esquerda, AlinhaV::Base);
            linhas_total += 1;
            y_min = y_min.min(y);
        }
        y -= 0.5 * ht;
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["y_final"] = json!(y);
    r["linhas"] = json!(linhas_total);
    r["colunas_usadas"] = json!(col + 1);
    r["estourou_altura_max"] = json!(estourou);
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn quebra_pela_largura_e_encadeia_o_y() {
        let l = quebrar("uma frase razoavelmente longa para quebrar em varias linhas curtas", 3.0, 40.0);
        assert!(l.len() >= 3, "{l:?}");
        assert!(l.iter().all(|s| feathercad_io::pdf::largura_do_conteudo(s, 3.0) <= 40.0 + 1e-9));
        let mut d = Documento::default();
        let r = texto_bloco(&mut d, &json!({ "origem": [0, 100], "largura": 60, "itens": [{ "tipo": "titulo", "texto": "Memorial" }, { "tipo": "texto", "texto": "A curva C6 tem raio de 350 m e transição de 70 m entre tangente e circular." }] })).unwrap();
        assert!(r["y_final"].as_f64().unwrap() < 100.0 - 10.0, "{r}");
        assert_eq!(r["estourou_altura_max"], false);
    }

    #[test]
    fn passa_para_a_proxima_coluna() {
        let mut d = Documento::default();
        let r = texto_bloco(&mut d, &json!({ "origem": [0, 0], "largura": 100, "colunas": 2, "altura_max": 9, "itens": [{ "texto": "a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j k l m n o p q r s t u v w x y z" }] })).unwrap();
        assert_eq!(r["colunas_usadas"], 2, "{r}");
    }
}
