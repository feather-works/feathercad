//! Gravação de DXF R12 (AC1009): sem handles, referências por nome. É o modelo
//! que qualquer programa de CAD lê. Ver `mod.rs` sobre o R2000+.

use std::fmt::Write;

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria, TipoLinha};
use feathercad_core::geom::{triangular, Vertice};
use feathercad_core::quadro::{lacos_da_hachura, tracos_do_padrao};
use feathercad_core::sessao::Mensagem;

use super::codificar_u;
use crate::aci::{aci_para_rgb, rgb_para_aci};

/// Uma tag: código e valor, cada um na sua linha.
fn tag(s: &mut String, codigo: i32, valor: impl std::fmt::Display) {
    let _ = write!(s, "{codigo}\n{valor}\n");
}

/// O ACI de uma cor RGB e se ela foi aproximada (o R12 só tem ACI).
fn aci_de(rgb: u32, aproximadas: &mut usize) -> u16 {
    let aci = rgb_para_aci(rgb);
    if aci_para_rgb(aci) != rgb {
        *aproximadas += 1;
    }
    aci
}

fn tabela_de_camadas(doc: &Documento, s: &mut String, aproximadas: &mut usize) {
    tag(s, 0, "TABLE");
    tag(s, 2, "LAYER");
    tag(s, 70, doc.camadas().len());
    for c in doc.camadas() {
        tag(s, 0, "LAYER");
        tag(s, 2, codificar_u(&c.nome));
        // Bits do R12: 1 = congelada, 4 = bloqueada. (Plotar, espessura e transparência o R12 não tem: só o `.fcdz` guarda.)
        tag(s, 70, i32::from(c.congelada) + 4 * i32::from(c.bloqueada));
        let aci = i32::from(aci_de(c.cor, aproximadas));
        tag(s, 62, if c.ligada { aci } else { -aci });
        tag(s, 6, c.tipo_linha.nome_dxf());
    }
    tag(s, 0, "ENDTAB");
}

/// A tabela LTYPE do R12: os tipos de linha conhecidos, com o padrão de traço e vão (em unidades do desenho).
fn tabela_de_tipos_de_linha(s: &mut String) {
    tag(s, 0, "TABLE");
    tag(s, 2, "LTYPE");
    tag(s, 70, TipoLinha::TODOS.len());
    for t in TipoLinha::TODOS {
        let padrao: &[f64] = match t {
            TipoLinha::Continua => &[],
            TipoLinha::Tracejada => &[0.5, -0.25],
            TipoLinha::Oculta => &[0.25, -0.125],
            TipoLinha::Eixo => &[1.25, -0.25, 0.25, -0.25],
            TipoLinha::Pontilhada => &[0.0, -0.25],
            TipoLinha::Fantasma => &[1.25, -0.25, 0.25, -0.25, 0.25, -0.25],
        };
        tag(s, 0, "LTYPE");
        tag(s, 2, t.nome_dxf());
        tag(s, 70, 0);
        tag(s, 3, if padrao.is_empty() { "Solid line" } else { t.nome_dxf() });
        tag(s, 72, 65);
        tag(s, 73, padrao.len());
        tag(s, 40, padrao.iter().map(|v| v.abs()).sum::<f64>());
        for v in padrao {
            tag(s, 49, v);
        }
    }
    tag(s, 0, "ENDTAB");
}

fn cabecalho_e_tabelas(doc: &Documento, s: &mut String, aproximadas: &mut usize) {
    for (c, v) in [(0, "SECTION"), (2, "HEADER"), (9, "$ACADVER"), (1, "AC1009")] {
        tag(s, c, v);
    }
    // O R12 não define $INSUNITS, mas leitor que não o conhece o ignora; quem o conhece (AutoCAD, ezdxf) usa.
    if let Some(u) = doc.unidade() {
        tag(s, 9, "$INSUNITS");
        tag(s, 70, u.codigo_dxf());
    }
    for (c, v) in [(0, "ENDSEC"), (0, "SECTION"), (2, "TABLES")] {
        tag(s, c, v);
    }
    tabela_de_tipos_de_linha(s);
    tabela_de_camadas(doc, s, aproximadas);
    tag(s, 0, "ENDSEC");
}

/// O que toda entidade grava além da forma (do desenho ou de dentro de um bloco).
struct Peca {
    camada: usize,
    cor: Option<u32>,
    tipo_linha: Option<TipoLinha>,
}

/// O começo comum de toda entidade: tipo, camada e cor própria (se houver).
fn cabeca(doc: &Documento, e: &Peca, tipo: &str, s: &mut String, aproximadas: &mut usize) {
    tag(s, 0, tipo);
    tag(s, 8, codificar_u(doc.camadas().get(e.camada).map_or("0", |c| c.nome.as_str())));
    if let Some(t) = e.tipo_linha.filter(|t| *t != TipoLinha::Continua) {
        tag(s, 6, t.nome_dxf());
    }
    if let Some(cor) = e.cor {
        tag(s, 62, aci_de(cor, aproximadas));
    }
}

