//! Os nomes que o usuário digita. As tabelas vêm do arquivo de língua
//! (`comandos.<língua>.json` no TypeScript) e são do Rust: é o Rust quem
//! interpreta a linha de comando.
//!
//! Duas tabelas, porque o mesmo apelido significa coisas diferentes conforme o
//! momento: `C` é o comando `CÍRCULO` no início, mas dentro da polilinha a opção
//! `Fechar` tem o seu próprio apelido. Uma opção só vale enquanto a ferramenta a
//! oferece.

use std::collections::{BTreeMap, HashMap};

/// Os comandos que o motor conhece (o nome interno, em minúsculas).
pub const CONHECIDOS: [&str; 34] = [
    "linha",
    "polilinha",
    "circulo",
    "arco",
    "inserir",
    "desfazer",
    "refazer",
    "apagar",
    "mover",
    "copiar",
    "girar",
    "escala",
    "espelhar",
    "esticar",
    "matriz",
    "aparar",
    "concordancia",
    "retangulo",
    "elipse",
    "hachura",
    "texto",
    "cota",
    "chamada",
    "tabela",
    "medir",
    "contar",
    "colar",
    "tornar_atual",
    "igualar_propriedades",
    "copiar_area",
    "recortar",
    "igualar_camada",
    "criar_bloco",
    "agrupar",
];

/// As opções de ferramenta que o motor conhece.
pub const OPCOES: [&str; 5] = ["fechar", "solida", "padrao", "area", "distancia"];

/// Nome digitado (ou apelido) → comando ou opção.
#[derive(Clone, Debug, Default)]
pub struct Comandos {
    apelidos: HashMap<String, &'static str>,
    opcoes: HashMap<String, &'static str>,
}

/// Uma tabela `{ "linha": ["LINHA", "L"] }` → apelido em maiúsculas → nome interno conhecido.
fn montar(tabela: &BTreeMap<String, Vec<String>>, conhecidos: &[&'static str], o_que: &str) -> Result<HashMap<String, &'static str>, String> {
    let mut novos = HashMap::new();
    for (id, apelidos) in tabela {
        let Some(conhecido) = conhecidos.iter().find(|c| **c == id.as_str()) else {
            return Err(format!("{o_que} desconhecido na tabela de língua: {id}"));
        };
        for a in apelidos {
            novos.insert(a.trim().to_uppercase(), *conhecido);
        }
    }
    Ok(novos)
}

impl Comandos {
    /// Troca as tabelas. Nome desconhecido é recusado, para um erro de digitação no
    /// arquivo de língua aparecer.
    pub fn definir(&mut self, comandos: &BTreeMap<String, Vec<String>>, opcoes: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
        let (c, o) = (montar(comandos, &CONHECIDOS, "comando")?, montar(opcoes, &OPCOES, "opção")?);
        self.apelidos = c;
        self.opcoes = o;
        Ok(())
    }

    /// O comando por trás do que foi digitado. **Só vale o que está na tabela da
    /// língua**: o nome interno (`linha`) não é digitável em toda língua. Botão e
    /// atalho usam `executar`, não isto.
    pub fn resolver(&self, digitado: &str) -> Option<&'static str> {
        self.apelidos.get(&digitado.trim().to_uppercase()).copied()
    }

    /// A opção de ferramenta por trás do que foi digitado (`Fechar`).
    pub fn resolver_opcao(&self, digitado: &str) -> Option<&'static str> {
        self.opcoes.get(&digitado.trim().to_uppercase()).copied()
    }
}
