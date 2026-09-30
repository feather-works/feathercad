//! Abrir e salvar desenhos, e a página inicial (recentes e pastas de projeto).
//!
//! Aqui só há disco e caixas de diálogo: a leitura/gravação do DXF e do `.fcdz`
//! está no `feathercad-io`, o documento mora na ponte, e a lista de recentes e
//! projetos é do `feathercad_io::inicio`. Todo comando devolve o **estado novo**
//! do motor (ou `None` se o usuário cancelou a caixa).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use feathercad_io::inicio::{self, ItemDeArquivo, Preferencias};
use feathercad_io::Formato;
use feathercad_ponte::Ponte;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// A data de modificação que o arquivo aberto tinha da última vez que o app o leu ou gravou.
/// Se mudar por fora (o servidor MCP, outro programa), o desenho é recarregado.
#[derive(Default)]
pub struct VistoNoDisco(Mutex<Option<SystemTime>>);

impl VistoNoDisco {
    fn definir(&self, quando: Option<SystemTime>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = quando;
    }
    fn trocar(&self, quando: Option<SystemTime>) -> Option<SystemTime> {
        std::mem::replace(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()), quando)
    }
}

fn data_do_arquivo(caminho: &Path) -> Option<SystemTime> {
    std::fs::metadata(caminho).and_then(|m| m.modified()).ok()
}

/// Onde ficam as preferências da página inicial.
fn arquivo_de_preferencias(app: &AppHandle) -> Result<PathBuf, String> {
    let pasta = app.path().app_data_dir().map_err(|e| format!("Não achei a pasta de dados do aplicativo: {e}"))?;
    Ok(pasta.join("inicio.json"))
}

/// A pasta padrão dos desenhos: `Documentos/FeatherCAD`. É criada se não existir; se não
/// der para criar (ou o sistema não tem Documentos), as caixas abrem onde o sistema quiser.
fn pasta_padrao(app: &AppHandle) -> Option<PathBuf> {
    let pasta = app.path().document_dir().ok()?.join("FeatherCAD");
    std::fs::create_dir_all(&pasta).ok()?;
    Some(pasta)
}

/// Onde a caixa de diálogo deve abrir: a pasta do arquivo atual, se houver, senão a padrão.
fn pasta_inicial(app: &AppHandle, atual: Option<&Path>) -> Option<PathBuf> {
    atual.and_then(Path::parent).filter(|p| p.is_dir()).map(Path::to_path_buf).or_else(|| pasta_padrao(app))
}

fn agora_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Um comando da ponte, com o lock.
fn ponte(app: &AppHandle, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
    let estado = app.state::<RwLock<Ponte>>();
    let mut p = estado.write().unwrap_or_else(|e| e.into_inner());
    p.chamar(comando, corpo)
}

fn estado(app: &AppHandle) -> Result<Value, String> {
    let bytes = ponte(app, "estado", b"")?;
    serde_json::from_slice(&bytes).map_err(|e| format!("estado ilegível: {e}"))
}

fn lembrar(app: &AppHandle, caminho: &Path) -> Result<(), String> {
    let arq = arquivo_de_preferencias(app)?;
    let mut p = Preferencias::carregar(&arq);
    p.adicionar_recente(&caminho.to_string_lossy(), agora_ms());
    p.gravar(&arq)
}

/// O formato de um caminho pela extensão.
fn formato_do_caminho(caminho: &Path) -> Option<Formato> {
    caminho.extension().and_then(|e| e.to_str()).and_then(Formato::da_extensao)
}

