//! O **formato compacto** (`.fct`): o desenho em texto puro, uma linha por objeto, feito para gastar poucos
//! tokens (um DXF de verdade repete códigos de grupo em cada número e passa fácil de 10 vezes o tamanho).
//! Serve nos dois sentidos: a IA lê o desenho (`cad_exportar`) e escreve desenho (`cad_importar`), e o
//! conversor grava/abre `.fct` no disco como qualquer outro formato.
//!
//! ```text
//! unidade mm
//! camada Paredes #FF0000
//! R 0 0 400 300              retângulo: canto, largura, altura [giro]
//! L 0 0 100 50               linha
//! P 0 0 100 0 ~0.5 100 50 f  polilinha (~b = arco até o próximo ponto; f = fechada)
//! C 50 25 10                 círculo: centro, raio
//! A 50 25 10 0 90            arco: centro, raio, ângulo inicial e final
//! T 10 10 2.5 0 "Sala" cm    texto: ponto, altura, giro, conteúdo [alinhamento: e/c/d na horizontal, b/m/t/f
//!                                           (base, meio, topo, fundo) na vertical; ex. `c`, `dm`; sem sufixo = esquerda/base]
//! . 5 5                      ponto
//! H solida 0 0 10 0 10 10 | 2 2 4 2 4 4   hachura (laços separados por |); em vez de `solida`: `padrao` (45°) ou um da
//!                                           biblioteca — concreto, areia, brita, terra, tijolo, ansi32, ansi37, horizontal,
//!                                           vertical, grade, pontos — e `terra*2` dobra o espaço do padrão
//! ```
//! Uma linha pode começar por `@id` (o id que a IA cita) e terminar por `#RRGGBB` (cor própria).
//! `camada NOME [#cor] [off] [congelada] [bloqueada] [esp=0.35] [linha=eixo] [transp=30]` define a camada e a usa
//! daí em diante (linha: continua, tracejada, oculta, eixo, pontilhada, fantasma).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use feathercad_core::documento::{AlinhaH, AlinhaV, Camada, Documento, Geometria, Id, TipoLinha};
use feathercad_core::edicao::transformar;
use feathercad_core::geom::{Ponto, Transf, Vertice};

use crate::imagem::numero;
use crate::json::Res;

/// Blocos aninhados além disto não são abertos (proteção contra bloco que se contém).
const PROFUNDIDADE: u8 = 8;

/// O que uma linha `camada` declara.
pub struct CamadaLida {
    pub nome: String,
    /// `None` = a linha não deu cor: fica a que a camada já tem (a de uma declaração anterior, ou a de fábrica).
    pub cor: Option<u32>,
    pub ligada: bool,
    pub congelada: bool,
    pub bloqueada: bool,
    /// Espessura em mm (`esp=`); `None` = a linha não disse.
    pub espessura: Option<f32>,
    pub tipo_linha: Option<TipoLinha>,
    pub transparencia: Option<u8>,
}

/// O que se leu de um texto compacto.
#[derive(Default)]
pub struct Lido {
    pub unidade: Option<String>,
    pub camadas: Vec<CamadaLida>,
    /// Cada forma com o nome da camada e a cor própria (se houver).
    pub formas: Vec<(String, Option<u32>, Geometria)>,
}

// ---------------------------------------------------------------- escrever

fn pt(p: Ponto) -> String {
    format!("{} {}", numero(p.x), numero(p.y))
}

/// Uma polilinha reta de 4 vértices em ângulo reto vira `R` (mais curta).
fn como_retangulo(v: &[Vertice], fechada: bool) -> Option<String> {
    if !fechada || v.len() != 4 || v.iter().any(|x| x.bulge != 0.0) {
        return None;
    }
    let (a, b, c, d) = (v[0].p, v[1].p, v[2].p, v[3].p);
    let (l, h) = (a.distancia(b), b.distancia(c));
    let reto = ((b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y)).abs() < 1e-9 * l.max(h).powi(2).max(1.0);
    let paralelogramo = (a.x + c.x - b.x - d.x).abs() < 1e-9 * l.max(h).max(1.0) && (a.y + c.y - b.y - d.y).abs() < 1e-9 * l.max(h).max(1.0);
    if !(reto && paralelogramo) || l < 1e-12 || h < 1e-12 {
        return None;
    }
    let giro = a.angulo_ate(b);
    // O retângulo `R` cresce para a esquerda do lado a→b; se o desenho anda no outro sentido, não é `R`.
    let esquerda = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x) > 0.0;
    esquerda.then(|| if giro.abs() < 1e-9 { format!("R {} {} {}", pt(a), numero(l), numero(h)) } else { format!("R {} {} {} {}", pt(a), numero(l), numero(h), numero(giro)) })
}

