//! O estado do servidor: o documento aberto e de onde ele veio. Cada chamada de ferramenta age sobre ele.
//! Abre e grava `.dxf`, `.fcdz` e `.fct` (o texto compacto, ver `compacto.rs`).

use std::path::{Path, PathBuf};

use feathercad_core::documento::{Documento, Unidade};
use feathercad_io::Formato;
use serde_json::{json, Value};

use crate::compacto;
use crate::json::Res;
use crate::mutacoes::aplicar_compacto;

#[derive(Default)]
pub struct Agente {
    pub doc: Documento,
    pub arquivo: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq)]
enum Fmt {
    Io(u8),
    Compacto,
}

const DXF: Fmt = Fmt::Io(0);
const FCDZ: Fmt = Fmt::Io(1);

fn formato_do_caminho(caminho: &Path) -> Res<Fmt> {
    match caminho.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("dxf") => Ok(DXF),
        Some("fcdz") => Ok(FCDZ),
        Some("fct") => Ok(Fmt::Compacto),
        Some("xyz" | "pts" | "csv" | "txt") => Ok(DXF),
        _ => Err("só trabalho com `.dxf`, `.fcdz`, `.fct` ou nuvem de pontos (`.xyz`, `.pts`, `.csv`, `.txt`)".into()),
    }
}

fn avisos_de(v: &[feathercad_core::sessao::Mensagem]) -> Vec<String> {
    v.iter().map(|m| format!("{} {:?}", m.chave, m.vars)).collect()
}

/// Lê um arquivo do disco como documento (e os avisos do que ficou de fora).
pub fn carregar(caminho: &str) -> Res<(Documento, Vec<String>)> {
    let p = PathBuf::from(caminho);
    let f = formato_do_caminho(&p)?;
    let bytes = std::fs::read(&p).map_err(|e| format!("não consegui ler `{caminho}`: {e}"))?;
    if f == Fmt::Compacto {
        let texto = String::from_utf8(bytes).map_err(|_| "o `.fct` não é UTF-8")?;
        let lido = compacto::importar(&texto)?;
        let mut doc = Documento::default().com_unidade(lido.unidade.as_deref().and_then(Unidade::de_nome));
        aplicar_compacto(&mut doc, &lido)?;
        doc.marcar_salvo();
        return Ok((doc, Vec::new()));
    }
    let aberto = feathercad_io::abrir(&bytes)?;
    Ok((aberto.doc, avisos_de(&aberto.avisos)))
}

/// Os bytes do documento no formato da extensão de `caminho` (e os avisos do que foi simplificado).
pub fn gravar_bytes(doc: &Documento, caminho: &Path) -> Res<(Vec<u8>, Vec<String>)> {
    match caminho.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("fct") => Ok((compacto::exportar(doc, None, false).0.into_bytes(), Vec::new())),
        Some("dxf") => {
            let g = feathercad_io::gravar(doc, Formato::Dxf)?;
            Ok((g.bytes, avisos_de(&g.avisos)))
        }
        Some("fcdz") => {
            let g = feathercad_io::gravar(doc, Formato::Fcdz)?;
            Ok((g.bytes, avisos_de(&g.avisos)))
        }
        _ => Err("só gravo em `.dxf`, `.fcdz` ou `.fct`".into()),
    }
}

/// Estimativa grosseira de tokens: cerca de 3,5 caracteres por token.
fn tokens(bytes: usize) -> usize {
    (bytes as f64 / 3.5).round() as usize
}