/// Abre um desenho: o do `caminho` (recentes, projetos) ou, sem ele, o que o
/// usuário escolher na caixa. `None` se cancelou.
#[tauri::command]
pub async fn desenho_abrir(app: AppHandle, caminho: Option<String>) -> Result<Option<Value>, String> {
    let caminho = match caminho {
        Some(c) => PathBuf::from(c),
        None => {
            let mut caixa = app.dialog().file();
            if let Some(pasta) = pasta_inicial(&app, None) {
                caixa = caixa.set_directory(pasta);
            }
            let escolhido = caixa
                .add_filter("Desenhos (DXF, FCDZ)", &["dxf", "fcdz"])
                .add_filter("DXF", &["dxf"])
                .add_filter("FeatherCAD (.fcdz)", &["fcdz"])
                .blocking_pick_file();
            let Some(f) = escolhido else { return Ok(None) };
            f.into_path().map_err(|e| format!("Caminho inválido: {e}"))?
        }
    };
    let formato = formato_do_caminho(&caminho).ok_or_else(|| format!("\"{}\" não é um desenho .dxf ou .fcdz.", caminho.display()))?;
    let bytes = std::fs::read(&caminho).map_err(|e| format!("Não consegui ler \"{}\": {e}", caminho.display()))?;
    ponte(&app, "documento_abrir", &bytes)?;
    let corpo = json!({ "caminho": caminho.to_string_lossy(), "formato": formato.extensao(), "salvo": false });
    ponte(&app, "documento_arquivo_definir", corpo.to_string().as_bytes())?;
    app.state::<VistoNoDisco>().definir(data_do_arquivo(&caminho));
    lembrar(&app, &caminho)?;
    estado(&app).map(Some)
}

/// Chamado de tempos em tempos pela interface: o arquivo aberto mudou no disco? Se mudou e o
/// desenho não tem alteração pendente, é recarregado (`{"recarregado": estado}`); se tem, nada
/// é perdido e a interface só é avisada, uma vez (`{"conflito": nome}`). `None` = nada a fazer.
#[tauri::command]
pub fn desenho_verificar_disco(app: AppHandle) -> Result<Option<Value>, String> {
    let atual: Value = serde_json::from_slice(&ponte(&app, "documento_arquivo", b"")?).map_err(|e| e.to_string())?;
    let Some(caminho) = atual.get("caminho").and_then(Value::as_str).map(PathBuf::from) else { return Ok(None) };
    let Some(agora) = data_do_arquivo(&caminho) else { return Ok(None) };
    if app.state::<VistoNoDisco>().trocar(Some(agora)) == Some(agora) {
        return Ok(None);
    }
    let sujo = estado(&app)?.get("sujo").and_then(Value::as_bool).unwrap_or(false);
    if sujo {
        let nome = atual.get("nome").and_then(Value::as_str).unwrap_or_default();
        return Ok(Some(json!({ "conflito": nome })));
    }
    let bytes = std::fs::read(&caminho).map_err(|e| format!("Não consegui ler \"{}\": {e}", caminho.display()))?;
    ponte(&app, "documento_abrir", &bytes)?;
    ponte(&app, "documento_arquivo_definir", atual_como_corpo(&atual).as_bytes())?;
    Ok(Some(json!({ "recarregado": estado(&app)? })))
}

/// O mesmo arquivo de antes, como pedido de `documento_arquivo_definir` (documento limpo).
fn atual_como_corpo(atual: &Value) -> String {
    json!({ "caminho": atual["caminho"], "formato": atual["formato"], "salvo": false }).to_string()
}

/// Onde e em que formato gravar. Com `como` (ou sem arquivo ainda) abre a caixa
/// "Salvar como", com o `formato_padrao` na frente; sem `como`, grava por cima.
fn destino(app: &AppHandle, como: bool, padrao: Formato) -> Result<Option<(PathBuf, Formato)>, String> {
    let atual: Value = serde_json::from_slice(&ponte(app, "documento_arquivo", b"")?).map_err(|e| e.to_string())?;
    if let (false, Some(c)) = (como, atual.get("caminho").and_then(Value::as_str)) {
        let caminho = PathBuf::from(c);
        let formato = formato_do_caminho(&caminho).unwrap_or(padrao);
        return Ok(Some((caminho, formato)));
    }
    let mut caixa = app.dialog().file();
    let arquivo_atual = atual.get("caminho").and_then(Value::as_str).map(PathBuf::from);
    if let Some(pasta) = pasta_inicial(app, arquivo_atual.as_deref()) {
        caixa = caixa.set_directory(pasta);
    }
    let filtros = [(Formato::Dxf, "DXF"), (Formato::Fcdz, "FeatherCAD (.fcdz)")];
    for (f, nome) in filtros.iter().filter(|(f, _)| *f == padrao).chain(filtros.iter().filter(|(f, _)| *f != padrao)) {
        caixa = caixa.add_filter(*nome, &[f.extensao()]);
    }
    let nome = atual.get("nome").and_then(Value::as_str).map_or_else(|| format!("desenho.{}", padrao.extensao()), str::to_string);
    let Some(f) = caixa.set_file_name(nome).blocking_save_file() else { return Ok(None) };
    let mut caminho = f.into_path().map_err(|e| format!("Caminho inválido: {e}"))?;
    let formato = formato_do_caminho(&caminho).unwrap_or(padrao);
    if formato_do_caminho(&caminho).is_none() {
        caminho.set_extension(padrao.extensao());
    }
    Ok(Some((caminho, formato)))
}