fn como_polilinha(v: &[Vertice], fechada: bool) -> String {
    let mut s = String::from("P");
    for (i, x) in v.iter().enumerate() {
        let _ = write!(s, " {}", pt(x.p));
        if x.bulge != 0.0 && (fechada || i + 1 < v.len()) {
            let _ = write!(s, " ~{}", numero(x.bulge));
        }
    }
    if fechada {
        s.push_str(" f");
    }
    s
}

fn texto_entre_aspas(t: &str) -> String {
    format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// ` c`, ` dm`… (vazio quando é esquerda/base, o padrão).
fn sufixo_de_alinhamento(h: AlinhaH, v: AlinhaV) -> String {
    let h = match h {
        AlinhaH::Esquerda => "",
        AlinhaH::Centro => "c",
        AlinhaH::Direita => "d",
    };
    let v = match v {
        AlinhaV::Base => "",
        AlinhaV::Baixo => "f",
        AlinhaV::Meio => "m",
        AlinhaV::Topo => "t",
    };
    if h.is_empty() && v.is_empty() { String::new() } else { format!(" {h}{v}") }
}

/// `c`, `dm`, `et`… → alinhamento; `None` se a palavra não é um sufixo de alinhamento.
fn alinhamento_de(p: &str) -> Option<(AlinhaH, AlinhaV)> {
    if p.is_empty() || p.len() > 2 {
        return None;
    }
    let (mut h, mut v, mut tem_h, mut tem_v) = (AlinhaH::Esquerda, AlinhaV::Base, false, false);
    for ch in p.chars() {
        match ch {
            'e' | 'c' | 'd' if !tem_h => {
                h = match ch {
                    'e' => AlinhaH::Esquerda,
                    'c' => AlinhaH::Centro,
                    _ => AlinhaH::Direita,
                };
                tem_h = true;
            }
            'b' | 'f' | 'm' | 't' if !tem_v => {
                v = match ch {
                    'b' => AlinhaV::Base,
                    'f' => AlinhaV::Baixo,
                    'm' => AlinhaV::Meio,
                    _ => AlinhaV::Topo,
                };
                tem_v = true;
            }
            _ => return None,
        }
    }
    Some((h, v))
}

fn como_laco(l: &[Vertice]) -> String {
    l.iter().map(|v| pt(v.p)).collect::<Vec<_>>().join(" ")
}

/// A linha de texto de uma forma (sem id nem cor), ou `None` se ela não tem representação.
fn linha_da_forma(g: &Geometria) -> Option<String> {
    Some(match g {
        Geometria::Linha { a, b } => format!("L {} {}", pt(*a), pt(*b)),
        Geometria::Polilinha { vertices, fechada } => como_retangulo(vertices, *fechada).unwrap_or_else(|| como_polilinha(vertices, *fechada)),
        Geometria::Circulo { centro, raio } => format!("C {} {}", pt(*centro), numero(*raio)),
        Geometria::Arco { centro, raio, inicio, fim } => format!("A {} {} {} {}", pt(*centro), numero(*raio), numero(*inicio), numero(*fim)),
        Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => format!("T {} {} {} {}{}", pt(*ponto), numero(*altura), numero(*rotacao), texto_entre_aspas(conteudo), sufixo_de_alinhamento(*h, *v)),
        Geometria::Ponto { p } => format!(". {}", pt(*p)),
        Geometria::Hachura { contornos, padrao, nome } => format!("H {} {}", if padrao.is_empty() { "solida".to_string() } else if feathercad_core::padroes::reconhecer(nome).is_some() { nome.to_lowercase() } else { "padrao".to_string() }, contornos.iter().map(|c| como_laco(c)).collect::<Vec<_>>().join(" | ")),
        Geometria::Face3D { p1, p2, p3, p4 } => format!("F3D {} {} {} {}", pt(*p1), pt(*p2), pt(*p3), pt(*p4)),
        Geometria::Insercao { .. } | Geometria::Imagem { .. } => return None,
    })
}

/// O que a exportação fez, para o relatório.
#[derive(Default)]
pub struct Estatistica {
    pub objetos: usize,
    pub blocos_abertos: usize,
    pub ignorados: usize,
}

/// Uma forma pronta para sair: camada, cor própria, id (0 = de dentro de bloco) e a geometria.
type Saida = (usize, Option<u32>, Id, Geometria);

/// Abre as inserções de bloco em formas soltas (o compacto não tem blocos), com a transformação acumulada.
fn abrir_blocos(doc: &Documento, e: &(usize, Option<u32>, Id, Geometria), t: &Transf, nivel: u8, fora: &mut Vec<Saida>, est: &mut Estatistica) {
    let (camada, cor, id, g) = e;
    let Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } = g else {
        fora.push((*camada, *cor, *id, transformar(g, t)));
        return;
    };
    let Some(b) = doc.blocos().get(*bloco).filter(|_| nivel < PROFUNDIDADE) else {
        est.ignorados += 1;
        return;
    };
    est.blocos_abertos += 1;
    let filha = t.depois_de(&Transf::de_insercao(b.base, *ponto, *escala_x, *escala_y, *rotacao));
    for n in &b.entidades {
        let c = if n.camada == 0 { *camada } else { n.camada };
        abrir_blocos(doc, &(c, n.cor.or(*cor), 0, n.geometria.clone()), &filha, nivel + 1, fora, est);
    }
}