fn ponto3(s: &mut String, base: i32, x: f64, y: f64) {
    tag(s, base, x);
    tag(s, base + 10, y);
    tag(s, base + 20, 0);
}

/// R12 não tem `LWPOLYLINE`: a polilinha vai como `POLYLINE` + `VERTEX`… + `SEQEND`.
fn polilinha(doc: &Documento, e: &Peca, vertices: &[Vertice], fechada: bool, s: &mut String, aproximadas: &mut usize) {
    cabeca(doc, e, "POLYLINE", s, aproximadas);
    tag(s, 66, 1);
    ponto3(s, 10, 0.0, 0.0);
    tag(s, 70, i32::from(fechada));
    let camada = codificar_u(doc.camadas().get(e.camada).map_or("0", |c| c.nome.as_str()));
    for v in vertices {
        tag(s, 0, "VERTEX");
        tag(s, 8, &camada);
        ponto3(s, 10, v.p.x, v.p.y);
        if v.bulge != 0.0 {
            tag(s, 42, v.bulge);
        }
    }
    tag(s, 0, "SEQEND");
    tag(s, 8, camada);
}

fn texto(doc: &Documento, e: &Peca, g: &Geometria, s: &mut String, aproximadas: &mut usize) {
    let Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } = g else { return };
    cabeca(doc, e, "TEXT", s, aproximadas);
    ponto3(s, 10, ponto.x, ponto.y);
    tag(s, 40, altura);
    tag(s, 1, codificar_u(&conteudo.replace(['\n', '\r'], " ")));
    if *rotacao != 0.0 {
        tag(s, 50, rotacao);
    }
    let (ch, cv) = (i32::from(*h != AlinhaH::Esquerda) + i32::from(*h == AlinhaH::Direita), match v {
        AlinhaV::Base => 0,
        AlinhaV::Baixo => 1,
        AlinhaV::Meio => 2,
        AlinhaV::Topo => 3,
    });
    if ch != 0 {
        tag(s, 72, ch);
    }
    if ch != 0 || cv != 0 {
        // Com alinhamento, o ponto que vale é o 11: repete o de inserção.
        ponto3(s, 11, ponto.x, ponto.y);
    }
    if cv != 0 {
        tag(s, 73, cv);
    }
}

/// O que aconteceu com uma hachura ao gravar: o R12 não tem `HATCH`.
#[derive(Clone, Copy, PartialEq)]
enum Hachurada {
    /// Não era hachura.
    Nao,
    /// Gravada como `SOLID` (área cheia, em triângulos) ou `LINE` (padrão).
    Decomposta,
    /// Não deu: área cheia com ilha, ou traços demais.
    Perdida,
}

/// O que se conta ao gravar, para virar avisos.
#[derive(Default)]
struct Contas {
    aproximadas: usize,
    decompostas: usize,
    perdidas: usize,
}

impl Contas {
    fn somar(&mut self, h: Hachurada) {
        match h {
            Hachurada::Nao => {}
            Hachurada::Decomposta => self.decompostas += 1,
            Hachurada::Perdida => self.perdidas += 1,
        }
    }
}

/// Uma hachura como entidades que o R12 conhece: triângulos `SOLID` (cheia) ou `LINE` (padrão).
fn hachura(doc: &Documento, e: &Peca, contornos: &[Vec<Vertice>], padrao: &[feathercad_core::documento::LinhaDePadrao], s: &mut String, aproximadas: &mut usize) -> Hachurada {
    let (mut min, mut max) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for v in contornos.iter().flatten() {
        min = (min.0.min(v.p.x), min.1.min(v.p.y));
        max = (max.0.max(v.p.x), max.1.max(v.p.y));
    }
    // Arcos por bulge viram cordas a ~1/2000 do tamanho da própria hachura (a maior extensão, X ou Y).
    let tamanho = (max.0 - min.0).max(max.1 - min.1).max(0.0);
    let lacos = lacos_da_hachura(contornos, (tamanho / 2000.0).max(1e-9));
    if padrao.is_empty() {
        let [laco] = lacos.as_slice() else { return Hachurada::Perdida };
        let Some(triangulos) = triangular(laco) else { return Hachurada::Perdida };
        for [a, b, c] in triangulos {
            cabeca(doc, e, "SOLID", s, aproximadas);
            for (i, p) in [a, b, c, c].into_iter().enumerate() {
                ponto3(s, 10 + i as i32, p.x, p.y);
            }
        }
    } else {
        let Some(tracos) = tracos_do_padrao(&lacos, padrao) else { return Hachurada::Perdida };
        for (a, b) in tracos {
            cabeca(doc, e, "LINE", s, aproximadas);
            ponto3(s, 10, a.x, a.y);
            ponto3(s, 11, b.x, b.y);
        }
    }
    Hachurada::Decomposta
}

