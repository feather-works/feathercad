//! Ler e gravar os arquivos do FeatherCAD: DXF (texto) e `.fcdz`. Nenhum
//! arquivo é confiado: malformado dá erro com mensagem em português, nunca pânico.

pub mod aci;
pub mod dxf;
pub mod fcdz;
pub mod inicio;
pub mod pdf;

use feathercad_core::documento::Documento;
use feathercad_core::sessao::Mensagem;

/// Os formatos de arquivo do app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formato {
    Dxf,
    Fcdz,
}

impl Formato {
    /// A extensão (sem ponto) que o formato usa.
    pub fn extensao(self) -> &'static str {
        match self {
            Formato::Dxf => "dxf",
            Formato::Fcdz => "fcdz",
        }
    }

    /// O formato de uma extensão (sem ponto, sem diferenciar maiúscula).
    pub fn da_extensao(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "dxf" => Some(Formato::Dxf),
            "fcdz" => Some(Formato::Fcdz),
            _ => None,
        }
    }
}

/// O que se leu: o documento, o que ficou de fora e a versão do DXF (se era DXF).
#[derive(Debug)]
pub struct Aberto {
    pub doc: Documento,
    pub avisos: Vec<Mensagem>,
    pub versao_dxf: Option<String>,
}

/// O que se gravou: os bytes e o que foi simplificado.
#[derive(Debug)]
pub struct Gravado {
    pub bytes: Vec<u8>,
    pub avisos: Vec<Mensagem>,
}

fn decodificar_texto(bytes: &[u8]) -> String {
    let sem_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(sem_bom) {
        Ok(t) => t.to_string(),
        Err(_) => sem_bom.iter().map(|b| char::from(*b)).collect(),
    }
}

fn tentar_abrir_nuvem(bytes: &[u8]) -> Option<Result<Aberto, String>> {
    let texto = decodificar_texto(bytes);
    let inicio = texto.trim_start();
    if inicio.starts_with("0\n") || inicio.starts_with("0\r\n") || inicio.starts_with("999\n") || inicio.starts_with("999\r\n") {
        return None;
    }
    match feathercad_core::topografia::ler_nuvem_pontos(&texto) {
        Ok(pontos) if pontos.len() >= 3 => {
            let mut doc = Documento::default();
            match feathercad_core::topografia::criar_levantamento_topografico(
                &mut doc,
                &pontos,
                &feathercad_core::topografia::OpcoesTopografia::default(),
            ) {
                Ok(resumo) => {
                    let aviso = Mensagem::nova("aviso.nuvemCarregada")
                        .com("pontos", &resumo.total_pontos.to_string())
                        .com("curvas", &resumo.total_curvas.to_string());
                    Some(Ok(Aberto {
                        doc,
                        avisos: vec![aviso],
                        versao_dxf: None,
                    }))
                }
                Err(e) => Some(Err(e)),
            }
        }
        _ => None,
    }
}

/// Abre pelo **conteúdo** (um ZIP é `.fcdz`; nuvem vira levantamento topográfico; o resto é tentado como DXF).
pub fn abrir(bytes: &[u8]) -> Result<Aberto, String> {
    if bytes.starts_with(b"PK") {
        fcdz::ler(bytes)
    } else if let Some(resultado) = tentar_abrir_nuvem(bytes) {
        resultado
    } else {
        dxf::leitor::ler(bytes)
    }
}

/// Grava no formato pedido.
pub fn gravar(doc: &Documento, formato: Formato) -> Result<Gravado, String> {
    match formato {
        Formato::Dxf => {
            let (bytes, avisos) = dxf::escritor::gravar(doc);
            Ok(Gravado { bytes, avisos })
        }
        Formato::Fcdz => Ok(Gravado { bytes: fcdz::gravar(doc)?, avisos: Vec::new() }),
    }
}
