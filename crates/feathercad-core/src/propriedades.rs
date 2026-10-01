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
    pub posicao_x: Campo<String>,
    pub posicao_y: Campo<String>,
    pub comprimento: Campo<String>,
    pub raio: Campo<String>,
    pub diametro: Campo<String>,
    pub area: Campo<String>,
    pub tamanho_geral: Campo<String>,
    pub rotacao: Campo<String>,
    pub conteudo: Campo<String>,
}

pub const POR_CAMADA: &str = "porcamada";

/// As sete cores básicas do AutoCAD (1 a 7): vermelho, amarelo, verde, ciano, azul, magenta, branco.
pub const CORES_BASICAS: [u32; 7] = [0xFF_00_00, 0xFF_FF_00, 0x00_FF_00, 0x00_FF_FF, 0x00_00_FF, 0xFF_00_FF, 0xFF_FF_FF];

fn num_fmt(val: f64) -> String {
    let s = format!("{:.3}", val);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
}

fn posicao_x(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Linha { a, b } => Some(num_fmt((a.x + b.x) / 2.0)),
        crate::documento::Geometria::Circulo { centro, .. } | crate::documento::Geometria::Arco { centro, .. } => Some(num_fmt(centro.x)),
        crate::documento::Geometria::Texto { ponto, .. } | crate::documento::Geometria::Insercao { ponto, .. } | crate::documento::Geometria::Ponto { p: ponto } => Some(num_fmt(ponto.x)),
        crate::documento::Geometria::Polilinha { vertices, .. } => vertices.first().map(|v| num_fmt(v.p.x)),
        _ => None,
    }
}

fn posicao_y(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Linha { a, b } => Some(num_fmt((a.y + b.y) / 2.0)),
        crate::documento::Geometria::Circulo { centro, .. } | crate::documento::Geometria::Arco { centro, .. } => Some(num_fmt(centro.y)),
        crate::documento::Geometria::Texto { ponto, .. } | crate::documento::Geometria::Insercao { ponto, .. } | crate::documento::Geometria::Ponto { p: ponto } => Some(num_fmt(ponto.y)),
        crate::documento::Geometria::Polilinha { vertices, .. } => vertices.first().map(|v| num_fmt(v.p.y)),
        _ => None,
    }
}

fn comprimento(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Linha { a, b } => Some(num_fmt(a.distancia(*b))),
        crate::documento::Geometria::Circulo { raio, .. } => Some(num_fmt(2.0 * std::f64::consts::PI * raio)),
        crate::documento::Geometria::Arco { raio, inicio, fim, .. } => {
            let varr = crate::geom::varredura_graus(*inicio, *fim);
            Some(num_fmt(raio * varr.to_radians()))
        }
        crate::documento::Geometria::Polilinha { vertices, fechada } => {
            if vertices.len() < 2 {
                return None;
            }
            let mut total = 0.0;
            for i in 0..vertices.len() - 1 {
                total += vertices[i].p.distancia(vertices[i + 1].p);
            }
            if *fechada && vertices.len() > 2 {
                total += vertices.last().unwrap().p.distancia(vertices[0].p);
            }
            Some(num_fmt(total))
        }
        _ => None,
    }
}

fn raio(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Circulo { raio, .. } | crate::documento::Geometria::Arco { raio, .. } => Some(num_fmt(*raio)),
        _ => None,
    }
}

fn diametro(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Circulo { raio, .. } | crate::documento::Geometria::Arco { raio, .. } => Some(num_fmt(2.0 * raio)),
        _ => None,
    }
}

fn area(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Circulo { raio, .. } => Some(num_fmt(std::f64::consts::PI * raio * raio)),
        crate::documento::Geometria::Polilinha { vertices, fechada: true } if vertices.len() >= 3 => {
            let mut a = 0.0;
            for i in 0..vertices.len() {
                let j = (i + 1) % vertices.len();
                a += vertices[i].p.x * vertices[j].p.y - vertices[j].p.x * vertices[i].p.y;
            }
            Some(num_fmt((a / 2.0).abs()))
        }
        _ => None,
    }
}

