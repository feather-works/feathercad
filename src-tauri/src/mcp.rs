//! O servidor MCP (`crates/feathercad-mcp`) visto pelas Configurações: onde está o programa e se ele responde.
//! O aplicativo **não** roda o servidor: quem o abre é o cliente de IA (Claude Code etc.), por stdio. Aqui só se
//! acha o binário, para mostrar o comando pronto, e se faz um teste de conversa (initialize + tools/list).

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::Serialize;

const NOME: &str = if cfg!(windows) { "feathercad-mcp.exe" } else { "feathercad-mcp" };

/// Onde procurar, na ordem: ao lado do aplicativo (instalado) e o `target/release` do repositório (desenvolvimento).
fn candidatos() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(pasta) = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from)) {
        v.push(pasta.join(NOME));
    }
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/feathercad-mcp/target/release").join(NOME));
    v
}

#[derive(Serialize)]
pub struct EstadoDoMcp {
    /// O caminho do programa (o que existe; senão o esperado no repositório).
    caminho: String,
    existe: bool,
}

fn achar() -> Option<PathBuf> {
    candidatos().into_iter().find(|c| c.is_file()).map(|c| c.canonicalize().unwrap_or(c))
}

/// Onde está o programa do MCP e se ele existe.
#[tauri::command]
pub fn mcp_estado() -> EstadoDoMcp {
    match achar() {
        Some(p) => EstadoDoMcp { caminho: p.display().to_string(), existe: true },
        None => EstadoDoMcp { caminho: candidatos().last().map(|p| p.display().to_string()).unwrap_or_default(), existe: false },
    }
}

#[derive(Serialize)]
pub struct ResultadoDoTeste {
    ok: bool,
    ferramentas: usize,
    mensagem: String,
}

fn falha(mensagem: impl Into<String>) -> ResultadoDoTeste {
    ResultadoDoTeste { ok: false, ferramentas: 0, mensagem: mensagem.into() }
}

/// Sobe o programa, faz `initialize` e `tools/list` e conta as ferramentas (até 5 s).
#[tauri::command]
pub fn mcp_testar() -> ResultadoDoTeste {
    let Some(programa) = achar() else { return falha("programa não encontrado") };
    let mut filho = match Command::new(&programa).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
        Ok(f) => f,
        Err(e) => return falha(format!("não consegui abrir o programa: {e}")),
    };
    let (Some(mut entrada), Some(saida)) = (filho.stdin.take(), filho.stdout.take()) else { return falha("sem stdin/stdout") };
    let pedidos = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"feathercad-config","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    ];
    for p in pedidos {
        if writeln!(entrada, "{p}").is_err() {
            let _ = filho.kill();
            return falha("o programa fechou sem responder");
        }
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut linhas = BufReader::new(saida).lines();
        let _ = tx.send((linhas.next(), linhas.next()));
    });
    let resposta = rx.recv_timeout(Duration::from_secs(5));
    let _ = filho.kill();
    let _ = filho.wait();
    let Ok((Some(Ok(a)), Some(Ok(b)))) = resposta else { return falha("o programa não respondeu a tempo") };
    let (Ok(init), Ok(lista)) = (serde_json::from_str::<serde_json::Value>(&a), serde_json::from_str::<serde_json::Value>(&b)) else { return falha("resposta que não é JSON") };
    let n = lista["result"]["tools"].as_array().map_or(0, Vec::len);
    let nome = init["result"]["serverInfo"]["name"].as_str().unwrap_or("?");
    if n == 0 {
        return falha("o programa respondeu, mas sem ferramentas");
    }
    ResultadoDoTeste { ok: true, ferramentas: n, mensagem: format!("{nome} respondeu ao initialize e listou {n} ferramentas") }
}
