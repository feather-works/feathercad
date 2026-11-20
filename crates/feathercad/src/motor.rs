//! O motor visto pela interface: os pedidos da `Ponte` (o mesmo contrato JSON do antigo `src/ponte/index.ts`)
//! com tipos Rust. Toda conta (vista, snap, comandos) é do motor; aqui só se monta o pedido e se lê a resposta.

use feathercad_core::vista::Vista;
use feathercad_ponte::Ponte;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

/// Texto para traduzir: chave no arquivo de língua + variáveis.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Mensagem {
    pub chave: String,
    #[serde(default)]
    pub vars: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entrada {
    Digitado(String),
    Mensagem(Mensagem),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Arquivo {
    pub caminho: String,
    pub nome: String,
    pub formato: String,
}

/// O estado que o motor devolve a cada mudança (só os campos que a interface usa).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Estado {
    pub versao: u64,
    pub unidade: Option<String>,
    pub orto_angulo: f64,
    pub arrastando: bool,
    pub janela_pendente: bool,
    pub sujo: bool,
    pub arquivo: Option<Arquivo>,
    pub avisos_do_arquivo: Vec<Mensagem>,
    pub pode_desfazer: bool,
    pub pode_refazer: bool,
    pub camada_atual: String,
    pub ferramenta: String,
    pub selecionados: usize,
    pub selecao_rev: u64,
    pub cota_tamanho: Option<f64>,
    pub layouts: Vec<Layout>,
    pub espaco_ativo: Value,
    pub fundo: String,
    pub snap: bool,
    pub orto: bool,
    pub osnap: bool,
    pub osnap_modos: std::collections::BTreeMap<String, bool>,
    pub prompt: Mensagem,
    pub blocos: Vec<String>,
    pub historico: Vec<Entrada>,
}

/// Um layout (espaço do papel).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub nome: String,
    pub papel: String,
    pub largura_mm: f64,
    pub altura_mm: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Segmento {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// O cursor resolvido (snap, orto, grade), com guias, prévia e o texto da entrada dinâmica.
#[derive(Debug, Clone, Deserialize)]
pub struct Cursor {
    pub x: f32,
    pub y: f32,
    pub texto_x: String,
    pub texto_y: String,
    pub texto_distancia: Option<String>,
    pub texto_angulo: Option<String>,
    pub guias: Vec<Segmento>,
    pub previa: Vec<Segmento>,
    pub marca: bool,
    pub snap: Option<String>,
    pub janela: Option<String>,
}

/// Uma linha do menu do botão direito, como o motor a monta.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemDeMenu {
    pub id: String,
    pub chave: String,
    pub atalho: Option<String>,
    pub marcado: Option<bool>,
    pub ativo: bool,
    pub separador: bool,
    pub repetido: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RespostaAtalho {
    pub tratado: bool,
    pub janela: Option<String>,
    pub estado: Estado,
}

#[derive(Default)]
pub struct Motor {
    pub ponte: Ponte,
}

impl Motor {
    pub fn json<T: DeserializeOwned>(&mut self, comando: &str, corpo: Value) -> Result<T, String> {
        let bytes = self.ponte.chamar(comando, corpo.to_string().as_bytes())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("resposta ilegível de `{comando}`: {e}"))
    }

    /// Um pedido que devolve o estado novo.
    pub fn estado_de(&mut self, comando: &str, corpo: Value) -> Result<Estado, String> {
        self.json(comando, corpo)
    }

    pub fn estado(&mut self) -> Result<Estado, String> {
        self.estado_de("estado", Value::Null)
    }

    pub fn vista(&mut self, comando: &str, corpo: Value) -> Result<Vista, String> {
        self.json(comando, corpo)
    }

    pub fn vista_texto(&mut self, v: &Vista) -> String {
        self.json::<Value>("vista_texto", json!({ "vista": v })).ok().and_then(|r| r["zoom"].as_str().map(str::to_string)).unwrap_or_default()
    }

    pub fn cursor(&mut self, v: &Vista, x: f32, y: f32) -> Option<Cursor> {
        self.json("cursor_mover", json!({ "vista": v, "x": x, "y": y })).ok()
    }

    pub fn mouse(&mut self, comando: &str, v: &Vista, x: f32, y: f32, aditivo: bool) -> Result<Estado, String> {
        self.estado_de(comando, json!({ "vista": v, "x": x, "y": y, "aditivo": aditivo }))
    }

    pub fn quadro(&mut self, v: &Vista, versao: Option<u64>, fundo_claro: bool) -> Result<Vec<u8>, String> {
        self.ponte.chamar("quadro", json!({ "vista": v, "versao_conhecida": versao, "fundo_claro": fundo_claro }).to_string().as_bytes())
    }

    pub fn quadro_arrasto(&mut self, v: &Vista, x: f32, y: f32, fundo_claro: bool) -> Result<Vec<u8>, String> {
        self.ponte.chamar("quadro_arrasto", json!({ "vista": v, "x": x, "y": y, "fundo_claro": fundo_claro }).to_string().as_bytes())
    }

    pub fn menu(&mut self, v: &Vista, x: f32, y: f32) -> Result<(Estado, Vec<ItemDeMenu>), String> {
        #[derive(Deserialize)]
        struct R {
            estado: Estado,
            itens: Vec<ItemDeMenu>,
        }
        let r: R = self.json("menu_contexto", json!({ "vista": v, "x": x, "y": y, "aditivo": false }))?;
        Ok((r.estado, r.itens))
    }

    pub fn gravar(&mut self, formato: &str) -> Result<Vec<u8>, String> {
        self.ponte.chamar("documento_gravar", json!({ "formato": formato }).to_string().as_bytes())
    }

    pub fn abrir(&mut self, bytes: &[u8]) -> Result<Estado, String> {
        let r = self.ponte.chamar("documento_abrir", bytes)?;
        serde_json::from_slice(&r).map_err(|e| format!("estado ilegível: {e}"))
    }
}