/// Todas as formas do desenho (ou só `ids`) com os blocos abertos em formas soltas: camada, cor própria, id e geometria.
pub fn formas_abertas(doc: &Documento, ids: Option<&BTreeSet<Id>>) -> Vec<(usize, Option<u32>, Id, Geometria)> {
    let mut est = Estatistica::default();
    let mut formas: Vec<Saida> = Vec::new();
    for e in doc.entidades().iter().filter(|e| ids.map_or(true, |s| s.contains(&e.id))) {
        abrir_blocos(doc, &(e.camada, e.cor, e.id, e.geometria.clone()), &Transf::IDENTIDADE, 0, &mut formas, &mut est);
    }
    formas
}

/// O desenho em texto compacto. `ids`: só esses objetos (ou todos); `com_ids`: escreve `@id` em cada linha.
pub fn exportar(doc: &Documento, ids: Option<&BTreeSet<Id>>, com_ids: bool) -> (String, Estatistica) {
    let mut est = Estatistica::default();
    let mut formas: Vec<Saida> = Vec::new();
    for e in doc.entidades().iter().filter(|e| ids.map_or(true, |s| s.contains(&e.id))) {
        abrir_blocos(doc, &(e.camada, e.cor, e.id, e.geometria.clone()), &Transf::IDENTIDADE, 0, &mut formas, &mut est);
    }
    let mut s = String::new();
    if let Some(u) = doc.unidade() {
        let _ = writeln!(s, "unidade {}", u.nome());
    }
    let mut usadas: Vec<usize> = formas.iter().map(|f| f.0).collect();
    usadas.sort_unstable();
    usadas.dedup();
    for c in usadas {
        let Some(camada) = doc.camadas().get(c) else { continue };
        s.push_str(&declaracao(camada));
        for (_, cor, id, g) in formas.iter().filter(|f| f.0 == c) {
            let Some(l) = linha_da_forma(g) else {
                est.ignorados += 1;
                continue;
            };
            est.objetos += 1;
            if com_ids && *id != 0 {
                let _ = write!(s, "@{id} ");
            }
            s.push_str(&l);
            if let Some(cor) = cor {
                let _ = write!(s, " #{cor:06X}");
            }
            s.push('\n');
        }
    }
    (s, est)
}

