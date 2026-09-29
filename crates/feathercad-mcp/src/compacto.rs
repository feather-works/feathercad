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
//! T 10 10 2.5 0 "Sala"       texto: ponto, altura, giro, conteúdo
//! . 5 5                      ponto
//! H solida 0 0 10 0 10 10 | 2 2 4 2 4 4   hachura (laços separados por |)
//! ```
//! Uma linha pode começar por `@id` (o id que a IA cita) e terminar por `#RRGGBB` (cor própria).
//! `camada NOME [#cor] [off] [congelada] [bloqueada]` define a camada e a usa daí em diante.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use feathercad_core::documento::{AlinhaH, AlinhaV, Camada, Documento, Geometria, Id, LinhaDePadrao};
use feathercad_core::edicao::transformar;
use feathercad_core::geom::{Ponto, Transf, Vertice};

use crate::imagem::numero;
use crate::json::Res;

/// Blocos aninhados além disto não são abertos (proteção contra bloco que se contém).
const PROFUNDIDADE: u8 = 8;

/// O que uma linha `camada` declara.
pub struct CamadaLida {
    pub nome: String,
    pub cor: u32,
    pub ligada: bool,
    pub congelada: bool,
    pub bloqueada: bool,
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
        Geometria::Texto { ponto, altura, rotacao, conteudo, .. } => format!("T {} {} {} {}", pt(*ponto), numero(*altura), numero(*rotacao), texto_entre_aspas(conteudo)),
        Geometria::Ponto { p } => format!(". {}", pt(*p)),
        Geometria::Hachura { contornos, padrao, .. } => format!("H {} {}", if padrao.is_empty() { "solida" } else { "padrao" }, contornos.iter().map(|c| como_laco(c)).collect::<Vec<_>>().join(" | ")),
        Geometria::Insercao { .. } => return None,
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
    s.push('\n');
    s
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
    let solida = p.first().map(String::as_str) == Some("solida");
    let corpo = p.get(1..).unwrap_or(&[]).join(" ");
    let contornos: Vec<Vec<Vertice>> = corpo.split('|').map(|l| lista_de_vertices(&l.split_whitespace().map(str::to_string).collect::<Vec<_>>())).collect::<Res<_>>()?;
    if contornos.iter().any(|c| c.len() < 3) {
        return Err("hachura: cada laço precisa de ao menos 3 pontos".into());
    }
    let padrao = if solida { Vec::new() } else { vec![LinhaDePadrao { angulo: 45.0, base: contornos[0][0].p, dx: -0.7, dy: 0.7 }] };
    Ok(Geometria::Hachura { contornos, padrao, nome: if solida { "SOLID".into() } else { "ANSI31".into() } })
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
            Ok(Geometria::Texto { ponto: Ponto::novo(n[0], n[1]), altura: n[2], rotacao: n[3], conteudo: p[4].clone(), h: AlinhaH::Esquerda, v: AlinhaV::Base })
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
        outra => Err(format!("comando `{outra}` desconhecido")),
    }
}

/// Uma linha de camada: `camada NOME [#cor] [off] [congelada] [bloqueada]`.
fn camada_da_linha(p: &[String]) -> Res<CamadaLida> {
    let nome = p.get(1).ok_or("`camada` sem nome")?.clone();
    let mut c = CamadaLida { nome, cor: 0xFF_FF_FF, ligada: true, congelada: false, bloqueada: false };
    for w in &p[2..] {
        match w.as_str() {
            "off" => c.ligada = false,
            "congelada" => c.congelada = true,
            "bloqueada" => c.bloqueada = true,
            outra => c.cor = cor_de(outra).ok_or_else(|| format!("`{outra}` não é cor (#RRGGBB) nem estado de camada"))?,
        }
    }
    Ok(c)
}

/// Lê o texto compacto. Erros dizem a linha. Linhas vazias e as que começam por `//` são ignoradas.
pub fn importar(texto: &str) -> Res<Lido> {
    let mut lido = Lido::default();
    let mut atual = String::from("0");
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
        let b = feathercad_core::documento::Bloco { nome: "B".into(), base: p(0.0, 0.0), entidades: vec![feathercad_core::documento::NovaEntidade { camada: 0, cor: None, geometria: Geometria::Linha { a: p(0.0, 0.0), b: p(1.0, 0.0) }, props: Default::default() }] };
        let d2 = Documento::de_partes_com_blocos(d.camadas().to_vec(), 0, vec![b], vec![feathercad_core::documento::NovaEntidade { camada: 0, cor: None, geometria: Geometria::Insercao { bloco: 0, ponto: p(10.0, 5.0), escala_x: 2.0, escala_y: 2.0, rotacao: 0.0 }, props: Default::default() }]);
        d = d2;
        let (texto, est) = exportar(&d, None, false);
        assert_eq!(est.blocos_abertos, 1);
        assert!(texto.contains("L 10 5 12 5"), "{texto}");
    }
}
