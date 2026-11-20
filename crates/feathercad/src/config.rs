//! As preferências do usuário (as do antigo `preferencias.ts`, que ficavam no `localStorage`), num JSON na pasta de dados
//! do app. O que é do desenho vai no arquivo; isto é só do jeito de trabalhar.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// `auto`, `claro` ou `escuro`.
    pub tema: String,
    /// Índice em `feather_ui::tema::ACENTOS`.
    pub acento: usize,
    /// `nada`, `pontos` ou `grade`.
    pub fundo: String,
    pub orto_angulo: f64,
    pub snap_modos: BTreeMap<String, bool>,
    /// Altura do texto da cota, na unidade do desenho; `None` = automático.
    pub cota: Option<f64>,
    /// A unidade oferecida primeiro num desenho novo.
    pub unidade: String,
    /// O formato do arquivo novo (`dxf` ou `fcdz`).
    pub formato: String,
    /// `pt-BR`, `en-US`…; `None` = a do sistema.
    pub idioma: Option<String>,
    /// Apelidos digitados acrescentados pelo usuário (`{ "mover": ["G"] }`).
    pub apelidos: BTreeMap<String, Vec<String>>,
    /// Página inicial: `lista` ou `grade` (miniaturas).
    pub visao_inicio: String,
    pub fps: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tema: "auto".into(),
            acento: 0,
            fundo: "nada".into(),
            orto_angulo: 90.0,
            snap_modos: BTreeMap::new(),
            cota: None,
            unidade: "mm".into(),
            formato: "dxf".into(),
            idioma: None,
            apelidos: BTreeMap::new(),
            visao_inicio: "lista".into(),
            fps: false,
        }
    }
}

fn arquivo() -> Option<PathBuf> {
    crate::arquivos::pasta_de_dados().map(|p| p.join("config.json"))
}

impl Config {
    /// Arquivo ausente ou ilegível = o padrão (um JSON estragado não impede o app de abrir).
    pub fn carregar() -> Config {
        arquivo().and_then(|a| std::fs::read(a).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn gravar(&self) -> Result<(), String> {
        let a = arquivo().ok_or("sem pasta de dados")?;
        if let Some(p) = a.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let parcial = a.with_extension("parcial");
        std::fs::write(&parcial, serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(parcial, a).map_err(|e| e.to_string())
    }

    pub fn claro(&self) -> bool {
        match self.tema.as_str() {
            "claro" => true,
            "escuro" => false,
            _ => !pelicano::system_dark(),
        }
    }
}

#[cfg(test)]
mod testes {
    #[test]
    fn json_velho_ou_parcial_vira_padrao_no_que_falta() {
        let c: super::Config = serde_json::from_str(r#"{"fundo":"grade"}"#).unwrap();
        assert_eq!(c.fundo, "grade");
        assert_eq!(c.orto_angulo, 90.0);
    }
}
