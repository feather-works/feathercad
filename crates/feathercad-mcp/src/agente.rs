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
    /// Modo projeto (`cad_modo`): troca os padrões de `cad_ver` por versões baratas em contexto.
    pub projeto: bool,
    /// O tema do aplicativo (fundo claro?), quando o servidor roda ao vivo dentro dele; `None` = isolado.
    pub tema_claro: Option<bool>,
    /// Alinhamentos calculados nesta sessão (nome → alinhamento e passo da estaca), para as ferramentas viárias.
    pub alinhamentos: std::collections::BTreeMap<String, (feathercad_core::viario::alinhamento::Alinhamento, f64)>,
    pub alinhamento_atual: Option<String>,
    /// Feições extraídas de nuvens nesta sessão (nome → pares de trilho e o que se achou de suspeito).
    pub feicoes: std::collections::BTreeMap<String, crate::nuvem_ferramentas::Feicoes>,
    pub nuvens: std::collections::BTreeMap<String, crate::nuvem_ferramentas::Nuvem>,
    pub nuvem_atual: Option<String>,
    pub greides: std::collections::BTreeMap<String, feathercad_core::viario::greide::Greide>,
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

/// Grava num `.parcial` ao lado, força para o disco e só então troca pelo destino: disco cheio ou queda no
/// meio não deixam o arquivo da pessoa cortado (o app já gravava assim).
pub(crate) fn gravar_seguro(destino: &std::path::Path, bytes: &[u8]) -> Res<()> {
    use std::io::Write;
    let mut nome = destino.as_os_str().to_owned();
    nome.push(".parcial");
    let tmp = PathBuf::from(nome);
    let r = std::fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(bytes)?;
        f.sync_all()
    });
    let r = r.and_then(|()| std::fs::rename(&tmp, destino));
    if let Err(e) = r {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("não consegui gravar `{}`: {e}", destino.display()));
    }
    Ok(())
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
    gravar_seguro(&d, &bytes)?;
    let antes = std::fs::metadata(origem).map_or(0, |m| m.len() as usize);
    let todos_os_avisos: Vec<String> = avisos.into_iter().chain(avisos_da_gravacao).collect();
    Ok(json!({ "ok": true, "entidades": doc.entidades().len(), "bytes_origem": antes, "bytes_destino": bytes.len(), "tokens_estimados_origem": tokens(antes), "tokens_estimados_destino": tokens(bytes.len()), "avisos": todos_os_avisos }))
}

impl Agente {
    /// Ao vivo o desenho é o da aba aberta: trocá-lo com alteração não salva só com `descartar: true`
    /// (antes `cad_novo`/`cad_abrir` jogavam fora o trabalho da pessoa, sem Ctrl+Z).
    pub(crate) fn pode_trocar_o_desenho(&self, args: &Value) -> Res<()> {
        if self.tema_claro.is_some() && self.doc.sujo() && !args.get("descartar").and_then(Value::as_bool).unwrap_or(false) {
            return Err("o desenho aberto na tela tem alterações não salvas: salve antes (`cad_salvar`) ou passe `descartar: true` para jogar fora".into());
        }
        Ok(())
    }

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
        gravar_seguro(&destino, &bytes)?;
        self.doc.marcar_salvo();
        self.arquivo = Some(destino.clone());
        Ok(json!({ "ok": true, "arquivo": destino.display().to_string(), "bytes": bytes.len(), "avisos": avisos }))
    }
}

#[cfg(test)]
mod testes_ao_vivo {
    use super::*;

    #[test]
    fn ao_vivo_nao_troca_desenho_com_alteracao_sem_descartar() {
        let mut ag = Agente { tema_claro: Some(true), ..Agente::default() };
        ag.doc.adicionar_linha(feathercad_core::geom::Ponto::novo(0.0, 0.0), feathercad_core::geom::Ponto::novo(1.0, 0.0));
        assert!(ag.pode_trocar_o_desenho(&json!({})).is_err(), "a aba tem trabalho não salvo");
        assert!(ag.pode_trocar_o_desenho(&json!({ "descartar": true })).is_ok());
        ag.tema_claro = None;
        assert!(ag.pode_trocar_o_desenho(&json!({})).is_ok(), "fora do modo ao vivo o desenho é só do MCP");
    }

    #[test]
    fn gravar_seguro_troca_o_arquivo_inteiro_e_nao_deixa_parcial() {
        let dir = std::env::temp_dir().join(format!("feathercad_gravar_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let alvo = dir.join("a.dxf");
        std::fs::write(&alvo, "antigo").unwrap();
        gravar_seguro(&alvo, b"novo").unwrap();
        assert_eq!(std::fs::read(&alvo).unwrap(), b"novo");
        assert!(!dir.join("a.dxf.parcial").exists());
        assert!(gravar_seguro(&dir.join("nao/existe.dxf"), b"x").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
