//! A paleta de propriedades dos objetos selecionados: o que os objetos têm em comum (ou "variado") e como
//! mudar de uma vez. Cor, camada, tipo de linha e espessura; "por camada" é o `None` da entidade.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::documento::{Documento, Id, Mudanca, Propriedades, TipoLinha};

/// O que a paleta mostra de um campo: sem objetos, o mesmo em todos, ou variado.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "estado", content = "valor", rename_all = "snake_case")]
pub enum Campo<T> {
    Vazio,
    Igual(T),
    Variado,
}

fn resumir<T: PartialEq + Clone>(valores: impl Iterator<Item = T>) -> Campo<T> {
    let mut resultado: Option<T> = None;
    for v in valores {
        match &resultado {
            None => resultado = Some(v),
            Some(r) if *r == v => {}
            Some(_) => return Campo::Variado,
        }
    }
    resultado.map_or(Campo::Vazio, Campo::Igual)
}

/// O resumo para a paleta. Os textos de "por camada" são as constantes `POR_CAMADA`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Resumo {
    pub n: usize,
    /// Nome da camada.
    pub camada: Campo<String>,
    /// `porcamada` ou `#rrggbb`.
    pub cor: Campo<String>,
    /// A cor que se vê de fato (a da camada, se a do objeto é "por camada"), quando é a mesma em todos.
    pub cor_efetiva: Campo<String>,
    /// `porcamada` ou o nome DXF (`DASHED`…).
    pub tipo_linha: Campo<String>,
    /// `porcamada` ou milímetros como texto (`0.5`).
    pub espessura: Campo<String>,
    /// As cores básicas (`#rrggbb`), na ordem: vermelho, amarelo, verde, ciano, azul, magenta, branco.
    pub cores_basicas: Vec<String>,
}

pub const POR_CAMADA: &str = "porcamada";

/// As sete cores básicas do AutoCAD (1 a 7): vermelho, amarelo, verde, ciano, azul, magenta, branco.
pub const CORES_BASICAS: [u32; 7] = [0xFF_00_00, 0xFF_FF_00, 0x00_FF_00, 0x00_FF_FF, 0x00_00_FF, 0xFF_00_FF, 0xFF_FF_FF];

/// O resumo dos objetos `ids` (os que ainda existem).
pub fn resumo(doc: &Documento, ids: &BTreeSet<Id>) -> Resumo {
    let ents: Vec<_> = ids.iter().filter_map(|id| doc.entidade(*id)).collect();
    let camada_de = |e: &crate::documento::Entidade| doc.camadas().get(e.camada);
    Resumo {
        n: ents.len(),
        camada: resumir(ents.iter().map(|e| camada_de(e).map_or_else(String::new, |c| c.nome.clone()))),
        cor: resumir(ents.iter().map(|e| e.cor.map_or_else(|| POR_CAMADA.to_string(), |c| format!("#{c:06x}")))),
        cor_efetiva: resumir(ents.iter().map(|e| format!("#{:06x}", doc.cor_resolvida(e)))),
        tipo_linha: resumir(ents.iter().map(|e| e.props.tipo_linha.map_or_else(|| POR_CAMADA.to_string(), |t| t.nome_dxf().to_string()))),
        espessura: resumir(ents.iter().map(|e| e.props.espessura.map_or_else(|| POR_CAMADA.to_string(), |mm| format!("{mm}")))),
        cores_basicas: CORES_BASICAS.iter().map(|c| format!("#{c:06x}")).collect(),
    }
}

/// Qual campo mudar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualCampo {
    Camada,
    Cor,
    TipoLinha,
    Espessura,
}