fn declaracao(c: &Camada) -> String {
    let mut s = format!("camada {} #{:06X}", if c.nome.contains(' ') { texto_entre_aspas(&c.nome) } else { c.nome.clone() }, c.cor);
    for (ativo, nome) in [(!c.ligada, "off"), (c.congelada, "congelada"), (c.bloqueada, "bloqueada")] {
        if ativo {
            s.push(' ');
            s.push_str(nome);
        }
    }
    if let Some(e) = c.espessura {
        let _ = write!(s, " esp={}", numero(f64::from(e)));
    }
    if c.tipo_linha != TipoLinha::Continua {
        let _ = write!(s, " linha={}", nome_do_tipo(c.tipo_linha));
    }
    if c.transparencia > 0 {
        let _ = write!(s, " transp={}", c.transparencia);
    }
    s.push('\n');
    s
}

fn nome_do_tipo(t: TipoLinha) -> &'static str {
    match t {
        TipoLinha::Continua => "continua",
        TipoLinha::Tracejada => "tracejada",
        TipoLinha::Oculta => "oculta",
        TipoLinha::Eixo => "eixo",
        TipoLinha::Pontilhada => "pontilhada",
        TipoLinha::Fantasma => "fantasma",
    }
}

// ------------------------------------------------------------------- ler

/// Divide uma linha em palavras, respeitando `"texto com espaços"` (com `\"`, `\\` e `\n`).
fn palavras(linha: &str) -> Res<Vec<String>> {
    let mut fora = Vec::new();
    let mut atual = String::new();
    let (mut em_aspas, mut escapou, mut tinha) = (false, false, false);
    for ch in linha.chars() {
        match (em_aspas, escapou, ch) {
            (true, true, 'n') => atual.push('\n'),
            (true, true, c) => atual.push(c),
            (true, false, '\\') => {
                escapou = true;
                continue;
            }
            (true, false, '"') => em_aspas = false,
            (true, false, c) => atual.push(c),
            (false, _, '"') => {
                em_aspas = true;
                tinha = true;
            }
            (false, _, c) if c.is_whitespace() => {
                if !atual.is_empty() || tinha {
                    fora.push(std::mem::take(&mut atual));
                    tinha = false;
                }
            }
            (false, _, c) => atual.push(c),
        }
        escapou = false;
    }
    if em_aspas {
        return Err("faltou fechar as aspas".into());
    }
    if !atual.is_empty() || tinha {
        fora.push(atual);
    }
    Ok(fora)
}

fn num(p: &str) -> Res<f64> {
    p.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or_else(|| format!("`{p}` não é número"))
}

fn nums(p: &[String]) -> Res<Vec<f64>> {
    p.iter().map(|s| num(s)).collect()
}

fn cor_de(p: &str) -> Option<u32> {
    let h = p.strip_prefix('#')?;
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten()
}

fn lista_de_vertices(p: &[String]) -> Res<Vec<Vertice>> {
    let mut v: Vec<Vertice> = Vec::new();
    let mut i = 0;
    while i < p.len() {
        if let Some(b) = p[i].strip_prefix('~') {
            v.last_mut().ok_or("`~` sem ponto antes")?.bulge = num(b)?;
            i += 1;
        } else {
            let (x, y) = (num(&p[i])?, num(p.get(i + 1).ok_or("ponto sem o Y")?)?);
            v.push(Vertice { p: Ponto::novo(x, y), bulge: 0.0 });
            i += 2;
        }
    }
    Ok(v)
}

fn retangulo(n: &[f64]) -> Res<Geometria> {
    let (x, y, l, h) = (n[0], n[1], n[2], n[3]);
    let giro = n.get(4).copied().unwrap_or(0.0).to_radians();
    let (s, c) = giro.sin_cos();
    let canto = |dx: f64, dy: f64| Vertice { p: Ponto::novo(x + dx * c - dy * s, y + dx * s + dy * c), bulge: 0.0 };
    Ok(Geometria::Polilinha { vertices: vec![canto(0.0, 0.0), canto(l, 0.0), canto(l, h), canto(0.0, h)], fechada: true })
}

