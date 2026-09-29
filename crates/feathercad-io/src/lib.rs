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

/// Abre pelo **conteúdo** (um ZIP é `.fcdz`; o resto é tentado como DXF), não pela extensão.
pub fn abrir(bytes: &[u8]) -> Result<Aberto, String> {
    if bytes.starts_with(b"PK") {
        fcdz::ler(bytes)
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
