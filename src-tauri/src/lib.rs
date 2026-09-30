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

use std::sync::RwLock;
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

/// Um comando do motor, com a métrica `[motor]`. Os de **leitura** (`Ponte::chamar_leitura`: quadro, cursor, vista)
/// rodam com a trava de leitura, ao mesmo tempo; o resto pega a trava de escrita.
fn executar(app: &tauri::AppHandle, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
    let estado = app.state::<RwLock<Ponte>>();
    let chegou = Instant::now();
    {
        let leitura = estado.read().unwrap_or_else(|envenenado| envenenado.into_inner());
        let espera = chegou.elapsed();
        let inicio = Instant::now();
        if let Some(resultado) = leitura.chamar_leitura(comando, corpo) {
            drop(leitura);
            metricas::registrar(comando, espera, inicio.elapsed(), corpo.len(), resultado.as_ref().map(Vec::len).map_err(String::as_str));
            return resultado;
        }
    }
    let chegou = Instant::now();
    let mut ponte = estado.write().unwrap_or_else(|envenenado| envenenado.into_inner());
    let espera = chegou.elapsed();
    let inicio = Instant::now();
    let resultado = ponte.chamar(comando, corpo);
    drop(ponte);
    metricas::registrar(comando, espera, inicio.elapsed(), corpo.len(), resultado.as_ref().map(Vec::len).map_err(String::as_str));
    resultado
}

/// Atende um pedido do `ponte://` (XHR síncrono da página): um comando do motor.
fn atender(app: &tauri::AppHandle, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    if req.method() == "OPTIONS" {
        return resposta(200, "text/plain", Vec::new());
    }
    let comando = req.uri().path().trim_start_matches('/');
    match executar(app, comando, req.body()) {
        Ok(bytes) => resposta(200, "application/octet-stream", bytes),
        Err(erro) => resposta(400, "text/plain", erro.into_bytes()),
    }
}

/// Um comando do motor de **resposta grande** (o quadro do desenho, alguns MB), pelo `invoke` do Tauri: assíncrono,
/// em outra thread (a janela não trava enquanto o Rust calcula) e com o corpo **cru**. Devolve `ipc::Response`, não
/// `Vec<u8>`: o `Vec<u8>` iria como JSON (um número decimal por byte), e o `Response` chega ao JavaScript como
/// `ArrayBuffer` — o mesmo padrão do `ler_arquivo` do FeatherSheet. O XHR síncrono do `ponte://` entregava o quadro
/// como texto, e converter 10 MB assim custava ~100 ms (medido no zoom com um mapa pesado).
#[tauri::command]
async fn ponte_binaria(app: tauri::AppHandle, comando: String, corpo: String) -> Result<tauri::ipc::Response, String> {
    tauri::async_runtime::spawn_blocking(move || executar(&app, &comando, corpo.as_bytes()).map(tauri::ipc::Response::new))
        .await
        .map_err(|e| format!("o comando {e}"))?
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

/// Pede ao WebKitGTK para compor e desenhar o canvas na **placa de vídeo, sempre**. O padrão (`ON_DEMAND`) só usa a
/// GPU quando a página pede animação 3D ou WebGL; um Canvas 2D comum ficava no processador (usuário 30/09/2026:
/// a placa nem se mexia durante o zoom). O resultado sai no terminal: `[FeatherCAD] aceleração …`.
#[cfg(target_os = "linux")]
fn ligar_aceleracao_por_gpu(app: &tauri::App) {
    use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
    let Some(janela) = app.get_webview_window("main") else {
        eprintln!("[FeatherCAD] aceleração por GPU: janela principal não encontrada");
        return;
    };
    let pedido = janela.with_webview(|webview| {
        let Some(ajustes) = webview.inner().settings() else { return };
        let antes = ajustes.hardware_acceleration_policy();
        ajustes.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);
        eprintln!("[FeatherCAD] aceleração por GPU: {:?} → {:?}", antes, ajustes.hardware_acceleration_policy());
    });
    if let Err(erro) = pedido {
        eprintln!("[FeatherCAD] aceleração por GPU: não consegui acessar a webview: {erro}");
    }
}

/// Sobe a janela do FeatherCAD. O conteúdo é o mesmo `dist/index.html` que roda no navegador.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("[FeatherCAD] Aplicativo iniciado (PID: {}). Logs ativados no terminal.", std::process::id());
    let executado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(RwLock::new(Ponte::default()))
        .manage(arquivos::VistoNoDisco::default())
        // Assíncrono: o WebKitGTK atende esquema próprio na thread da janela, e
        // esperar o lock ali congelaria a janela. A página continua esperando
        // (o XHR é síncrono), mas a janela não.
        .register_asynchronous_uri_scheme_protocol("ponte", |ctx, req, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(atender(&app, &req)));
        })
        .setup(|app| {
            #[cfg(target_os = "linux")]
            ligar_aceleracao_por_gpu(app);
            #[cfg(not(target_os = "linux"))]
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            uso_memoria_arvore,
            ponte_binaria,
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
            arquivos::inicio_miniatura,
            metricas::metricas_do_motor,
            mcp::mcp_estado,
            mcp::mcp_testar,
        ])
        .run(tauri::generate_context!());
    if let Err(erro) = executado {
        eprintln!("[FeatherCAD] Não consegui abrir a janela: {erro}");
    }
}