fn tamanho_geral(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Texto { altura, .. } => Some(num_fmt(*altura)),
        crate::documento::Geometria::Circulo { raio, .. } => Some(num_fmt(2.0 * raio)),
        crate::documento::Geometria::Linha { a, b } => Some(num_fmt(a.distancia(*b))),
        crate::documento::Geometria::Insercao { escala_x, .. } => Some(num_fmt(*escala_x)),
        _ => None,
    }
}

fn rotacao(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Texto { rotacao, .. } | crate::documento::Geometria::Insercao { rotacao, .. } => Some(format!("{:.1}°", rotacao)),
        crate::documento::Geometria::Linha { a, b } => Some(format!("{:.1}°", a.angulo_ate(*b))),
        _ => None,
    }
}

fn conteudo(e: &crate::documento::Entidade) -> Option<String> {
    match &e.geometria {
        crate::documento::Geometria::Texto { conteudo, .. } => Some(conteudo.clone()),
        _ => None,
    }
}

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
        posicao_x: resumir(ents.iter().copied().filter_map(posicao_x)),
        posicao_y: resumir(ents.iter().copied().filter_map(posicao_y)),
        comprimento: resumir(ents.iter().copied().filter_map(comprimento)),
        raio: resumir(ents.iter().copied().filter_map(raio)),
        diametro: resumir(ents.iter().copied().filter_map(diametro)),
        area: resumir(ents.iter().copied().filter_map(area)),
        tamanho_geral: resumir(ents.iter().copied().filter_map(tamanho_geral)),
        rotacao: resumir(ents.iter().copied().filter_map(rotacao)),
        conteudo: resumir(ents.iter().copied().filter_map(conteudo)),
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
    PosicaoX,
    PosicaoY,
    Comprimento,
    Raio,
    Diametro,
    Area,
    TamanhoGeral,
    Rotacao,
    Conteudo,
}

