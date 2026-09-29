//! `feathercad-mcp`: o servidor MCP do FeatherCAD. Lê uma mensagem JSON por linha no stdin e responde no
//! stdout; o que for diagnóstico vai para o stderr. Tudo local: nenhum dado sai da máquina.

mod agente;
mod compacto;
mod consulta;
mod esquemas;
mod expressao;
mod ferramentas;
mod folhas;
mod fonte;
mod imagem;
mod json;
mod lote;
mod medidas;
mod eletroduto;
mod guias;
mod mutacoes;
mod rpc;
mod simbolos;

use std::io::{BufRead, Write};

fn main() {
    let mut agente = agente::Agente::default();
    let fonte = fonte::carregar();
    let saida = std::io::stdout();
    for linha in std::io::stdin().lock().lines() {
        let Ok(linha) = linha else { break };
        if linha.trim().is_empty() {
            continue;
        }
        let resposta = match serde_json::from_str::<serde_json::Value>(&linha) {
            Ok(msg) => rpc::atender(&mut agente, &fonte, &msg),
            Err(e) => Some(serde_json::json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON inválido: {e}") } })),
        };
        if let Some(r) = resposta {
            let mut s = saida.lock();
            let _ = writeln!(s, "{r}");
            let _ = s.flush();
        }
    }
}
