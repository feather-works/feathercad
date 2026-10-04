//! Regras normativas em tabela (JSON), não em código: o binário traz `ferrovia_metrica`; o usuário põe as suas em
//! `~/.config/feathercad/regras/<nome>.json` (ou `$FEATHERCAD_REGRAS/<nome>.json`), que valem sobre as de fábrica.
//! Numa chamada, `regras` é o nome de uma tabela ou um objeto com só os campos que mudam.

use feathercad_core::viario::superelevacao::Regras;
use serde_json::Value;

use crate::json::Res;

const FERROVIA_METRICA: &str = include_str!("../regras/ferrovia_metrica.json");

fn pasta_do_usuario() -> Option<std::path::PathBuf> {
    std::env::var_os("FEATHERCAD_REGRAS").map(Into::into).or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config/feathercad/regras")))
}

fn texto_da_tabela(nome: &str) -> Res<String> {
    if let Some(dir) = pasta_do_usuario() {
        if let Ok(t) = std::fs::read_to_string(dir.join(format!("{nome}.json"))) {
            return Ok(t);
        }
    }
    match nome {
        "ferrovia_metrica" => Ok(FERROVIA_METRICA.to_string()),
        outro => Err(format!("a tabela de regras `{outro}` não existe (de fábrica: ferrovia_metrica; as suas ficam em ~/.config/feathercad/regras/)")),
    }
}

/// `B`/`V` e `b`/`v` são o mesmo campo: tudo vira minúsculo antes de mesclar.
fn normalizar(v: &Value) -> serde_json::Map<String, Value> {
    v.as_object().map(|o| o.iter().map(|(k, x)| (if k == "B" || k == "V" { k.to_lowercase() } else { k.clone() }, x.clone())).collect()).unwrap_or_default()
}

/// As regras do pedido: `args.regras` (nome ou objeto parcial) sobre a tabela de fábrica; `V`/`velocidade` à parte vale por último.
pub fn carregar(args: &Value) -> Res<Regras> {
    let mut base = normalizar(&serde_json::from_str::<Value>(FERROVIA_METRICA).map_err(|e| e.to_string())?);
    match args.get("regras") {
        None | Some(Value::Null) => {}
        Some(Value::String(nome)) => base.extend(normalizar(&serde_json::from_str(&texto_da_tabela(nome)?).map_err(|e| format!("regras `{nome}`: {e}"))?)),
        Some(o @ Value::Object(_)) => base.extend(normalizar(o)),
        Some(_) => return Err("`regras` é o nome de uma tabela ou um objeto com os campos a mudar".into()),
    }
    if let Some(v) = args.get("V").or_else(|| args.get("velocidade")).and_then(Value::as_f64) {
        base.insert("v".into(), Value::from(v));
    }
    serde_json::from_value(Value::Object(base)).map_err(|e| format!("regras inválidas: {e}"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn o_json_de_fabrica_e_o_default_e_o_objeto_sobrepoe() {
        assert_eq!(carregar(&json!({})).unwrap(), Regras::default());
        let r = carregar(&json!({ "regras": { "h_max_mm": 120, "V": 80 } })).unwrap();
        assert_eq!((r.h_max_mm, r.v, r.b), (120.0, 80.0, 1.075));
        assert_eq!(carregar(&json!({ "V": 40 })).unwrap().v, 40.0);
        assert!(carregar(&json!({ "regras": "nao_existe" })).unwrap_err().contains("não existe"));
    }
}