/// Uma entidade (qualquer geometria), do desenho ou de dentro de um bloco.
fn peca(doc: &Documento, e: &Peca, g: &Geometria, s: &mut String, aproximadas: &mut usize) -> Hachurada {
    if let Geometria::Hachura { contornos, padrao, .. } = g {
        return hachura(doc, e, contornos, padrao, s, aproximadas);
    }
    forma(doc, e, g, s, aproximadas);
    Hachurada::Nao
}

/// As entidades que não são hachura.
fn forma(doc: &Documento, e: &Peca, g: &Geometria, s: &mut String, aproximadas: &mut usize) {
    match g {
        Geometria::Linha { a, b } => {
            cabeca(doc, e, "LINE", s, aproximadas);
            ponto3(s, 10, a.x, a.y);
            ponto3(s, 11, b.x, b.y);
        }
        Geometria::Polilinha { vertices, fechada } => polilinha(doc, e, vertices, *fechada, s, aproximadas),
        Geometria::Circulo { centro, raio } => {
            cabeca(doc, e, "CIRCLE", s, aproximadas);
            ponto3(s, 10, centro.x, centro.y);
            tag(s, 40, raio);
        }
        Geometria::Arco { centro, raio, inicio, fim } => {
            cabeca(doc, e, "ARC", s, aproximadas);
            ponto3(s, 10, centro.x, centro.y);
            tag(s, 40, raio);
            tag(s, 50, inicio);
            tag(s, 51, fim);
        }
        g @ Geometria::Texto { .. } => texto(doc, e, g, s, aproximadas),
        Geometria::Ponto { p } => {
            cabeca(doc, e, "POINT", s, aproximadas);
            ponto3(s, 10, p.x, p.y);
        }
        Geometria::Hachura { .. } => {}
        Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => {
            // Bloco que não existe (índice solto) não gera INSERT: o arquivo ficaria inválido.
            let Some(b) = doc.blocos().get(*bloco) else { return };
            cabeca(doc, e, "INSERT", s, aproximadas);
            tag(s, 2, codificar_u(&b.nome));
            ponto3(s, 10, ponto.x, ponto.y);
            tag(s, 41, escala_x);
            tag(s, 42, escala_y);
            tag(s, 43, 1);
            if *rotacao != 0.0 {
                tag(s, 50, rotacao);
            }
        }
    }
}

/// A seção BLOCKS (antes de ENTITIES): cada bloco com o ponto-base e suas entidades.
fn blocos(doc: &Documento, s: &mut String, c: &mut Contas) {
    tag(s, 0, "SECTION");
    tag(s, 2, "BLOCKS");
    for b in doc.blocos() {
        let nome = codificar_u(&b.nome);
        tag(s, 0, "BLOCK");
        tag(s, 8, 0);
        tag(s, 2, &nome);
        tag(s, 70, 0);
        ponto3(s, 10, b.base.x, b.base.y);
        tag(s, 3, &nome);
        for n in &b.entidades {
            let h = peca(doc, &Peca { camada: n.camada, cor: n.cor, tipo_linha: n.props.tipo_linha }, &n.geometria, s, &mut c.aproximadas);
            c.somar(h);
        }
        tag(s, 0, "ENDBLK");
        tag(s, 8, 0);
    }
    tag(s, 0, "ENDSEC");
}

fn entidades(doc: &Documento, s: &mut String, c: &mut Contas) {
    tag(s, 0, "SECTION");
    tag(s, 2, "ENTITIES");
    for e in doc.entidades() {
        let h = peca(doc, &Peca { camada: e.camada, cor: e.cor, tipo_linha: e.props.tipo_linha }, &e.geometria, s, &mut c.aproximadas);
        c.somar(h);
    }
    tag(s, 0, "ENDSEC");
    tag(s, 0, "EOF");
}

/// Grava o documento em DXF R12. Devolve os bytes e os avisos do que foi simplificado.
pub fn gravar(doc: &Documento) -> (Vec<u8>, Vec<Mensagem>) {
    let mut s = String::new();
    let mut c = Contas::default();
    cabecalho_e_tabelas(doc, &mut s, &mut c.aproximadas);
    blocos(doc, &mut s, &mut c);
    entidades(doc, &mut s, &mut c);
    let mut avisos = Vec::new();
    if c.decompostas > 0 {
        avisos.push(Mensagem::nova("aviso.hachuraDecomposta").com("n", &c.decompostas.to_string()));
    }
    if c.perdidas > 0 {
        avisos.push(Mensagem::nova("aviso.hachuraNaoGravada").com("n", &c.perdidas.to_string()));
    }
    if c.aproximadas > 0 {
        avisos.push(Mensagem::nova("aviso.corAproximada").com("n", &c.aproximadas.to_string()));
    }
    (s.into_bytes(), avisos)
}