fn hachura(p: &[String]) -> Res<Geometria> {
    // `solida`, `padrao` (45°) ou o nome de um padrão da biblioteca (`concreto`, `terra`, `tijolo`…); vírgula-escala: `terra*2`
    let primeiro = p.first().map(String::as_str).unwrap_or("padrao");
    let (nome_pedido, escala) = match primeiro.split_once('*') {
        Some((n, e)) => (n, e.replace(',', ".").parse::<f64>().map_err(|_| format!("hachura: escala `{e}` inválida"))?),
        None => (primeiro, 1.0),
    };
    let nome = if nome_pedido.eq_ignore_ascii_case("solida") { feathercad_core::padroes::SOLIDA } else if nome_pedido.eq_ignore_ascii_case("padrao") { feathercad_core::padroes::PADRAO_DE_FABRICA } else { feathercad_core::padroes::reconhecer(nome_pedido).ok_or_else(|| format!("hachura: padrão `{nome_pedido}` não existe (solida, padrao ou {})", feathercad_core::padroes::NOMES.join(", ").to_lowercase()))? };
    let corpo = p.get(1..).unwrap_or(&[]).join(" ");
    let contornos: Vec<Vec<Vertice>> = corpo.split('|').map(|l| lista_de_vertices(&l.split_whitespace().map(str::to_string).collect::<Vec<_>>())).collect::<Res<_>>()?;
    if contornos.iter().any(|c| c.len() < 3) {
        return Err("hachura: cada laço precisa de ao menos 3 pontos".into());
    }
    let (padrao, nome) = feathercad_core::padroes::para_contornos(nome, escala, &contornos);
    Ok(Geometria::Hachura { contornos, padrao, nome })
}

/// Uma forma a partir da letra e dos argumentos (já sem `@id` nem `#cor`).
fn forma(letra: &str, p: &[String]) -> Res<Geometria> {
    let precisa = |n: usize| if p.len() >= n { Ok(()) } else { Err(format!("`{letra}` precisa de {n} valores")) };
    match letra {
        "L" => {
            precisa(4)?;
            let n = nums(&p[..4])?;
            Ok(Geometria::Linha { a: Ponto::novo(n[0], n[1]), b: Ponto::novo(n[2], n[3]) })
        }
        "R" => {
            precisa(4)?;
            retangulo(&nums(p)?)
        }
        "C" => {
            precisa(3)?;
            let n = nums(&p[..3])?;
            Ok(Geometria::Circulo { centro: Ponto::novo(n[0], n[1]), raio: n[2] })
        }
        "A" => {
            precisa(5)?;
            let n = nums(&p[..5])?;
            Ok(Geometria::Arco { centro: Ponto::novo(n[0], n[1]), raio: n[2], inicio: n[3], fim: n[4] })
        }
        "T" => {
            precisa(5)?;
            let n = nums(&p[..4])?;
            let (h, v) = match p.get(5) {
                Some(w) => alinhamento_de(w).ok_or_else(|| format!("alinhamento `{w}` inválido (e/c/d + b/m/t/f, ex.: `c`, `dm`)"))?,
                None => (AlinhaH::Esquerda, AlinhaV::Base),
            };
            Ok(Geometria::Texto { ponto: Ponto::novo(n[0], n[1]), altura: n[2], rotacao: n[3], conteudo: p[4].clone(), h, v })
        }
        "." => {
            precisa(2)?;
            Ok(Geometria::Ponto { p: Ponto::novo(num(&p[0])?, num(&p[1])?) })
        }
        "P" => {
            let fechada = p.last().map(String::as_str) == Some("f");
            let v = lista_de_vertices(&p[..p.len() - usize::from(fechada)])?;
            if v.len() < 2 {
                return Err("`P` precisa de ao menos 2 pontos".into());
            }
            Ok(Geometria::Polilinha { vertices: v, fechada })
        }
        "H" => hachura(p),
        "F3D" => {
            precisa(8)?;
            let n = nums(&p[..8])?;
            Ok(Geometria::Face3D {
                p1: Ponto::novo(n[0], n[1]),
                p2: Ponto::novo(n[2], n[3]),
                p3: Ponto::novo(n[4], n[5]),
                p4: Ponto::novo(n[6], n[7]),
            })
        }
        outra => Err(format!("comando `{outra}` desconhecido")),
    }
}