/// Salva o desenho. `formato_padrao` (`dxf` ou `fcdz`) é a preferência do usuário
/// para arquivo novo. `None` se cancelou a caixa.
#[tauri::command]
pub async fn desenho_salvar(app: AppHandle, como: bool, formato_padrao: String) -> Result<Option<Value>, String> {
    let padrao = Formato::da_extensao(&formato_padrao).unwrap_or(Formato::Dxf);
    let Some((caminho, formato)) = destino(&app, como, padrao)? else { return Ok(None) };
    let bytes = ponte(&app, "documento_gravar", json!({ "formato": formato.extensao() }).to_string().as_bytes())?;
    // Grava num `.parcial` e renomeia: falha no meio não estraga o arquivo que já existia.
    let parcial = caminho.with_extension("parcial");
    std::fs::write(&parcial, bytes).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))?;
    std::fs::rename(&parcial, &caminho).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))?;
    let corpo = json!({ "caminho": caminho.to_string_lossy(), "formato": formato.extensao(), "salvo": true });
    ponte(&app, "documento_arquivo_definir", corpo.to_string().as_bytes())?;
    app.state::<VistoNoDisco>().definir(data_do_arquivo(&caminho));
    lembrar(&app, &caminho)?;
    estado(&app).map(Some)
}

/// Exporta o desenho em PDF (uma página por folha) num arquivo que o usuário escolhe. `None` se cancelou.
#[tauri::command]
pub async fn desenho_exportar_pdf(app: AppHandle, opcoes: Value) -> Result<Option<Value>, String> {
    let bytes = ponte(&app, "documento_exportar_pdf", opcoes.to_string().as_bytes())?;
    let atual: Value = serde_json::from_slice(&ponte(&app, "documento_arquivo", b"")?).map_err(|e| e.to_string())?;
    let arquivo_atual = atual.get("caminho").and_then(Value::as_str).map(PathBuf::from);
    let nome = arquivo_atual.as_deref().and_then(Path::file_stem).map_or_else(|| "desenho".to_string(), |n| n.to_string_lossy().into_owned());
    let mut caixa = app.dialog().file().set_file_name(format!("{nome}.pdf")).add_filter("PDF", &["pdf"]);
    if let Some(pasta) = pasta_inicial(&app, arquivo_atual.as_deref()) {
        caixa = caixa.set_directory(pasta);
    }
    let Some(escolhido) = caixa.blocking_save_file() else { return Ok(None) };
    let mut caminho = escolhido.into_path().map_err(|e| format!("Caminho inválido: {e}"))?;
    if caminho.extension().is_none() {
        caminho.set_extension("pdf");
    }
    std::fs::write(&caminho, &bytes).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))?;
    Ok(Some(json!({ "caminho": caminho.to_string_lossy(), "bytes": bytes.len() })))
}

/// Um recente já descrito, para a página inicial.
#[derive(Serialize)]
pub struct RecenteDescrito {
    #[serde(flatten)]
    item: ItemDeArquivo,
    aberto_em_ms: u64,
}

/// O que a página inicial mostra: recentes (só os que ainda existem) e pastas de projeto.
#[derive(Serialize)]
pub struct EstadoInicio {
    recentes: Vec<RecenteDescrito>,
    projetos: Vec<String>,
}

