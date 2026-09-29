//! O texto pronto para a tela: o TypeScript nunca formata número.

use serde::{Deserialize, Serialize};

/// Como a língua escreve número (vem do arquivo de língua, via ponte).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Regional {
    /// Separador decimal (`,` no pt-BR, `.` no en-US).
    pub decimal: char,
    /// Separador entre duas coordenadas digitadas (`;` no pt-BR, `,` no en-US).
    pub separador_de_coordenadas: char,
    /// Casas decimais mostradas.
    pub casas: usize,
}

impl Default for Regional {
    fn default() -> Self {
        Self { decimal: ',', separador_de_coordenadas: ';', casas: 3 }
    }
}

impl Regional {
    /// Escreve um número na língua (`-0,000` vira `0,000`).
    pub fn numero(&self, valor: f64) -> String {
        self.numero_com(valor, self.casas)
    }

    /// Como `numero`, com `casas` decimais à escolha (porcentagem, contagem…).
    pub fn numero_com(&self, valor: f64, casas: usize) -> String {
        let texto = format!("{valor:.casas$}");
        let texto = if texto.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
            texto.trim_start_matches('-').to_string()
        } else {
            texto
        };
        texto.replace('.', &self.decimal.to_string())
    }

    /// Lê um número escrito na língua; `None` se não for número.
    pub fn ler_numero(&self, texto: &str) -> Option<f64> {
        let limpo = texto.trim().replace(self.decimal, ".");
        limpo.parse::<f64>().ok().filter(|v| v.is_finite())
    }
}