/// Uma linha de camada: `camada NOME [#cor] [off] [congelada] [bloqueada] [esp=] [linha=] [transp=]`.
fn camada_da_linha(p: &[String]) -> Res<CamadaLida> {
    let nome = p.get(1).ok_or("`camada` sem nome")?.clone();
    let mut c = CamadaLida { nome, cor: None, ligada: true, congelada: false, bloqueada: false, espessura: None, tipo_linha: None, transparencia: None };
    for w in &p[2..] {
        if let Some((chave, valor)) = w.split_once('=') {
            match chave {
                "esp" => c.espessura = Some(num(&valor.replace(',', "."))? as f32),
                "linha" => c.tipo_linha = Some(TipoLinha::TODOS.into_iter().find(|t| nome_do_tipo(*t) == valor).ok_or_else(|| format!("linha `{valor}` não existe (continua, tracejada, oculta, eixo, pontilhada, fantasma)"))?),
                "transp" => c.transparencia = Some(num(valor)?.clamp(0.0, 90.0) as u8),
                outra => return Err(format!("`{outra}=` não é propriedade de camada (esp, linha, transp)")),
            }
            continue;
        }
        match w.as_str() {
            "off" => c.ligada = false,
            "congelada" => c.congelada = true,
            "bloqueada" => c.bloqueada = true,
            outra => c.cor = Some(cor_de(outra).ok_or_else(|| format!("`{outra}` não é cor (#RRGGBB) nem estado de camada"))?),
        }
    }
    Ok(c)
}

/// `camera iso|obliqua` ou `camera az= el= escala= origem=x,y` (o que faltar fica no padrão da isométrica, origem 0,0).
fn camera_da_linha(p: &[String]) -> Res<(crate::projecao::Camera, Ponto)> {
    let mut origem = Ponto::novo(0.0, 0.0);
    let mut obj = serde_json::Map::new();
    for w in &p[1..] {
        if let Some((k, v)) = w.split_once('=') {
            match k {
                "origem" => {
                    let (x, y) = v.split_once(',').ok_or("`origem=x,y`")?;
                    origem = Ponto::novo(num(x)?, num(y)?);
                }
                "az" | "azimute" => drop(obj.insert("azimute".into(), num(v)?.into())),
                "el" | "elevacao" => drop(obj.insert("elevacao".into(), num(v)?.into())),
                "escala" => drop(obj.insert("escala".into(), num(v)?.into())),
                o => return Err(format!("`{o}=` não é da câmera (az, el, escala, origem)")),
            }
        } else if matches!(w.as_str(), "iso" | "obliqua") {
            return Ok((crate::projecao::camera(Some(&serde_json::Value::String(w.clone())))?, origem));
        } else {
            return Err(format!("câmera `{w}`: iso, obliqua ou az= el= escala= origem="));
        }
    }
    obj.entry("azimute").or_insert(45.0.into());
    Ok((crate::projecao::camera(Some(&serde_json::Value::Object(obj)))?, origem))
}

/// `L3 x y z x y z` ou `P3 x y z x y z …` (3+ pontos): projetados para 2D pela câmera vigente.
fn forma_3d(letra: &str, p: &[String], (cam, origem): &(crate::projecao::Camera, Ponto)) -> Res<Geometria> {
    let n = nums(p)?;
    if n.len() % 3 != 0 || n.len() < 6 {
        return Err(format!("`{letra}` precisa de pontos x y z (no mínimo 2)"));
    }
    let pts: Vec<Ponto> = n.chunks(3).map(|c| { let (x, y) = crate::projecao::projetar(*cam, c[0], c[1], c[2]); Ponto::novo(origem.x + x, origem.y + y) }).collect();
    if letra == "L3" {
        if pts.len() != 2 {
            return Err("`L3` leva exatamente 2 pontos (use `P3` para vários)".into());
        }
        return Ok(Geometria::Linha { a: pts[0], b: pts[1] });
    }
    Ok(Geometria::Polilinha { vertices: pts.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada: false })
}

