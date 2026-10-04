//! `feathercad-mcp`: o servidor MCP do FeatherCAD. Lê uma mensagem JSON por linha no stdin e responde no
//! stdout; o que for diagnóstico vai para o stderr. Tudo local: nenhum dado sai da máquina.

use std::io::{BufRead, Write};

use feathercad_mcp::{agente, ao_vivo, fonte, rpc};

fn main() {
    let mut agente = agente::Agente::default();
    let fonte = fonte::carregar();
    let mut vivo = ao_vivo::Cliente::default();
    let saida = std::io::stdout();
    for linha in std::io::stdin().lock().lines() {
        let Ok(linha) = linha else { break };
        if linha.trim().is_empty() {
            continue;
        }
        let resposta = match serde_json::from_str::<serde_json::Value>(&linha) {
            Ok(msg) => responder(&mut agente, &fonte, &mut vivo, &linha, &msg),
            Err(e) => Some(serde_json::json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON inválido: {e}") } })),
        };
        if let Some(r) = resposta {
            let mut s = saida.lock();
            let _ = writeln!(s, "{r}");
            let _ = s.flush();
        }
    }
}

/// Com o aplicativo aberto, as chamadas de ferramenta vão para ele (agem no desenho da tela); o resto, e tudo sem o
/// aplicativo, é atendido aqui.
fn responder(agente: &mut agente::Agente, fonte: &fonte::Fonte, vivo: &mut ao_vivo::Cliente, linha: &str, msg: &serde_json::Value) -> Option<serde_json::Value> {
    if msg.get("method").and_then(serde_json::Value::as_str) == Some("tools/call") {
        match vivo.repassar(linha) {
            Some(Ok(r)) => return serde_json::from_str(&r).ok(),
            Some(Err(e)) => {
                let id = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);
                return Some(serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": { "isError": true, "content": [{ "type": "text", "text": e }] } }));
            }
            None => {}
        }
    }
    rpc::atender(agente, fonte, msg)
}