fn alterar_posicao_x(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nx = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    match g {
        crate::documento::Geometria::Linha { a, b } => {
            let cx = (a.x + b.x) / 2.0;
            let dx = nx - cx;
            Ok(Some(crate::documento::Geometria::Linha {
                a: crate::geom::Ponto::novo3d(a.x + dx, a.y, a.z),
                b: crate::geom::Ponto::novo3d(b.x + dx, b.y, b.z),
            }))
        }
        crate::documento::Geometria::Circulo { centro, raio } => Ok(Some(crate::documento::Geometria::Circulo {
            centro: crate::geom::Ponto::novo3d(nx, centro.y, centro.z),
            raio: *raio,
        })),
        crate::documento::Geometria::Arco { centro, raio, inicio, fim } => Ok(Some(crate::documento::Geometria::Arco {
            centro: crate::geom::Ponto::novo3d(nx, centro.y, centro.z),
            raio: *raio,
            inicio: *inicio,
            fim: *fim,
        })),
        crate::documento::Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => Ok(Some(crate::documento::Geometria::Texto {
            ponto: crate::geom::Ponto::novo3d(nx, ponto.y, ponto.z),
            altura: *altura,
            rotacao: *rotacao,
            conteudo: conteudo.clone(),
            h: *h,
            v: *v,
        })),
        crate::documento::Geometria::Insercao { ponto, bloco, escala_x, escala_y, rotacao } => Ok(Some(crate::documento::Geometria::Insercao {
            ponto: crate::geom::Ponto::novo3d(nx, ponto.y, ponto.z),
            bloco: *bloco,
            escala_x: *escala_x,
            escala_y: *escala_y,
            rotacao: *rotacao,
        })),
        crate::documento::Geometria::Ponto { p } => Ok(Some(crate::documento::Geometria::Ponto {
            p: crate::geom::Ponto::novo3d(nx, p.y, p.z),
        })),
        crate::documento::Geometria::Polilinha { vertices, fechada } => {
            if let Some(v0) = vertices.first() {
                let dx = nx - v0.p.x;
                let novos = vertices.iter().map(|v| crate::geom::Vertice { p: crate::geom::Ponto::novo3d(v.p.x + dx, v.p.y, v.p.z), bulge: v.bulge }).collect();
                Ok(Some(crate::documento::Geometria::Polilinha { vertices: novos, fechada: *fechada }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn alterar_posicao_y(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let ny = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    match g {
        crate::documento::Geometria::Linha { a, b } => {
            let cy = (a.y + b.y) / 2.0;
            let dy = ny - cy;
            Ok(Some(crate::documento::Geometria::Linha {
                a: crate::geom::Ponto::novo3d(a.x, a.y + dy, a.z),
                b: crate::geom::Ponto::novo3d(b.x, b.y + dy, b.z),
            }))
        }
        crate::documento::Geometria::Circulo { centro, raio } => Ok(Some(crate::documento::Geometria::Circulo {
            centro: crate::geom::Ponto::novo3d(centro.x, ny, centro.z),
            raio: *raio,
        })),
        crate::documento::Geometria::Arco { centro, raio, inicio, fim } => Ok(Some(crate::documento::Geometria::Arco {
            centro: crate::geom::Ponto::novo3d(centro.x, ny, centro.z),
            raio: *raio,
            inicio: *inicio,
            fim: *fim,
        })),
        crate::documento::Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => Ok(Some(crate::documento::Geometria::Texto {
            ponto: crate::geom::Ponto::novo3d(ponto.x, ny, ponto.z),
            altura: *altura,
            rotacao: *rotacao,
            conteudo: conteudo.clone(),
            h: *h,
            v: *v,
        })),
        crate::documento::Geometria::Insercao { ponto, bloco, escala_x, escala_y, rotacao } => Ok(Some(crate::documento::Geometria::Insercao {
            ponto: crate::geom::Ponto::novo3d(ponto.x, ny, ponto.z),
            bloco: *bloco,
            escala_x: *escala_x,
            escala_y: *escala_y,
            rotacao: *rotacao,
        })),
        crate::documento::Geometria::Ponto { p } => Ok(Some(crate::documento::Geometria::Ponto {
            p: crate::geom::Ponto::novo3d(p.x, ny, p.z),
        })),
        crate::documento::Geometria::Polilinha { vertices, fechada } => {
            if let Some(v0) = vertices.first() {
                let dy = ny - v0.p.y;
                let novos = vertices.iter().map(|v| crate::geom::Vertice { p: crate::geom::Ponto::novo3d(v.p.x, v.p.y + dy, v.p.z), bulge: v.bulge }).collect();
                Ok(Some(crate::documento::Geometria::Polilinha { vertices: novos, fechada: *fechada }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn alterar_tamanho_geral(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nval = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    if nval <= 0.0 {
        return Err("propriedades.numeroInvalido");
    }
    match g {
        crate::documento::Geometria::Texto { ponto, rotacao, conteudo, h, v, .. } => Ok(Some(crate::documento::Geometria::Texto {
            ponto: *ponto,
            altura: nval,
            rotacao: *rotacao,
            conteudo: conteudo.clone(),
            h: *h,
            v: *v,
        })),
        crate::documento::Geometria::Circulo { centro, .. } => Ok(Some(crate::documento::Geometria::Circulo {
            centro: *centro,
            raio: nval / 2.0,
        })),
        crate::documento::Geometria::Arco { centro, inicio, fim, .. } => Ok(Some(crate::documento::Geometria::Arco {
            centro: *centro,
            raio: nval / 2.0,
            inicio: *inicio,
            fim: *fim,
        })),
        crate::documento::Geometria::Insercao { ponto, bloco, rotacao, .. } => Ok(Some(crate::documento::Geometria::Insercao {
            ponto: *ponto,
            bloco: *bloco,
            escala_x: nval,
            escala_y: nval,
            rotacao: *rotacao,
        })),
        crate::documento::Geometria::Linha { a, b } => {
            let cx = (a.x + b.x) / 2.0;
            let cy = (a.y + b.y) / 2.0;
            let ang = a.angulo_ate(*b).to_radians();
            let half = nval / 2.0;
            let dx = half * ang.cos();
            let dy = half * ang.sin();
            Ok(Some(crate::documento::Geometria::Linha {
                a: crate::geom::Ponto::novo3d(cx - dx, cy - dy, a.z),
                b: crate::geom::Ponto::novo3d(cx + dx, cy + dy, b.z),
            }))
        }
        _ => Ok(None),
    }
}

fn alterar_raio(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nr = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    if nr <= 0.0 {
        return Err("propriedades.numeroInvalido");
    }
    match g {
        crate::documento::Geometria::Circulo { centro, .. } => Ok(Some(crate::documento::Geometria::Circulo { centro: *centro, raio: nr })),
        crate::documento::Geometria::Arco { centro, inicio, fim, .. } => Ok(Some(crate::documento::Geometria::Arco { centro: *centro, raio: nr, inicio: *inicio, fim: *fim })),
        _ => Ok(None),
    }
}

fn alterar_diametro(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nd = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    if nd <= 0.0 {
        return Err("propriedades.numeroInvalido");
    }
    let nr = nd / 2.0;
    match g {
        crate::documento::Geometria::Circulo { centro, .. } => Ok(Some(crate::documento::Geometria::Circulo { centro: *centro, raio: nr })),
        crate::documento::Geometria::Arco { centro, inicio, fim, .. } => Ok(Some(crate::documento::Geometria::Arco { centro: *centro, raio: nr, inicio: *inicio, fim: *fim })),
        _ => Ok(None),
    }
}

fn alterar_comprimento(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nval = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    if nval <= 0.0 {
        return Err("propriedades.numeroInvalido");
    }
    match g {
        crate::documento::Geometria::Linha { a, b } => {
            let cx = (a.x + b.x) / 2.0;
            let cy = (a.y + b.y) / 2.0;
            let ang = a.angulo_ate(*b).to_radians();
            let half = nval / 2.0;
            let dx = half * ang.cos();
            let dy = half * ang.sin();
            Ok(Some(crate::documento::Geometria::Linha {
                a: crate::geom::Ponto::novo3d(cx - dx, cy - dy, a.z),
                b: crate::geom::Ponto::novo3d(cx + dx, cy + dy, b.z),
            }))
        }
        crate::documento::Geometria::Circulo { centro, .. } => {
            let nr = nval / (2.0 * std::f64::consts::PI);
            Ok(Some(crate::documento::Geometria::Circulo { centro: *centro, raio: nr }))
        }
        _ => Ok(None),
    }
}

fn alterar_area(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let nval = v.replace(',', ".").parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    if nval <= 0.0 {
        return Err("propriedades.numeroInvalido");
    }
    match g {
        crate::documento::Geometria::Circulo { centro, .. } => {
            let nr = (nval / std::f64::consts::PI).sqrt();
            Ok(Some(crate::documento::Geometria::Circulo { centro: *centro, raio: nr }))
        }
        _ => Ok(None),
    }
}

fn alterar_rotacao(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    let clean = v.trim().trim_end_matches('°').trim().replace(',', ".");
    let nrot = clean.parse::<f64>().map_err(|_| "propriedades.numeroInvalido")?;
    let nrot_norm = crate::geom::normalizar_graus(nrot);
    match g {
        crate::documento::Geometria::Texto { ponto, altura, conteudo, h, v: align_v, .. } => Ok(Some(crate::documento::Geometria::Texto {
            ponto: *ponto,
            altura: *altura,
            rotacao: nrot_norm,
            conteudo: conteudo.clone(),
            h: *h,
            v: *align_v,
        })),
        crate::documento::Geometria::Insercao { ponto, bloco, escala_x, escala_y, .. } => Ok(Some(crate::documento::Geometria::Insercao {
            ponto: *ponto,
            bloco: *bloco,
            escala_x: *escala_x,
            escala_y: *escala_y,
            rotacao: nrot_norm,
        })),
        crate::documento::Geometria::Linha { a, b } => {
            let cx = (a.x + b.x) / 2.0;
            let cy = (a.y + b.y) / 2.0;
            let half = a.distancia(*b) / 2.0;
            let rad = nrot_norm.to_radians();
            let dx = half * rad.cos();
            let dy = half * rad.sin();
            Ok(Some(crate::documento::Geometria::Linha {
                a: crate::geom::Ponto::novo3d(cx - dx, cy - dy, a.z),
                b: crate::geom::Ponto::novo3d(cx + dx, cy + dy, b.z),
            }))
        }
        _ => Ok(None),
    }
}

fn alterar_conteudo(g: &crate::documento::Geometria, v: &str) -> Result<Option<crate::documento::Geometria>, &'static str> {
    match g {
        crate::documento::Geometria::Texto { ponto, altura, rotacao, h, v: align_v, .. } => Ok(Some(crate::documento::Geometria::Texto {
            ponto: *ponto,
            altura: *altura,
            rotacao: *rotacao,
            conteudo: v.to_string(),
            h: *h,
            v: *align_v,
        })),
        _ => Ok(None),
    }
}

/// As mudanças que põem `valor` no `campo` de todos os `ids` (respeita camada bloqueada: esses ficam de fora).
/// `valor` é o texto que a paleta mandou; inválido devolve `Err` com a chave da mensagem.
pub fn definir(doc: &Documento, ids: &BTreeSet<Id>, campo: QualCampo, valor: &str) -> Result<Vec<Mudanca>, &'static str> {
    let valor = valor.trim();
    enum Novo {
        Camada(usize),
        Cor(Option<u32>),
        Props(fn(&mut Propriedades, &str) -> Result<(), &'static str>),
        Geometria(fn(&crate::documento::Geometria, &str) -> Result<Option<crate::documento::Geometria>, &'static str>),
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
        QualCampo::PosicaoX => Novo::Geometria(alterar_posicao_x),
        QualCampo::PosicaoY => Novo::Geometria(alterar_posicao_y),
        QualCampo::TamanhoGeral => Novo::Geometria(alterar_tamanho_geral),
        QualCampo::Raio => Novo::Geometria(alterar_raio),
        QualCampo::Diametro => Novo::Geometria(alterar_diametro),
        QualCampo::Comprimento => Novo::Geometria(alterar_comprimento),
        QualCampo::Area => Novo::Geometria(alterar_area),
        QualCampo::Rotacao => Novo::Geometria(alterar_rotacao),
        QualCampo::Conteudo => Novo::Geometria(alterar_conteudo),
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
            Novo::Geometria(f) => {
                if let Some(nova_g) = f(&e.geometria, valor)? {
                    ms.push(Mudanca::Trocar(e.id, nova_g));
                }
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

    #[test]
    fn mudar_geometria_tamanho_rotacao_e_conteudo() {
        let mut d = Documento::default();
        let txt_id = d.adicionar_geometria(crate::documento::Geometria::Texto {
            ponto: Ponto::novo(10.0, 20.0),
            altura: 2.5,
            rotacao: 0.0,
            conteudo: "QD1".into(),
            h: crate::documento::AlinhaH::Esquerda,
            v: crate::documento::AlinhaV::Base,
        });
        let ids = BTreeSet::from([txt_id]);

        // Testar alteração de conteúdo
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::Conteudo, "QUADRO GERAL").unwrap());
        let r = resumo(&d, &ids);
        assert_eq!(r.conteudo, Campo::Igual("QUADRO GERAL".into()));

        // Testar alteração de tamanho geral (altura)
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::TamanhoGeral, "5.0").unwrap());
        let r = resumo(&d, &ids);
        assert_eq!(r.tamanho_geral, Campo::Igual("5".into()));

        // Testar alteração de rotação
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::Rotacao, "45°").unwrap());
        let r = resumo(&d, &ids);
        assert_eq!(r.rotacao, Campo::Igual("45.0°".into()));

        // Testar alteração de posição X e Y
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::PosicaoX, "100.0").unwrap());
        d.aplicar_mudancas(definir(&d, &ids, QualCampo::PosicaoY, "200.0").unwrap());
        let r = resumo(&d, &ids);
        assert_eq!(r.posicao_x, Campo::Igual("100".into()));
        assert_eq!(r.posicao_y, Campo::Igual("200".into()));
    }
}