/// Converte `origem` em `destino` sem mexer no desenho aberto, contando o quanto cada um pesaria em tokens.
pub fn converter(origem: &str, destino: &str, sobrescrever: bool) -> Res<Value> {
    let d = PathBuf::from(destino);
    if d.exists() && !sobrescrever {
        return Err(format!("`{destino}` já existe; passe `sobrescrever: true` se é isso mesmo"));
    }
    let (doc, avisos) = carregar(origem)?;
    let (bytes, avisos_da_gravacao) = gravar_bytes(&doc, &d)?;
    std::fs::write(&d, &bytes).map_err(|e| format!("não consegui gravar `{destino}`: {e}"))?;
    let antes = std::fs::metadata(origem).map_or(0, |m| m.len() as usize);
    let todos_os_avisos: Vec<String> = avisos.into_iter().chain(avisos_da_gravacao).collect();
    Ok(json!({ "ok": true, "entidades": doc.entidades().len(), "bytes_origem": antes, "bytes_destino": bytes.len(), "tokens_estimados_origem": tokens(antes), "tokens_estimados_destino": tokens(bytes.len()), "avisos": todos_os_avisos }))
}

impl Agente {
    /// Um desenho em branco na unidade pedida (`mm`, `cm`, `m`).
    pub fn novo(&mut self, unidade: &str) -> Res<Value> {
        let u = Unidade::de_nome(unidade).ok_or("a unidade tem que ser `mm`, `cm` ou `m`")?;
        self.doc = Documento::default().com_unidade(Some(u));
        self.arquivo = None;
        Ok(json!({ "ok": true, "unidade": unidade, "nota": "desenho em branco; X para a direita, Y para cima, ângulos em graus no sentido anti-horário a partir do eixo X" }))
    }

    /// Abre um arquivo do disco no lugar do desenho atual.
    pub fn abrir(&mut self, caminho: &str) -> Res<Value> {
        let (doc, avisos) = carregar(caminho)?;
        self.doc = doc;
        self.arquivo = Some(PathBuf::from(caminho));
        Ok(json!({ "ok": true, "arquivo": caminho, "entidades": self.doc.entidades().len(), "unidade": self.doc.unidade().map(Unidade::nome), "avisos": avisos }))
    }

    /// Grava no caminho dado (ou no de onde abriu). Não sobrescreve arquivo existente sem `sobrescrever`.
    /// Se `verificar_checklist` for verdadeiro ou `dominio` for informado, valida o checklist antes de salvar.
    pub fn salvar(&mut self, caminho: Option<&str>, sobrescrever: bool, verificar_checklist: bool, dominio: Option<&str>) -> Res<Value> {
        let destino = match (caminho, &self.arquivo) {
            (Some(c), _) => PathBuf::from(c),
            (None, Some(a)) => a.clone(),
            (None, None) => return Err("diga o `caminho` (.dxf, .fcdz ou .fct): este desenho ainda não tem arquivo".into()),
        };
        let mesmo_arquivo = self.arquivo.as_deref() == Some(destino.as_path());
        if destino.exists() && !sobrescrever && !mesmo_arquivo {
            return Err(format!("`{}` já existe; passe `sobrescrever: true` se é isso mesmo", destino.display()));
        }

        if let Some(dom) = dominio.or(if verificar_checklist { Some("geral") } else { None }) {
            let chk = crate::consulta::checklist_do_dominio(&self.doc, dom);
            if !chk["aprovado"].as_bool().unwrap_or(true) {
                let pendencias: Vec<String> = chk["checklist"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter(|i| !i["passou"].as_bool().unwrap_or(false))
                    .map(|i| format!("• {}: {}", i["item"].as_str().unwrap_or(""), i["detalhes"].as_str().unwrap_or("")))
                    .collect();
                return Err(format!(
                    "Salvamento Bloqueado! Checklist do domínio `{}` REPROVADO:\n{}\n\nCorrija as pendências acimas antes de salvar.",
                    dom,
                    pendencias.join("\n")
                ));
            }
        }

        let (bytes, avisos) = gravar_bytes(&self.doc, &destino)?;
        std::fs::write(&destino, &bytes).map_err(|e| format!("não consegui gravar `{}`: {e}", destino.display()))?;
        self.doc.marcar_salvo();
        self.arquivo = Some(destino.clone());
        Ok(json!({ "ok": true, "arquivo": destino.display().to_string(), "bytes": bytes.len(), "avisos": avisos }))
    }
}
