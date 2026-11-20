//! Resultado grande vai para arquivo (`<desenho>.dados/<nome>.json`, ou a pasta temporária se o desenho não tem
//! arquivo): a resposta traz o resumo e o caminho, e as ferramentas seguintes aceitam o nome guardado na sessão
//! (`feicoes: "F1"`), então os números não passam pela conversa.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::Res;

/// Linhas que entram direto na resposta fora do modo projeto.
pub const LIMITE_DE_LINHAS: usize = 200;

fn pasta(ag: &Agente) -> PathBuf {
    match &ag.arquivo {
        Some(a) => {
            let mut p = a.clone().into_os_string();
            p.push(".dados");
            PathBuf::from(p)
        }
        None => std::env::temp_dir().join("feathercad_dados"),
    }
}

/// Grava `valor` em `<pasta>/<nome>.json` e devolve o caminho.
pub fn gravar(ag: &Agente, nome: &str, valor: &Value) -> Res<String> {
    let dir = pasta(ag);
    std::fs::create_dir_all(&dir).map_err(|e| format!("não consegui criar `{}`: {e}", dir.display()))?;
    // O nome vem do agente (`nome`, `nuvem`): barra e `..` não tiram o arquivo da pasta de dados.
    let nome: String = nome.chars().map(|c| if matches!(c, '/' | '\\' | ':') { '_' } else { c }).collect();
    let caminho = dir.join(format!("{}.json", nome.trim_start_matches('.')));
    std::fs::write(&caminho, serde_json::to_vec(valor).map_err(|e| e.to_string())?).map_err(|e| format!("não consegui gravar `{}`: {e}", caminho.display()))?;
    Ok(caminho.display().to_string())
}

/// Põe `linhas` no resumo: inline se são poucas e não é o modo projeto; senão em arquivo (com caminho e contagem).
/// `salvar: true` nos argumentos força o arquivo.
pub fn com_linhas(ag: &Agente, args: &Value, nome: &str, mut resumo: Value, linhas: Vec<Value>) -> Res<Value> {
    let forcar = args.get("salvar").and_then(Value::as_bool).unwrap_or(false);
    if ag.projeto || forcar || linhas.len() > LIMITE_DE_LINHAS {
        let caminho = gravar(ag, nome, &Value::Array(linhas.clone()))?;
        resumo["linhas_total"] = json!(linhas.len());
        resumo["arquivo"] = json!(caminho);
        resumo["aviso"] = json!(format!("{} linhas gravadas em `{}` (não entram na conversa); passe `{{\"de_arquivo\": \"{}\"}}` onde a ferramenta pedir a lista", linhas.len(), caminho, caminho));
        if !ag.projeto && !forcar {
            resumo["primeiras"] = json!(linhas.iter().take(5).collect::<Vec<_>>());
        }
    } else {
        resumo["linhas"] = Value::Array(linhas);
    }
    Ok(resumo)
}

/// Uma lista que pode vir inline ou como `{"de_arquivo": "caminho.json"}`.
pub fn ler_lista(v: &Value) -> Res<Vec<Value>> {
    if let Some(c) = v.get("de_arquivo").and_then(Value::as_str) {
        let t = std::fs::read_to_string(c).map_err(|e| format!("não consegui ler `{c}`: {e}"))?;
        return serde_json::from_str::<Vec<Value>>(&t).map_err(|e| format!("`{c}` não é uma lista JSON: {e}"));
    }
    v.as_array().cloned().ok_or_else(|| "esperava uma lista (ou {\"de_arquivo\": \"caminho\"})".into())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn poucas_linhas_ficam_inline_e_muitas_vao_para_arquivo_e_voltam() {
        let mut ag = Agente::default();
        let pequena = com_linhas(&ag, &json!({}), "t_pequena", json!({ "n": 3 }), vec![json!([1]), json!([2])]).unwrap();
        assert_eq!(pequena["linhas"].as_array().unwrap().len(), 2);
        let grande: Vec<Value> = (0..LIMITE_DE_LINHAS + 5).map(|i| json!([i])).collect();
        let r = com_linhas(&ag, &json!({}), "t_grande", json!({}), grande).unwrap();
        assert!(r.get("linhas").is_none() && r["linhas_total"] == LIMITE_DE_LINHAS + 5);
        let volta = ler_lista(&json!({ "de_arquivo": r["arquivo"] })).unwrap();
        assert_eq!(volta.len(), LIMITE_DE_LINHAS + 5);
        ag.projeto = true;
        assert!(com_linhas(&ag, &json!({}), "t_modo", json!({}), vec![json!(1)]).unwrap().get("arquivo").is_some(), "no modo projeto vai sempre para arquivo");
    }
}
