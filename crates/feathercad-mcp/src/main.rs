//! `feathercad-mcp`: o servidor MCP do FeatherCAD. Lê uma mensagem JSON por linha no stdin e responde no
//! stdout; o que for diagnóstico vai para o stderr. Tudo local: nenhum dado sai da máquina.

mod agente;
mod ao_longo;
mod compacto;
mod componentes;
mod consulta;
mod esquemas;
mod expressao;
mod ferramentas;
mod folhas;
mod fonte;
mod imagem;
mod leiaute;
mod json;
mod lote;
mod medidas;
mod dados;
mod detalhe;
mod eletroduto;
mod exemplos;
mod guias;
mod grafico;
mod malha_norte;
mod modelos;
mod mutacoes;
mod nuvem_ferramentas;
mod pecas;
mod projecao;
mod tabela;
mod texto_bloco;
mod regras;
mod rpc;
mod simbolos;
mod topografia;
mod via;
mod viario;
mod viario_projeto;

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
