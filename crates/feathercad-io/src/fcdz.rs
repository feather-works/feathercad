//! O `.fcdz`: contêiner ZIP com o desenho em JSON dentro (`manifest.json` +
//! `data/desenho.json`). A estrutura de `FORMATO.md` prevê também um `.git/`
//! embutido; ele **ainda não entrou** (a decisão de quando commitar segue
//! pendente naquele documento). Sem compatibilidade com rascunhos: se o formato
//! mudar antes da primeira versão, muda.

use std::io::{Cursor, Read, Write};

use feathercad_core::documento::{AlinhaH, AlinhaV, Bloco, Camada, Documento, Geometria, LinhaDePadrao, NovaEntidade, Propriedades, TipoLinha, Unidade};
use feathercad_core::geom::{Ponto, Vertice};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use crate::Aberto;

const VERSAO: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Manifesto {
    aplicativo: String,
    formato: u32,
}

#[derive(Serialize, Deserialize)]
struct CamadaJson {
    nome: String,
    cor: u32,
    ligada: bool,
    #[serde(default)]
    congelada: bool,
    #[serde(default)]
    bloqueada: bool,
    #[serde(default = "verdadeiro")]
    plotar: bool,
    #[serde(default = "continua")]
    tipo_linha: TipoLinha,
    #[serde(default)]
    espessura: Option<f32>,
    #[serde(default)]
    transparencia: u8,
}

fn verdadeiro() -> bool {
    true
}