/// As mudanças que põem `valor` no `campo` de todos os `ids` (respeita camada bloqueada: esses ficam de fora).
/// `valor` é o texto que a paleta mandou; inválido devolve `Err` com a chave da mensagem.
pub fn definir(doc: &Documento, ids: &BTreeSet<Id>, campo: QualCampo, valor: &str) -> Result<Vec<Mudanca>, &'static str> {
    let valor = valor.trim();
    enum Novo {
        Camada(usize),
        Cor(Option<u32>),
        Props(fn(&mut Propriedades, &str) -> Result<(), &'static str>),
    }
    let novo = match campo {
        QualCampo::Camada => Novo::Camada(doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(valor)).ok_or("propriedades.semCamada")?),
        QualCampo::Cor => Novo::Cor(if valor.eq_ignore_ascii_case(POR_CAMADA) { None } else { Some(ler_cor(valor).ok_or("propriedades.corInvalida")?) }),
        QualCampo::TipoLinha => Novo::Props(|p, v| {
            p.tipo_linha = if v.eq_ignore_ascii_case(POR_CAMADA) { None } else { Some(TipoLinha::TODOS.into_iter().find(|t| t.nome_dxf().eq_ignore_ascii_case(v)).ok_or("propriedades.tipoInvalido")?) };
            Ok(())
        }),
        QualCampo::Espessura => Novo::Props(|p, v| {
            p.espessura = if v.eq_ignore_ascii_case(POR_CAMADA) { None } else { Some(v.replace(',', ".").parse::<f32>().ok().filter(|e| (0.0..=2.11).contains(e)).ok_or("propriedades.espessuraInvalida")?) };
            Ok(())
        }),
    };
    let mut ms = Vec::new();
    for e in ids.iter().filter_map(|id| doc.entidade(*id)).filter(|e| !doc.bloqueada(e)) {
        match &novo {
            Novo::Camada(c) => ms.push(Mudanca::Alterar { id: e.id, camada: Some(*c), cor: None, props: None }),
            Novo::Cor(c) => ms.push(Mudanca::Alterar { id: e.id, camada: None, cor: Some(*c), props: None }),
            Novo::Props(f) => {
                let mut p = e.props.clone();
                f(&mut p, valor)?;
                ms.push(Mudanca::Alterar { id: e.id, camada: None, cor: None, props: Some(p) });
            }
        }
    }
    Ok(ms)
}

/// `#rrggbb`, `rrggbb`, ou um dos sete números básicos do AutoCAD (1 vermelho … 7 branco).
pub fn ler_cor(texto: &str) -> Option<u32> {
    let t = texto.trim().trim_start_matches('#');
    if t.len() == 6 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        return u32::from_str_radix(t, 16).ok();
    }
    t.parse::<usize>().ok().filter(|n| (1..=7).contains(n)).map(|n| CORES_BASICAS[n - 1])
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::geom::Ponto;

    #[test]
    fn resumo_iguais_variados_e_mudar_tudo_de_uma_vez() {
        let mut d = Documento::default();
        let a = d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(1.0, 0.0));
        let b = d.adicionar_linha(Ponto::novo(0.0, 1.0), Ponto::novo(1.0, 1.0));
        d.nova_camada("Cotas", None).unwrap();
        let ids = BTreeSet::from([a, b]);
        let r = resumo(&d, &ids);
        assert_eq!((r.n, r.camada.clone(), r.cor.clone()), (2, Campo::Igual("0".into()), Campo::Igual(POR_CAMADA.into())));
        assert_eq!(resumo(&d, &BTreeSet::new()).camada, Campo::Vazio);
        d.aplicar_mudancas(definir(&d, &BTreeSet::from([a]), QualCampo::Cor, "#ff0000").unwrap());
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::Espessura, "0,5").unwrap());
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::TipoLinha, "hidden").unwrap());
        d.aplicar_mudancas(definir(&d, &BTreeSet::from([b]), QualCampo::Camada, "cotas").unwrap());
        let r = resumo(&d, &ids);
        assert_eq!((r.cor, r.camada), (Campo::Variado, Campo::Variado));
        assert_eq!((r.espessura, r.tipo_linha), (Campo::Igual("0.5".into()), Campo::Igual("HIDDEN".into())));
        assert_eq!(definir(&d, &ids, QualCampo::Cor, "verde"), Err("propriedades.corInvalida"));
        assert_eq!(definir(&d, &ids, QualCampo::Camada, "nao existe"), Err("propriedades.semCamada"));
        assert_eq!(definir(&d, &ids, QualCampo::Espessura, "9"), Err("propriedades.espessuraInvalida"));
        // Voltar tudo a "por camada".
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::Cor, "PorCamada").unwrap());
        assert_eq!(resumo(&d, &ids).cor, Campo::Igual(POR_CAMADA.into()));
        // Objeto em camada bloqueada fica de fora.
        d.definir_propriedade_de_camadas(&[1], crate::camadas::Propriedade::Bloqueada(true)).unwrap();
        assert_eq!(definir(&d, &ids, QualCampo::Cor, "#00ff00").unwrap().len(), 1);
    }
}
