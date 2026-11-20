//! Abrir e salvar desenhos e a lista de recentes. A leitura/gravação do DXF e do `.fcdz` é do `feathercad-io` (pela
//! `Ponte`); aqui só há disco e caixas de diálogo. Mesmo comportamento do antigo `src-tauri/src/arquivos.rs`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use feathercad_io::inicio::{self, ItemDeArquivo, Preferencias};
use feathercad_io::Formato;
use serde_json::json;

use crate::motor::{Estado, Motor};

/// A pasta de dados do app: a mesma do Tauri (`br.feathercad.app`), para os recentes de antes continuarem valendo.
pub fn pasta_de_dados() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    Some(base?.join("br.feathercad.app"))
}

fn arquivo_de_preferencias() -> Option<PathBuf> {
    pasta_de_dados().map(|p| p.join("inicio.json"))
}

fn agora_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub fn lembrar(caminho: &Path) {
    let Some(arq) = arquivo_de_preferencias() else { return };
    let mut p = Preferencias::carregar(&arq);
    p.adicionar_recente(&caminho.to_string_lossy(), agora_ms());
    if let Some(pasta) = arq.parent() {
        let _ = std::fs::create_dir_all(pasta);
    }
    let _ = p.gravar(&arq);
}

/// As preferências da página inicial (recentes e pastas de projeto). Na primeira vez, `Documentos/FeatherCAD` já
/// entra em Projetos (pode ser tirada da lista depois), como no app Tauri.
pub fn preferencias() -> Preferencias {
    let Some(arq) = arquivo_de_preferencias() else { return Preferencias::default() };
    let primeira_vez = !arq.exists();
    let mut p = Preferencias::carregar(&arq);
    if primeira_vez {
        if let Some(pasta) = pasta_padrao() {
            p.adicionar_projeto(&pasta.to_string_lossy());
            if let Some(d) = arq.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = p.gravar(&arq);
        }
    }
    p
}

/// Muda e grava as preferências da página inicial.
pub fn alterar(f: impl FnOnce(&mut Preferencias)) {
    let Some(arq) = arquivo_de_preferencias() else { return };
    let mut p = Preferencias::carregar(&arq);
    f(&mut p);
    if let Some(d) = arq.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = p.gravar(&arq);
}

/// `Documentos/FeatherCAD` (criada se não existir); `None` se não há como.
pub fn pasta_padrao() -> Option<PathBuf> {
    let casa = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)?;
    let docs = ["Documentos", "Documents"].iter().map(|d| casa.join(d)).find(|d| d.is_dir()).unwrap_or_else(|| casa.join("Documents"));
    let pasta = docs.join("FeatherCAD");
    std::fs::create_dir_all(&pasta).ok()?;
    Some(pasta)
}

/// Os recentes que ainda existem no disco, do mais novo ao mais antigo, com a hora em que foram abertos.
pub fn recentes() -> Vec<(ItemDeArquivo, u64)> {
    inicio::recentes_existentes(&preferencias())
}

/// A caixa "escolher pasta" (Projetos).
pub async fn escolher_pasta() -> Option<PathBuf> {
    let mut caixa = rfd::AsyncFileDialog::new();
    if let Some(p) = pasta_padrao() {
        caixa = caixa.set_directory(p);
    }
    caixa.pick_folder().await.map(|f| f.path().to_path_buf())
}

/// A miniatura (SVG) de um desenho do disco; vazia se não há o que mostrar. Lê e desenha só aqui, fora do desenho aberto.
pub fn miniatura(caminho: &Path) -> Option<String> {
    let bytes = std::fs::read(caminho).ok()?;
    let aberto = feathercad_io::abrir(&bytes).ok()?;
    feathercad_io::pdf::miniatura_svg(&aberto.doc, 320.0, 200.0)
}

/// Data e hora locais de um instante (ms desde 1970) como `dd/mm/aaaa hh:mm`, em UTC.
/// shortcut: sem fuso horário (a biblioteca padrão não o conhece); trocar quando houver dependência de data na suíte.
pub fn data(ms: u64) -> String {
    let s = (ms / 1000) as i64;
    let (dias, resto) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // Dias desde 1970 → data civil (algoritmo de Howard Hinnant).
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let a = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}/{m:02}/{a} {:02}:{:02}", resto / 3600, resto % 3600 / 60)
}

pub fn formato_do_caminho(caminho: &Path) -> Option<Formato> {
    caminho.extension().and_then(|e| e.to_str()).and_then(Formato::da_extensao)
}

fn eh_nuvem_de_pontos(caminho: &Path) -> bool {
    matches!(caminho.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("xyz" | "pts" | "csv" | "txt"))
}

