//! A casca do FeatherCAD: uma janela e o protocolo `ponte://`, que leva cada
//! comando do TypeScript até o motor Rust e devolve os bytes **na hora**
//! (síncrono do lado da página, como o `pasta://` do FeatherSheet).
//!
//! Aqui não há regra de desenho: a janela recebe o pedido, chama
//! `Ponte::chamar` e devolve a resposta. Abrir e gravar arquivo (`.fcdz`,
//! `.dxf`) entram com o `feathercad-io`.

mod arquivos;
mod memoria_arvore;
mod mcp;
mod metricas;

use std::sync::Mutex;
use std::time::Instant;

use feathercad_ponte::Ponte;
use tauri::http::{Request, Response};
use tauri::Manager;

/// Uma resposta com os cabeçalhos que a página precisa para ler o XHR.
fn resposta(status: u16, tipo: &str, corpo: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .header("Content-Type", tipo)
        .body(corpo)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

/// Atende um pedido do `ponte://`: um comando do motor, com a métrica `[motor]`.
fn atender(app: &tauri::AppHandle, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    if req.method() == "OPTIONS" {
        return resposta(200, "text/plain", Vec::new());
    }
    let estado = app.state::<Mutex<Ponte>>();
    let chegou = Instant::now();
    let mut ponte = estado.lock().unwrap_or_else(|envenenado| envenenado.into_inner());
    let espera = chegou.elapsed();
    let comando = req.uri().path().trim_start_matches('/');
    let inicio = Instant::now();
    let resultado = ponte.chamar(comando, req.body());
    drop(ponte);
    metricas::registrar(
        comando,
        espera,
        inicio.elapsed(),
        req.body().len(),
        resultado.as_ref().map(Vec::len).map_err(String::as_str),
    );
    match resultado {
        Ok(bytes) => resposta(200, "application/octet-stream", bytes),
        Err(erro) => resposta(400, "text/plain", erro.into_bytes()),
    }
}

/// Memória residente da árvore inteira do aplicativo (o principal e os
/// processos de WebView, onde a interface mora). Só no Linux.
#[tauri::command]
fn uso_memoria_arvore() -> Option<memoria_arvore::MemoriaArvore> {
    memoria_arvore::medir()
}

/// Recebe as mensagens do console da interface (`console.log`/`error`,
/// exceções) e imprime no terminal de quem abriu o aplicativo.
#[tauri::command]
fn log_mensagem(nivel: String, mensagem: String) {
    eprintln!("[FeatherCAD Log][{}] {}", nivel.to_uppercase(), mensagem);
}

/// A página Monitor (memória, console e métricas numa janela à parte) — a mesma
/// do FeatherSheet, aberta por F12.
#[tauri::command]
fn abrir_janela_monitor(app: tauri::AppHandle) -> Result<(), String> {
    abrir_pagina(&app, "monitor", "monitor.html", "Monitor do FeatherCAD", (1100.0, 720.0))
}

/// Abre (ou traz para a frente, se já aberta) uma janela com uma página do `dist/`.
fn abrir_pagina(app: &tauri::AppHandle, rotulo: &str, html: &str, titulo: &str, (w, h): (f64, f64)) -> Result<(), String> {
    if let Some(janela) = app.get_webview_window(rotulo) {
        let _ = janela.show();
        let _ = janela.unminimize();
        let _ = janela.set_focus();
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(app, rotulo, tauri::WebviewUrl::App(html.into()))
        .title(titulo)
        .inner_size(w, h)
        .min_inner_size(640.0, 420.0)
        .resizable(true)
        .focused(true)
        .build()
        .map_err(|e| format!("Não consegui abrir a janela \"{titulo}\": {e}"))?;
    Ok(())
}

/// Sobe a janela do FeatherCAD. O conteúdo é o mesmo `dist/index.html` que roda no navegador.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("[FeatherCAD] Aplicativo iniciado (PID: {}). Logs ativados no terminal.", std::process::id());
    let executado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(Ponte::default()))
        .manage(arquivos::VistoNoDisco::default())
        // Assíncrono: o WebKitGTK atende esquema próprio na thread da janela, e
        // esperar o lock ali congelaria a janela. A página continua esperando
        // (o XHR é síncrono), mas a janela não.
        .register_asynchronous_uri_scheme_protocol("ponte", |ctx, req, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(atender(&app, &req)));
        })
        .invoke_handler(tauri::generate_handler![
            uso_memoria_arvore,
            log_mensagem,
            abrir_janela_monitor,
            arquivos::desenho_abrir,
            arquivos::desenho_salvar,
            arquivos::desenho_verificar_disco,
            arquivos::desenho_exportar_pdf,
            arquivos::inicio_estado,
            arquivos::inicio_recente_remover,
            arquivos::inicio_projeto_adicionar,
            arquivos::inicio_projeto_remover,
            arquivos::inicio_pasta_listar,
            metricas::metricas_do_motor,
            mcp::mcp_estado,
            mcp::mcp_testar,
        ])
        .run(tauri::generate_context!());
    if let Err(erro) = executado {
        eprintln!("[FeatherCAD] Não consegui abrir a janela: {erro}");
    }
}
