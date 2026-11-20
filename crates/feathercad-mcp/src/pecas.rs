//! Construtor comum das ferramentas de documento (tabela, gráfico, texto em bloco, detalhe…): acumula formas
//! com camada e cor e as grava de uma vez (um passo de desfazer). Medidas em **mm de papel** quando a chamada
//! traz `escala` (`fator_papel`); sem escala valem as unidades do desenho.

use std::collections::HashMap;

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Espaco, Geometria, Id, Mudanca, NovaEntidade, Propriedades};
use feathercad_core::geom::{Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{arred, numero_opcional, par, Res};
use crate::mutacoes::{camada_por_nome, cor_hex};

pub struct Peca {
    pub camada: String,
    pub cor: Option<u32>,
    pub g: Geometria,
}

/// Acumula formas na camada e cor correntes.
pub struct Pincel {
    pub camada: String,
    pub cor: Option<u32>,
    pub pecas: Vec<Peca>,
}

impl Pincel {
    pub fn novo(camada: &str) -> Self {
        Self { camada: camada.to_string(), cor: None, pecas: Vec::new() }
    }

    pub fn em(&mut self, camada: &str, cor: Option<u32>) -> &mut Self {
        self.camada = camada.to_string();
        self.cor = cor;
        self
    }

    fn dar(&mut self, g: Geometria) {
        self.pecas.push(Peca { camada: self.camada.clone(), cor: self.cor, g });
    }

    pub fn linha(&mut self, a: Ponto, b: Ponto) {
        self.dar(Geometria::Linha { a, b });
    }

    pub fn retangulo(&mut self, min: Ponto, max: Ponto) {
        let v = [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)].map(|p| Vertice { p, bulge: 0.0 }).to_vec();
        self.dar(Geometria::Polilinha { vertices: v, fechada: true });
    }

    pub fn polilinha(&mut self, pontos: &[Ponto]) {
        match pontos.len() {
            0 | 1 => {}
            2 => self.linha(pontos[0], pontos[1]),
            _ => self.dar(Geometria::Polilinha { vertices: pontos.iter().map(|p| Vertice { p: *p, bulge: 0.0 }).collect(), fechada: false }),
        }
    }

    pub fn texto(&mut self, p: Ponto, altura: f64, giro: f64, conteudo: &str, h: AlinhaH, v: AlinhaV) {
        if !conteudo.is_empty() {
            self.dar(Geometria::Texto { ponto: p, altura, rotacao: giro, conteudo: conteudo.to_string(), h, v });
        }
    }
}

/// Grava as peças (um passo de desfazer) e devolve os ids.
pub fn aplicar(doc: &mut Documento, pecas: Vec<Peca>) -> Res<Vec<Id>> {
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut ms = Vec::with_capacity(pecas.len());
    for p in pecas {
        let camada = match indices.get(&p.camada) {
            Some(i) => *i,
            None => {
                let i = camada_por_nome(doc, &p.camada)?;
                indices.insert(p.camada.clone(), i);
                i
            }
        };
        ms.push(Mudanca::Novo(NovaEntidade { camada, cor: p.cor, geometria: p.g, props: Propriedades::default(), espaco: Espaco::default() }));
    }
    Ok(doc.aplicar_mudancas(ms))
}

/// Unidades do desenho por mm de papel: `escala` (1:S) ÷ mm da unidade. Sem `escala`, 1 (as medidas já são do desenho).
pub fn fator_papel(doc: &Documento, args: &Value) -> Res<f64> {
    match numero_opcional(args, "escala") {
        Some(s) if s > 0.0 => Ok(s / doc.unidade().map_or(1.0, feathercad_core::documento::Unidade::em_milimetros)),
        Some(_) => Err("`escala` precisa ser maior que zero (100 = 1:100)".into()),
        None => Ok(1.0),
    }
}

/// A cor opcional `cor` (#RRGGBB) de `args`.
pub fn cor_de(args: &Value, nome: &str) -> Res<Option<u32>> {
    args.get(nome).and_then(Value::as_str).map(cor_hex).transpose()
}

/// Número para tabela/rótulo: `decimais` casas, vírgula decimal se `virgula`.
pub fn formatar(v: f64, decimais: usize, virgula: bool) -> String {
    // `decimais` vem do pedido: 10 bilhões de casas eram uma string de 10 GB.
    let decimais = decimais.min(12);
    let s = format!("{v:.decimais$}");
    let s = if s == "-0" || s.starts_with("-0.") && s[1..].chars().all(|c| c == '0' || c == '.') { s[1..].to_string() } else { s };
    if virgula { s.replace('.', ",") } else { s }
}

/// Casas decimais que um passo (0,5; 0,25; 10) pede para ser escrito sem sobra.
pub fn casas_do_passo(passo: f64) -> usize {
    (0..=6).find(|c| ((passo * 10f64.powi(*c as i32)).round() - passo * 10f64.powi(*c as i32)).abs() < 1e-9).unwrap_or(2)
}

/// A caixa de um conjunto de peças (só para o relatório).
pub fn caixa_das(pecas: &[Peca]) -> Option<(Ponto, Ponto)> {
    let (mut min, mut max) = (Ponto::novo(f64::MAX, f64::MAX), Ponto::novo(f64::MIN, f64::MIN));
    let mut tem = false;
    for p in pecas {
        let mut ponta = |q: Ponto| {
            tem = true;
            min = Ponto::novo(min.x.min(q.x), min.y.min(q.y));
            max = Ponto::novo(max.x.max(q.x), max.y.max(q.y));
        };
        match &p.g {
            Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => crate::leiaute::caixa_do_texto(*ponto, *altura, *rotacao, conteudo, *h, *v).into_iter().for_each(&mut ponta),
            g => g.para_cada_ponto_de_caixa(&mut ponta),
        }
    }
    tem.then_some((min, max))
}

/// O relatório padrão: quantas formas, a caixa e os ids.
pub fn relatorio(ids: &[Id], pecas_caixa: Option<(Ponto, Ponto)>) -> Value {
    json!({ "criadas": ids.len(), "caixa": pecas_caixa.map(|(a, b)| json!({ "min": par(a), "max": par(b), "largura": arred(b.x - a.x), "altura": arred(b.y - a.y) })), "primeiro_id": ids.first(), "ultimo_id": ids.last() })
}
