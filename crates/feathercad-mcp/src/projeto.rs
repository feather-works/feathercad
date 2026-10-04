//! O projeto calculado, como as folhas o leem: traçado, superelevação, greide, o levantamento projetado no traçado e a análise
//! de criticidade. Só consulta; as contas estão no `feathercad-core`.

use feathercad_core::viario::alinhamento::Alinhamento;
use feathercad_core::viario::criticidade::{cota_existente, Analise, Elemento, Observacao};
use feathercad_core::viario::greide::Greide;
use feathercad_core::viario::superelevacao::{Regras, ResultadoCurva};

pub(crate) struct Projeto<'a> {
    pub a: &'a Alinhamento,
    pub tabela: &'a [ResultadoCurva],
    pub regras: &'a Regras,
    pub greide: &'a Greide,
    pub analise: &'a Analise,
    /// Em ordem de progressiva.
    pub obs: &'a [Observacao],
}

impl Projeto<'_> {
    /// Cota do greide em `s` (progressiva absoluta).
    pub fn zg(&self, s: f64) -> f64 {
        self.greide.z(s - self.a.progressiva_inicial)
    }

    /// Cota existente em `s`, se o levantamento cobre ali.
    pub fn ze(&self, s: f64) -> Option<f64> {
        cota_existente(self.obs, s)
    }

    /// O elemento (tangente ou curva) que contém `s`.
    pub fn elemento(&self, s: f64) -> Option<&Elemento> {
        self.analise.elementos.iter().find(|e| s >= e.de - 1e-9 && s <= e.ate + 1e-9)
    }
}