/// A caixa "Abrir" (fora da thread da interface).
pub async fn escolher_para_abrir(pasta: Option<PathBuf>) -> Option<PathBuf> {
    let mut caixa = rfd::AsyncFileDialog::new()
        .add_filter("Todos os formatos (*.dxf, *.fcdz, *.xyz, *.pts, *.csv, *.txt)", &["dxf", "fcdz", "xyz", "pts", "csv", "txt"])
        .add_filter("Desenhos (DXF, FCDZ)", &["dxf", "fcdz"])
        .add_filter("Nuvem de pontos (XYZ, PTS, CSV, TXT)", &["xyz", "pts", "csv", "txt"]);
    if let Some(p) = pasta {
        caixa = caixa.set_directory(p);
    }
    caixa.pick_file().await.map(|f| f.path().to_path_buf())
}

/// A caixa "Salvar como", com o formato atual na frente. Sem extensão conhecida, ganha a do formato.
pub async fn escolher_para_salvar(atual: Option<PathBuf>, nome: String, padrao: Formato) -> Option<(PathBuf, Formato)> {
    let filtros = [(Formato::Dxf, "DXF"), (Formato::Fcdz, "FeatherCAD (.fcdz)")];
    let mut caixa = rfd::AsyncFileDialog::new().set_file_name(nome);
    for (f, n) in filtros.iter().filter(|(f, _)| *f == padrao).chain(filtros.iter().filter(|(f, _)| *f != padrao)) {
        caixa = caixa.add_filter(*n, &[f.extensao()]);
    }
    if let Some(p) = atual.as_deref().and_then(Path::parent).filter(|p| p.is_dir()) {
        caixa = caixa.set_directory(p);
    }
    let mut caminho = caixa.save_file().await?.path().to_path_buf();
    let formato = formato_do_caminho(&caminho).unwrap_or(padrao);
    if formato_do_caminho(&caminho).is_none() {
        caminho.set_extension(padrao.extensao());
    }
    Some((caminho, formato))
}

/// Abre o arquivo no motor. Nuvem de pontos vira um `.dxf` novo ao lado (ainda não salvo).
pub fn abrir(motor: &mut Motor, caminho: &Path) -> Result<Estado, String> {
    let nuvem = eh_nuvem_de_pontos(caminho);
    let formato = formato_do_caminho(caminho);
    if !nuvem && formato.is_none() {
        return Err(format!("\"{}\" não é um desenho .dxf/.fcdz nem uma nuvem de pontos (.xyz, .pts, .csv, .txt).", caminho.display()));
    }
    let bytes = std::fs::read(caminho).map_err(|e| format!("Não consegui ler \"{}\": {e}", caminho.display()))?;
    motor.abrir(&bytes)?;
    let (destino, formato, conhecido) = match formato {
        Some(f) if !nuvem => (caminho.to_path_buf(), f.extensao(), true),
        _ => (caminho.with_file_name(format!("{}.dxf", caminho.file_stem().and_then(|s| s.to_str()).unwrap_or("topografia"))), "dxf", false),
    };
    motor.estado_de("documento_arquivo_definir", json!({ "caminho": destino.to_string_lossy(), "formato": formato, "salvo": false }))?;
    if conhecido {
        lembrar(caminho);
    }
    motor.estado()
}

/// Grava o desenho em `caminho` (num `.parcial` e renomeia: uma falha no meio não estraga o arquivo que já existia).
pub fn gravar(motor: &mut Motor, caminho: &Path, formato: Formato) -> Result<Estado, String> {
    let bytes = motor.gravar(formato.extensao())?;
    let parcial = caminho.with_extension("parcial");
    std::fs::write(&parcial, bytes).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))?;
    std::fs::rename(&parcial, caminho).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))?;
    let e = motor.estado_de("documento_arquivo_definir", json!({ "caminho": caminho.to_string_lossy(), "formato": formato.extensao(), "salvo": true }))?;
    lembrar(caminho);
    Ok(e)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn grava_e_reabre_dxf_e_fcdz() {
        let pasta = std::env::temp_dir().join(format!("feathercad_arquivos_{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        std::env::set_var("XDG_DATA_HOME", &pasta);
        std::env::set_var("HOME", &pasta); // a 1ª vez cria Documentos/FeatherCAD: que seja aqui
        for (nome, f) in [("a.dxf", Formato::Dxf), ("a.fcdz", Formato::Fcdz)] {
            let mut m = Motor::default();
            m.estado_de("documento_novo", json!({ "unidade": "mm" })).unwrap();
            m.estado_de("linha_criar", json!({ "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } })).unwrap();
            let caminho = pasta.join(nome);
            let e = gravar(&mut m, &caminho, f).unwrap();
            assert!(!e.sujo, "gravado = limpo");
            let mut outro = Motor::default();
            let e = abrir(&mut outro, &caminho).unwrap();
            assert_eq!(e.arquivo.unwrap().formato, f.extensao());
        }
        assert_eq!(recentes().len(), 2, "os dois entram nos recentes");
        assert!(miniatura(&pasta.join("a.fcdz")).is_some_and(|s| s.contains("<svg")));
        assert_eq!(data(0), "01/01/1970 00:00");
        assert_eq!(data(1_794_960_000_000), "18/11/2026 00:00");
        assert!(abrir(&mut Motor::default(), &pasta.join("x.doc")).is_err());
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
