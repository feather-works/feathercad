//! FeatherCAD: CAD 2D com motor e interface em Rust (pelicano). O motor é o `feathercad-core` (pela `feathercad-ponte`);
//! esta casca só desenha o que ele manda e repassa os gestos.

mod app;
mod area;
mod arquivos;
mod caixas;
mod camadas;
mod config;
mod configuracoes;
mod cubo;
mod doc;
mod faixa;
mod icones;
mod idioma;
mod inicio;
mod mcp_vivo;
mod motor;
mod propriedades;
mod quadro;
mod telas;

fn main() -> Result<(), pelicano::Error> {
    // `feathercad desenho.dxf` (ou o duplo clique do sistema): só o que existe e não é opção.
    let arquivos: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file()).collect();
    // Já há um FeatherCAD aberto: ele recebe os arquivos e vem para a frente; este sai.
    if std::env::var("FEATHERCAD_VIVO").map_or(true, |v| v != "0") && feathercad_mcp::ao_vivo::pedir_foco(&arquivos) {
        return Ok(());
    }
    eprintln!("[FeatherCAD] {}", mcp_vivo::ligar());
    // O pelicano roda as tarefas (caixas de arquivo) no tokio dele; este dá um contexto ao que o `update` chama direto.
    let rt = tokio::runtime::Runtime::new().expect("falha ao iniciar o tokio");
    let _guarda = rt.enter();
    let ajustes = pelicano::Settings {
        title: "FeatherCAD".into(),
        app_id: Some("feathercad".into()),
        size: pelicano::Size::new(1360.0, 860.0),
        min_size: Some(pelicano::Size::new(900.0, 560.0)),
        decorations: false,
        transparent: !cfg!(windows),
        close_on_request: false,
        ..pelicano::Settings::default()
    };
    let r = pelicano::run(ajustes, move || app::App::novo(arquivos));
    mcp_vivo::encerrar();
    r
}
