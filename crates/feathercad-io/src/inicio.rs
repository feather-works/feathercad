//! A página inicial: arquivos recentes, pastas de projeto e a listagem de uma
//! pasta. Tudo **local** (regra 5 da suíte): não há "projetos na nuvem"; um
//! projeto é uma pasta do disco onde o usuário guarda desenhos.
//!
//! As preferências vão num JSON pequeno (`inicio.json`) na pasta de dados do
//! aplicativo; arquivo ausente ou corrompido vira lista vazia, nunca erro.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::Formato;

/// Quantos arquivos recentes se guardam.
const TETO_DE_RECENTES: usize = 30;

/// Um arquivo aberto ou salvo, com o momento (ms desde 1970).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recente {
    pub caminho: String,
    pub em_ms: u64,
}

/// O que a página inicial guarda entre uma sessão e outra.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferencias {
    pub recentes: Vec<Recente>,
    /// Pastas de projeto, na ordem em que foram adicionadas.
    pub projetos: Vec<String>,
}

impl Preferencias {
    /// Lê o arquivo; ausente ou ilegível dá as preferências vazias.
    pub fn carregar(caminho: &Path) -> Self {
        fs::read(caminho).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Grava (criando a pasta se preciso). A gravação é atômica: escreve `.parcial` e renomeia.
    pub fn gravar(&self, caminho: &Path) -> Result<(), String> {
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta).map_err(|e| format!("Não consegui criar a pasta \"{}\": {e}", pasta.display()))?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| format!("Não consegui montar as preferências: {e}"))?;
        let parcial = caminho.with_extension("parcial");
        fs::write(&parcial, bytes).map_err(|e| format!("Não consegui gravar \"{}\": {e}", parcial.display()))?;
        fs::rename(&parcial, caminho).map_err(|e| format!("Não consegui gravar \"{}\": {e}", caminho.display()))
    }

    /// Põe o arquivo no topo dos recentes (sem repetir) e corta no teto.
    pub fn adicionar_recente(&mut self, caminho: &str, agora_ms: u64) {
        self.recentes.retain(|r| r.caminho != caminho);
        self.recentes.insert(0, Recente { caminho: caminho.to_string(), em_ms: agora_ms });
        self.recentes.truncate(TETO_DE_RECENTES);
    }

    /// Tira um arquivo dos recentes (o usuário pediu, ou ele sumiu do disco).
    pub fn remover_recente(&mut self, caminho: &str) {
        self.recentes.retain(|r| r.caminho != caminho);
    }

    /// Acrescenta uma pasta de projeto (sem repetir).
    pub fn adicionar_projeto(&mut self, pasta: &str) {
        if !self.projetos.iter().any(|p| p == pasta) {
            self.projetos.push(pasta.to_string());
        }
    }

    /// Tira uma pasta da lista de projetos (a pasta em si não é apagada).
    pub fn remover_projeto(&mut self, pasta: &str) {
        self.projetos.retain(|p| p != pasta);
    }
}

/// Um desenho no disco, como a página inicial o mostra.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemDeArquivo {
    pub nome: String,
    pub caminho: String,
    /// `dxf` ou `fcdz`.
    pub tipo: &'static str,
    pub modificado_ms: u64,
    /// Nem todo sistema de arquivos guarda a data de criação.
    pub criado_ms: Option<u64>,
    /// A pasta onde está.
    pub pasta: String,
}

fn ms(t: std::io::Result<std::time::SystemTime>) -> Option<u64> {
    t.ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

/// Descreve um arquivo que existe e é de um formato do app; `None` caso contrário.
pub fn descrever(caminho: &Path) -> Option<ItemDeArquivo> {
    let meta = fs::metadata(caminho).ok().filter(fs::Metadata::is_file)?;
    let tipo = Formato::da_extensao(caminho.extension()?.to_str()?)?.extensao();
    Some(ItemDeArquivo {
        nome: caminho.file_name()?.to_string_lossy().into_owned(),
        caminho: caminho.to_string_lossy().into_owned(),
        tipo,
        modificado_ms: ms(meta.modified()).unwrap_or(0),
        criado_ms: ms(meta.created()),
        pasta: caminho.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
    })
}

/// Os desenhos (`.dxf` e `.fcdz`) de uma pasta, do mais recente para o mais antigo.
/// Não desce em subpastas.
pub fn listar_pasta(pasta: &Path) -> Result<Vec<ItemDeArquivo>, String> {
    let entradas = fs::read_dir(pasta).map_err(|e| format!("Não consegui abrir a pasta \"{}\": {e}", pasta.display()))?;
    let mut itens: Vec<ItemDeArquivo> = entradas.flatten().filter_map(|e| descrever(&e.path())).collect();
    itens.sort_by(|a, b| b.modificado_ms.cmp(&a.modificado_ms).then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase())));
    Ok(itens)
}

/// Os recentes que ainda existem no disco, já descritos. Os que sumiram ficam de fora.
pub fn recentes_existentes(prefs: &Preferencias) -> Vec<(ItemDeArquivo, u64)> {
    prefs.recentes.iter().filter_map(|r| descrever(Path::new(&r.caminho)).map(|i| (i, r.em_ms))).collect()
}