fn estado_inicio(app: &AppHandle) -> Result<EstadoInicio, String> {
    let arq = arquivo_de_preferencias(app)?;
    let primeiro_uso = !arq.exists();
    let mut prefs = Preferencias::carregar(&arq);
    // Na primeira vez, `Documentos/FeatherCAD` já aparece em Projetos (pode ser tirada da lista depois).
    if primeiro_uso {
        if let Some(pasta) = pasta_padrao(app) {
            prefs.adicionar_projeto(&pasta.to_string_lossy());
            prefs.gravar(&arq)?;
        }
    }
    let recentes = inicio::recentes_existentes(&prefs).into_iter().map(|(item, aberto_em_ms)| RecenteDescrito { item, aberto_em_ms }).collect();
    Ok(EstadoInicio { recentes, projetos: prefs.projetos })
}

#[tauri::command]
pub fn inicio_estado(app: AppHandle) -> Result<EstadoInicio, String> {
    estado_inicio(&app)
}

#[tauri::command]
pub fn inicio_recente_remover(app: AppHandle, caminho: String) -> Result<EstadoInicio, String> {
    let arq = arquivo_de_preferencias(&app)?;
    let mut p = Preferencias::carregar(&arq);
    p.remover_recente(&caminho);
    p.gravar(&arq)?;
    estado_inicio(&app)
}

/// Escolhe uma pasta na caixa e a acrescenta aos projetos.
#[tauri::command]
pub async fn inicio_projeto_adicionar(app: AppHandle) -> Result<EstadoInicio, String> {
    let mut caixa = app.dialog().file();
    if let Some(inicial) = pasta_padrao(&app) {
        caixa = caixa.set_directory(inicial);
    }
    if let Some(pasta) = caixa.blocking_pick_folder() {
        let pasta = pasta.into_path().map_err(|e| format!("Caminho inválido: {e}"))?;
        let arq = arquivo_de_preferencias(&app)?;
        let mut p = Preferencias::carregar(&arq);
        p.adicionar_projeto(&pasta.to_string_lossy());
        p.gravar(&arq)?;
    }
    estado_inicio(&app)
}

#[tauri::command]
pub fn inicio_projeto_remover(app: AppHandle, pasta: String) -> Result<EstadoInicio, String> {
    let arq = arquivo_de_preferencias(&app)?;
    let mut p = Preferencias::carregar(&arq);
    p.remover_projeto(&pasta);
    p.gravar(&arq)?;
    estado_inicio(&app)
}

/// Os desenhos de uma pasta de projeto.
#[tauri::command]
pub fn inicio_pasta_listar(pasta: String) -> Result<Vec<ItemDeArquivo>, String> {
    inicio::listar_pasta(Path::new(&pasta))
}

/// Miniaturas já feitas, por caminho e data de modificação: a página inicial pede de novo a cada visita.
static MINIATURAS: Mutex<Vec<((String, u64), String)>> = Mutex::new(Vec::new());
const MINIATURAS_GUARDADAS: usize = 200;

/// A miniatura (SVG) de um desenho do disco, para a página inicial. Vazia se o desenho não tem o que mostrar.
/// Roda fora da thread da interface; o arquivo é lido e desenhado só aqui, nada passa pelo desenho aberto.
#[tauri::command]
pub async fn inicio_miniatura(caminho: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let modificado = std::fs::metadata(&caminho).map_err(|e| e.to_string())?.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
        let chave = (caminho.clone(), modificado);
        if let Some((_, svg)) = MINIATURAS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == chave) {
            return Ok(svg.clone());
        }
        let bytes = std::fs::read(&caminho).map_err(|e| e.to_string())?;
        let aberto = feathercad_io::abrir(&bytes)?;
        let svg = feathercad_io::pdf::miniatura_svg(&aberto.doc, 320.0, 200.0).unwrap_or_default();
        let mut guardadas = MINIATURAS.lock().unwrap_or_else(|e| e.into_inner());
        guardadas.retain(|(k, _)| k.0 != chave.0);
        if guardadas.len() >= MINIATURAS_GUARDADAS {
            guardadas.remove(0);
        }
        guardadas.push((chave, svg.clone()));
        Ok(svg)
    })
    .await
    .map_err(|e| e.to_string())?
}
