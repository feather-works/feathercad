//! O MCP **ao vivo**: o aplicativo escuta num socket local e executa as ferramentas do `feathercad-mcp` direto sobre o
//! desenho que está na tela. O Claude Code continua abrindo o `feathercad-mcp` por stdio; com o aplicativo aberto, esse
//! programa só repassa as chamadas para cá (`crates/feathercad-mcp/src/ao_vivo.rs`). Cada ferramenta é um passo de
//! Ctrl+Z, a interface se atualiza sozinha (`mcp_vivo_verificar`) e nada sai da máquina.
//!
//! O documento **não é copiado**: durante a ferramenta ele é trocado de lugar com o do `Agente` (que guarda o resto da
//! sessão: alinhamentos, nuvens, greides) e devolvido no fim, com a trava de escrita da `Ponte` presa — o mouse espera.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use feathercad_mcp::agente::Agente;
use feathercad_mcp::fonte::{self, Fonte};
use feathercad_mcp::{ao_vivo, rpc};
use feathercad_ponte::Ponte;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

/// A IA conta como "trabalhando" por tantos segundos depois da última ferramenta.
const ATIVA_POR_S: u64 = 4;

struct Vivo {
    agente: Mutex<Agente>,
    fonte: Fonte,
    /// Houve ferramenta que mexeu no desenho desde a última vez que a interface perguntou.
    mudou: AtomicBool,
    /// Instante (ms desde a época) da última ferramenta, 0 = nenhuma.
    ultima_ms: AtomicU64,
    /// Arquivos que outro FeatherCAD (ou a linha de comando) pediu para abrir; a interface os abre em abas.
    abrir: Mutex<Vec<String>>,
}

fn agora_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Uma ferramenta (`tools/call`) sobre o documento da tela.
fn chamar_ao_vivo(app: &AppHandle, vivo: &Vivo, msg: &Value) -> Option<Value> {
    let nome = msg.pointer("/params/name").and_then(Value::as_str).unwrap_or("").to_string();
    let inicio = Instant::now();
    let mut ag = vivo.agente.lock().unwrap_or_else(|e| e.into_inner());
    let estado = app.state::<RwLock<Ponte>>();
    let mut ponte = estado.write().unwrap_or_else(|e| e.into_inner());
    std::mem::swap(&mut ag.doc, ponte.documento_mut());
    let versao = ag.doc.versao();
    ag.tema_claro = ponte.tema_claro();
    ag.arquivo = ponte.caminho_do_arquivo().map(PathBuf::from);
    let antes = ag.arquivo.clone();
    let resposta = rpc::atender(&mut ag, &vivo.fonte, msg);
    let mudou = ag.doc.versao() != versao;
    std::mem::swap(&mut ag.doc, ponte.documento_mut());
    if mudou {
        ponte.depois_de_mudanca_externa();
    }
    drop(ponte);
    let depois = ag.arquivo.clone();
    drop(ag);
    crate::arquivos::depois_da_ia(app, antes.as_deref(), depois.as_deref(), nome == "cad_salvar");
    if mudou || antes != depois {
        vivo.mudou.store(true, Ordering::SeqCst);
    }
    vivo.ultima_ms.store(agora_ms(), Ordering::SeqCst);
    eprintln!("[mcp] {nome} {} ms{}", inicio.elapsed().as_millis(), if mudou { " (mudou o desenho)" } else { "" });
    resposta
}

/// Os arquivos da linha de comando (`feathercad desenho.dxf`, ou o duplo clique do sistema): só o que existe e não é opção.
pub fn arquivos_da_linha_de_comando() -> Vec<String> {
    std::env::args().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file()).collect()
}

/// Outro FeatherCAD foi aberto: este é o único. Traz a janela para a frente e enfileira os arquivos para a interface.
fn receber_instancia(app: &AppHandle, vivo: &Vivo, msg: &Value) -> Option<Value> {
    let arquivos: Vec<String> = msg.pointer("/params/arquivos").and_then(Value::as_array).into_iter().flatten().filter_map(|a| a.as_str().map(str::to_string)).collect();
    vivo.abrir.lock().unwrap_or_else(|e| e.into_inner()).extend(arquivos);
    if let Some(janela) = app.get_webview_window("main") {
        let _ = janela.unminimize();
        let _ = janela.show();
        let _ = janela.set_focus();
    }
    Some(json!({ "jsonrpc": "2.0", "id": msg.get("id").cloned().unwrap_or(Value::Null), "result": { "ok": true } }))
}

/// Sobe o canal local (uma vez, ao abrir o aplicativo).
pub fn iniciar(app: &AppHandle) {
    let vivo = std::sync::Arc::new(Vivo { agente: Mutex::new(Agente::default()), fonte: fonte::carregar(), mudou: AtomicBool::new(false), ultima_ms: AtomicU64::new(0), abrir: Mutex::new(arquivos_da_linha_de_comando()) });
    app.manage(vivo.clone());
    let app = app.clone();
    let tratar = move |msg: Value| -> Option<Value> {
        match msg.get("method").and_then(Value::as_str) {
            Some("tools/call") => return chamar_ao_vivo(&app, &vivo, &msg),
            Some(ao_vivo::METODO_INSTANCIA) => return receber_instancia(&app, &vivo, &msg),
            _ => {}
        }
        let mut ag = vivo.agente.lock().unwrap_or_else(|e| e.into_inner());
        rpc::atender(&mut ag, &vivo.fonte, &msg)
    };
    match ao_vivo::servir(tratar) {
        Ok(true) => eprintln!("[FeatherCAD] MCP ao vivo escutando em {}", ao_vivo::onde_escuta().display()),
        Ok(false) => eprintln!("[FeatherCAD] MCP ao vivo: já há outro FeatherCAD escutando; este não aceita a IA"),
        Err(e) => eprintln!("[FeatherCAD] MCP ao vivo indisponível: {e}"),
    }
}

/// Ao fechar o aplicativo.
pub fn encerrar() {
    ao_vivo::encerrar();
}

/// A interface pergunta de tempos em tempos: `{ativa, estado, abrir}`. `abrir` = arquivos que outro FeatherCAD entregou a este. `estado` só vem quando a IA mexeu no desenho (a
/// interface o aplica como aplica o de um arquivo recarregado); `ativa` = a IA chamou uma ferramenta há pouco.
#[tauri::command]
pub fn mcp_vivo_verificar(app: AppHandle) -> Result<Value, String> {
    let Some(vivo) = app.try_state::<std::sync::Arc<Vivo>>() else { return Ok(json!({ "ativa": false, "estado": null, "abrir": [] })) };
    let ativa = vivo.ultima_ms.load(Ordering::SeqCst).checked_add(ATIVA_POR_S * 1000).is_some_and(|f| f > agora_ms());
    let estado = if vivo.mudou.swap(false, Ordering::SeqCst) { Some(crate::arquivos::estado(&app)?) } else { None };
    let abrir = std::mem::take(&mut *vivo.abrir.lock().unwrap_or_else(|e| e.into_inner()));
    Ok(json!({ "ativa": ativa, "estado": estado, "abrir": abrir }))
}