fn continua() -> TipoLinha {
    TipoLinha::Continua
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
enum GeometriaJson {
    Linha { a: Ponto, b: Ponto },
    Polilinha { vertices: Vec<Vertice>, fechada: bool },
    Insercao { bloco: usize, ponto: Ponto, escala_x: f64, escala_y: f64, rotacao: f64 },
    Hachura { contornos: Vec<Vec<Vertice>>, padrao: Vec<LinhaDePadrao>, nome: String },
    Ponto { p: Ponto },
    Circulo { centro: Ponto, raio: f64 },
    Arco { centro: Ponto, raio: f64, inicio: f64, fim: f64 },
    Texto { ponto: Ponto, altura: f64, rotacao: f64, conteudo: String, h: AlinhaH, v: AlinhaV },
}

impl From<&Geometria> for GeometriaJson {
    fn from(g: &Geometria) -> Self {
        match g.clone() {
            Geometria::Linha { a, b } => GeometriaJson::Linha { a, b },
            Geometria::Polilinha { vertices, fechada } => GeometriaJson::Polilinha { vertices, fechada },
            Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => GeometriaJson::Insercao { bloco, ponto, escala_x, escala_y, rotacao },
            Geometria::Hachura { contornos, padrao, nome } => GeometriaJson::Hachura { contornos, padrao, nome },
            Geometria::Ponto { p } => GeometriaJson::Ponto { p },
            Geometria::Circulo { centro, raio } => GeometriaJson::Circulo { centro, raio },
            Geometria::Arco { centro, raio, inicio, fim } => GeometriaJson::Arco { centro, raio, inicio, fim },
            Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => GeometriaJson::Texto { ponto, altura, rotacao, conteudo, h, v },
        }
    }
}

impl From<GeometriaJson> for Geometria {
    fn from(g: GeometriaJson) -> Self {
        match g {
            GeometriaJson::Linha { a, b } => Geometria::Linha { a, b },
            GeometriaJson::Polilinha { vertices, fechada } => Geometria::Polilinha { vertices, fechada },
            GeometriaJson::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao },
            GeometriaJson::Hachura { contornos, padrao, nome } => Geometria::Hachura { contornos, padrao, nome },
            GeometriaJson::Ponto { p } => Geometria::Ponto { p },
            GeometriaJson::Circulo { centro, raio } => Geometria::Circulo { centro, raio },
            GeometriaJson::Arco { centro, raio, inicio, fim } => Geometria::Arco { centro, raio, inicio, fim },
            GeometriaJson::Texto { ponto, altura, rotacao, conteudo, h, v } => Geometria::Texto { ponto, altura, rotacao, conteudo, h, v },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EntidadeJson {
    camada: usize,
    cor: Option<u32>,
    #[serde(default)]
    props: Propriedades,
    #[serde(flatten)]
    geometria: GeometriaJson,
}

#[derive(Serialize, Deserialize)]
struct BlocoJson {
    nome: String,
    base: Ponto,
    entidades: Vec<EntidadeJson>,
}

#[derive(Serialize, Deserialize)]
struct DesenhoJson {
    camadas: Vec<CamadaJson>,
    camada_atual: usize,
    /// `mm`, `cm`, `m`; ausente em arquivo sem unidade.
    #[serde(default)]
    unidade: Option<String>,
    #[serde(default)]
    blocos: Vec<BlocoJson>,
    entidades: Vec<EntidadeJson>,
}

fn nova_para_json(n: &NovaEntidade) -> EntidadeJson {
    EntidadeJson { camada: n.camada, cor: n.cor, props: n.props.clone(), geometria: (&n.geometria).into() }
}

fn para_json(doc: &Documento) -> DesenhoJson {
    DesenhoJson {
        unidade: doc.unidade().map(|u| u.nome().to_string()),
        blocos: doc.blocos().iter().map(|b| BlocoJson { nome: b.nome.clone(), base: b.base, entidades: b.entidades.iter().map(nova_para_json).collect() }).collect(),
        camadas: doc.camadas().iter().map(|c| CamadaJson { nome: c.nome.clone(), cor: c.cor, ligada: c.ligada, congelada: c.congelada, bloqueada: c.bloqueada, plotar: c.plotar, tipo_linha: c.tipo_linha, espessura: c.espessura, transparencia: c.transparencia }).collect(),
        camada_atual: doc.camada_atual(),
        entidades: doc
            .entidades()
            .iter()
            .map(|e| EntidadeJson { camada: e.camada, cor: e.cor, props: e.props.clone(), geometria: (&e.geometria).into() })
            .collect(),
    }
}

/// Grava o documento como `.fcdz`.
pub fn gravar(doc: &Documento) -> Result<Vec<u8>, String> {
    let erro = |e: &dyn std::fmt::Display| format!("Não consegui montar o .fcdz: {e}");
    let manifesto = serde_json::to_vec_pretty(&Manifesto { aplicativo: "feathercad".into(), formato: VERSAO }).map_err(|e| erro(&e))?;
    let desenho = serde_json::to_vec(&para_json(doc)).map_err(|e| erro(&e))?;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nome, dados) in [("manifest.json", manifesto), ("data/desenho.json", desenho)] {
        zip.start_file(nome, opcoes).map_err(|e| erro(&e))?;
        zip.write_all(&dados).map_err(|e| erro(&e))?;
    }
    Ok(zip.finish().map_err(|e| erro(&e))?.into_inner())
}

fn ler_arquivo(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, nome: &str) -> Result<Vec<u8>, String> {
    let mut f = zip.by_name(nome).map_err(|_| format!("O .fcdz está incompleto: falta \"{nome}\"."))?;
    let mut dados = Vec::new();
    f.read_to_end(&mut dados).map_err(|e| format!("Não consegui ler \"{nome}\" do .fcdz: {e}"))?;
    Ok(dados)
}

/// Lê um `.fcdz`. Arquivo malformado dá erro com mensagem, nunca pânico.
pub fn ler(bytes: &[u8]) -> Result<Aberto, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Isto não é um .fcdz válido: {e}"))?;
    let manifesto: Manifesto = serde_json::from_slice(&ler_arquivo(&mut zip, "manifest.json")?).map_err(|e| format!("manifest.json ilegível: {e}"))?;
    if manifesto.aplicativo != "feathercad" || manifesto.formato > VERSAO {
        return Err(format!("Este .fcdz é de outro programa ou de uma versão mais nova (formato {}).", manifesto.formato));
    }
    let d: DesenhoJson = serde_json::from_slice(&ler_arquivo(&mut zip, "data/desenho.json")?).map_err(|e| format!("desenho.json ilegível: {e}"))?;
    let camadas = d.camadas.into_iter().map(|c| Camada { ligada: c.ligada, congelada: c.congelada, bloqueada: c.bloqueada, plotar: c.plotar, tipo_linha: c.tipo_linha, espessura: c.espessura, transparencia: c.transparencia.min(90), ..Camada::nova(c.nome, c.cor) }).collect();
    let nova = |e: EntidadeJson| NovaEntidade { camada: e.camada, cor: e.cor, geometria: e.geometria.into(), props: e.props };
    let blocos = d.blocos.into_iter().map(|b| Bloco { nome: b.nome, base: b.base, entidades: b.entidades.into_iter().map(nova).collect() }).collect();
    let entidades = d.entidades.into_iter().map(nova).collect();
    Ok(Aberto { doc: Documento::de_partes_com_blocos(camadas, d.camada_atual, blocos, entidades).com_unidade(d.unidade.as_deref().and_then(Unidade::de_nome)), avisos: Vec::new(), versao_dxf: None })
}