/// Lê o texto compacto. Erros dizem a linha. Linhas vazias e as que começam por `//` são ignoradas.
pub fn importar(texto: &str) -> Res<Lido> {
    let mut lido = Lido::default();
    let mut atual = String::from("0");
    let mut camera3d = (crate::projecao::camera(None)?, Ponto::novo(0.0, 0.0));
    for (n, bruta) in texto.lines().enumerate() {
        let linha = bruta.trim();
        if linha.is_empty() || linha.starts_with("//") {
            continue;
        }
        let ctx = |e: String| format!("linha {}: {e}", n + 1);
        let mut p = palavras(linha).map_err(ctx)?;
        if p[0].starts_with('@') {
            p.remove(0);
        }
        match p.first().map(String::as_str) {
            Some("unidade") => lido.unidade = p.get(1).cloned(),
            // câmera das linhas 3D: `camera iso`, `camera obliqua` ou `camera az=20 el=35 escala=2 origem=x,y`
            Some("camera") => camera3d = camera_da_linha(&p).map_err(ctx)?,
            Some(l @ ("L3" | "P3")) => {
                let g = forma_3d(l, &p[1..], &camera3d).map_err(ctx)?;
                lido.formas.push((atual.clone(), None, g));
            }
            Some("camada") => {
                let c = camada_da_linha(&p).map_err(ctx)?;
                atual.clone_from(&c.nome);
                lido.camadas.push(c);
            }
            Some(letra) => {
                let cor = p.last().filter(|_| p.len() > 1).and_then(|u| cor_de(u));
                let fim = p.len() - usize::from(cor.is_some());
                let g = forma(letra, &p[1..fim]).map_err(ctx)?;
                lido.formas.push((atual.clone(), cor, g));
            }
            None => {}
        }
    }
    Ok(lido)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    #[test]
    fn linhas_3d_sao_projetadas_pela_camera() {
        let l = importar("camera az=0 el=90 origem=10,20\nL3 1 2 9 3 4 9\nP3 0 0 0 1 0 0 1 1 5").unwrap();
        assert_eq!(l.formas.len(), 2);
        assert!(matches!(&l.formas[0].2, Geometria::Linha { a, .. } if (a.x - 11.0).abs() < 1e-9 && (a.y - 22.0).abs() < 1e-9));
        assert!(importar("L3 1 2 3").is_err());
        assert!(importar("camera cabeca").is_err());
    }

    #[test]
    fn camada_declarada_de_novo_sem_cor_mantem_a_cor() {
        let l = importar("camada A #FF0000\nL 0 0 1 1\ncamada B #00FF00\nL 0 0 1 1\ncamada A\nL 0 0 2 2\n").unwrap();
        let mut d = Documento::default();
        crate::mutacoes::aplicar_compacto(&mut d, &l).unwrap();
        let a = d.camadas().iter().find(|c| c.nome == "A").unwrap();
        assert_eq!(a.cor, 0xFF_00_00, "a 2ª declaração sem cor não volta para branco");
    }

    #[test]
    fn texto_alinhado_e_camada_com_propriedades_vao_e_voltam() {
        let l = importar("camada EIXO #FF0000 esp=0.35 linha=eixo transp=30\nT 1 2 2.5 0 \"A\" cm\nT 0 0 1 0 \"B\"\nT 0 0 1 0 \"C\" d\nT 0 0 1 0 \"D\" f\n").unwrap();
        assert_eq!(l.camadas[0].espessura, Some(0.35));
        assert_eq!(l.camadas[0].tipo_linha, Some(TipoLinha::Eixo));
        assert_eq!(l.camadas[0].transparencia, Some(30));
        let mut d = Documento::default();
        crate::mutacoes::aplicar_compacto(&mut d, &l).unwrap();
        let c = d.camadas().iter().find(|c| c.nome == "EIXO").unwrap();
        assert_eq!((c.espessura, c.tipo_linha, c.transparencia), (Some(0.35), TipoLinha::Eixo, 30));
        let (texto, _) = exportar(&d, None, false);
        assert!(texto.contains("esp=0.35 linha=eixo transp=30"), "{texto}");
        let l2 = importar(&texto).unwrap();
        for (a, b) in l.formas.iter().zip(&l2.formas) {
            assert_eq!(a.2, b.2);
        }
        assert!(matches!(l.formas[0].2, Geometria::Texto { h: AlinhaH::Centro, v: AlinhaV::Meio, .. }));
        assert!(matches!(l.formas[3].2, Geometria::Texto { h: AlinhaH::Esquerda, v: AlinhaV::Baixo, .. }));
        assert!(importar("T 0 0 1 0 \"x\" zz\n").is_err());
    }

    #[test]
    fn ida_e_volta_de_todas_as_formas() {
        let mut d = Documento::default();
        d.adicionar_linha(p(0.0, 0.0), p(100.0, 50.0));
        d.adicionar_circulo(p(50.0, 25.0), 10.0);
        d.adicionar_arco(p(0.0, 0.0), 5.0, 10.0, 100.0);
        d.adicionar_geometria(Geometria::Polilinha { vertices: vec![Vertice { p: p(0.0, 0.0), bulge: 0.5 }, Vertice { p: p(4.0, 0.0), bulge: 0.0 }, Vertice { p: p(4.0, 3.0), bulge: 0.0 }], fechada: true });
        d.adicionar_geometria(Geometria::Polilinha { vertices: [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)].iter().map(|q| Vertice { p: *q, bulge: 0.0 }).collect(), fechada: true });
        d.adicionar_geometria(Geometria::Texto { ponto: p(1.0, 2.0), altura: 2.5, rotacao: 0.0, conteudo: "Sala \"A\"".into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
        let (texto, est) = exportar(&d, None, false);
        assert_eq!(est.objetos, 6);
        assert!(texto.contains("R 0 0 4 3"), "{texto}");
        let l = importar(&texto).unwrap();
        assert_eq!(l.formas.len(), 6);
        for (i, e) in d.entidades().iter().enumerate() {
            assert_eq!(l.formas[i].2, e.geometria, "forma {i}");
        }
    }

    #[test]
    fn e_muito_menor_que_o_dxf() {
        let mut d = Documento::default();
        for i in 0..50 {
            d.adicionar_linha(p(f64::from(i), 0.0), p(f64::from(i) + 0.5, 12.25));
        }
        let (texto, _) = exportar(&d, None, false);
        let dxf = feathercad_io::gravar(&d, feathercad_io::Formato::Dxf).unwrap().bytes;
        assert!(dxf.len() > texto.len() * 3, "dxf {} x compacto {}", dxf.len(), texto.len());
    }

    #[test]
    fn erros_dizem_a_linha_e_camada_e_cor_funcionam() {
        let e = importar("camada A #FF0000\nL 0 0 10\n").err().unwrap();
        assert!(e.starts_with("linha 2:"), "{e}");
        let l = importar("unidade cm\ncamada \"Minha Camada\" #00FF00 off\n@7 C 1 2 3 #112233\n").unwrap();
        assert_eq!(l.unidade.as_deref(), Some("cm"));
        assert!(!l.camadas[0].ligada && l.camadas[0].nome == "Minha Camada");
        assert_eq!(l.formas[0].0, "Minha Camada");
        assert_eq!(l.formas[0].1, Some(0x11_22_33));
    }

    #[test]
    fn abre_blocos_em_formas_soltas() {
        let mut d = Documento::default();
        let b = feathercad_core::documento::Bloco { nome: "B".into(), base: p(0.0, 0.0), entidades: vec![feathercad_core::documento::NovaEntidade { camada: 0, cor: None, geometria: Geometria::Linha { a: p(0.0, 0.0), b: p(1.0, 0.0) }, props: Default::default(), espaco: Default::default() }] };
        let d2 = Documento::de_partes_com_blocos(d.camadas().to_vec(), 0, vec![b], vec![feathercad_core::documento::NovaEntidade { camada: 0, cor: None, geometria: Geometria::Insercao { bloco: 0, ponto: p(10.0, 5.0), escala_x: 2.0, escala_y: 2.0, rotacao: 0.0 }, props: Default::default(), espaco: Default::default() }]);
        d = d2;
        let (texto, est) = exportar(&d, None, false);
        assert_eq!(est.blocos_abertos, 1);
        assert!(texto.contains("L 10 5 12 5"), "{texto}");
    }
}
