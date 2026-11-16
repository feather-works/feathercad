//! A arquitetura que a planta elétrica precisa, e o que se apoia nela:
//! - `cad_parede`: parede CHEIA com espessura, cortada nos vãos (porta de giro com folha e arco, porta de correr, janela, passagem);
//! - `cad_ambiente`: nome e área do ambiente;
//! - `encostar` (de `cad_simbolo`): leva tomada/interruptor/arandela para a face da parede mais próxima e gira sozinho;
//! - `cad_ramais`: o ramal de um ambiente DENTRO da parede, cortado em cada vão, com uma descida perpendicular do ponto de luz
//!   por trecho de parede.
//!
//! Nasceu do projeto do apartamento de 04/11/2026, em que tudo isso foi feito em scripts soltos porque o MCP não sabia o que
//! era parede, porta nem ambiente. As paredes são lidas do próprio desenho (hachura cheia ou polilinha fechada numa camada
//! com PAREDE no nome), então funcionam com paredes de `cad_parede` ou de um DXF importado.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Espaco, Geometria, Mudanca, NovaEntidade, Propriedades};
use feathercad_core::geom::{Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{arred, lista_de_pontos, numero_opcional, ponto_de, texto_de, texto_opcional, Res};
use crate::mutacoes::camada_por_nome;
use crate::simbolos::mm_de_papel;

/// Prefixo dos blocos de símbolo elétrico (`SIMB-tomada_baixa-12`): é por ele que se acham tomadas e luzes no desenho.
pub const PREFIXO_SIMBOLO: &str = "SIMB-";

/// `m` metros na unidade do desenho.
fn metros(doc: &Documento, m: f64) -> Res<f64> {
    let un = doc.unidade().ok_or("o desenho ainda não tem unidade: use cad_novo primeiro")?;
    Ok(m * 1000.0 / un.em_milimetros())
}

fn novo(camada: usize, geometria: Geometria) -> Mudanca {
    Mudanca::Novo(NovaEntidade { camada, cor: None, geometria, props: Propriedades::default(), espaco: Espaco::default() })
}

fn mais(a: Ponto, d: (f64, f64), k: f64) -> Ponto {
    Ponto::novo(a.x + d.0 * k, a.y + d.1 * k)
}

fn cheia(cantos: [Ponto; 4]) -> Geometria {
    Geometria::Hachura { contornos: vec![cantos.iter().map(|p| Vertice { p: *p, bulge: 0.0 }).collect()], padrao: Vec::new(), nome: "SOLID".into() }
}

#[derive(Clone, Copy, PartialEq)]
enum TipoVao {
    Porta,
    Correr,
    Janela,
    Passagem,
}

struct Vao {
    inicio: f64,
    fim: f64,
    tipo: TipoVao,
    dobradica_no_fim: bool,
    abre_a_direita: bool,
}

fn vaos_de(p: &Value, comprimento: f64, k: usize) -> Res<Vec<Vao>> {
    let mut vaos = Vec::new();
    for (j, v) in p.get("vaos").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default().iter().enumerate() {
        let onde = format!("paredes[{k}].vaos[{j}]");
        let inicio = numero_opcional(v, "inicio").ok_or_else(|| format!("{onde}: falta `inicio` (distância desde `de`)"))?;
        let largura = numero_opcional(v, "largura").filter(|l| *l > 0.0).ok_or_else(|| format!("{onde}: falta `largura` > 0"))?;
        let tipo = match texto_opcional(v, "tipo").unwrap_or("porta") {
            "porta" => TipoVao::Porta,
            "correr" => TipoVao::Correr,
            "janela" => TipoVao::Janela,
            "passagem" => TipoVao::Passagem,
            t => return Err(format!("{onde}: `tipo` `{t}` não vale (porta, correr, janela, passagem)")),
        };
        if inicio < -1e-9 || inicio + largura > comprimento + 1e-9 {
            return Err(format!("{onde}: o vão ({inicio} a {}) sai da parede (0 a {comprimento:.3})", inicio + largura));
        }
        vaos.push(Vao {
            inicio,
            fim: inicio + largura,
            tipo,
            dobradica_no_fim: matches!(texto_opcional(v, "dobradica"), Some("fim")),
            abre_a_direita: matches!(texto_opcional(v, "abre"), Some("direita")),
        });
    }
    vaos.sort_by(|a, b| a.inicio.total_cmp(&b.inicio));
    if let Some(w) = vaos.windows(2).find(|w| w[1].inicio < w[0].fim - 1e-9) {
        return Err(format!("paredes[{k}]: vãos sobrepostos ({} a {} e {} a {})", w[0].inicio, w[0].fim, w[1].inicio, w[1].fim));
    }
    Ok(vaos)
}

/// O desenho de um vão na camada de esquadrias.
fn esquadria(ms: &mut Vec<Mudanca>, ce: usize, de: Ponto, d: (f64, f64), n: (f64, f64), t: f64, v: &Vao) {
    let em = |s: f64, k: f64| mais(mais(de, d, s), n, k);
    // batentes: o traço que fecha a parede dos dois lados do vão
    for s in [v.inicio, v.fim] {
        ms.push(novo(ce, Geometria::Linha { a: em(s, -t / 2.0), b: em(s, t / 2.0) }));
    }
    let w = v.fim - v.inicio;
    match v.tipo {
        TipoVao::Passagem => {}
        TipoVao::Janela => {
            for k in [-t / 2.0, 0.0, t / 2.0] {
                ms.push(novo(ce, Geometria::Linha { a: em(v.inicio, k), b: em(v.fim, k) }));
            }
        }
        TipoVao::Correr => {
            let (meio, f) = ((v.inicio + v.fim) / 2.0, t / 6.0);
            ms.push(novo(ce, Geometria::Linha { a: em(v.inicio, f), b: em(meio + 0.05 * w, f) }));
            ms.push(novo(ce, Geometria::Linha { a: em(meio - 0.05 * w, -f), b: em(v.fim, -f) }));
        }
        TipoVao::Porta => {
            let (s_dob, s_outra) = if v.dobradica_no_fim { (v.fim, v.inicio) } else { (v.inicio, v.fim) };
            let lado = if v.abre_a_direita { -1.0 } else { 1.0 };
            let h = em(s_dob, 0.0);
            let folha = mais(h, n, lado * w);
            ms.push(novo(ce, Geometria::Linha { a: h, b: folha }));
            let ang = |p: Ponto| (p.y - h.y).atan2(p.x - h.x).to_degrees();
            let (a1, a2) = (ang(folha), ang(em(s_outra, 0.0)));
            let (inicio, fim) = if (a2 - a1).rem_euclid(360.0) <= 180.0 { (a1, a2) } else { (a2, a1) };
            ms.push(novo(ce, Geometria::Arco { centro: h, raio: w, inicio: inicio.rem_euclid(360.0), fim: fim.rem_euclid(360.0) }));
        }
    }
}

/// `cad_parede`: paredes cheias com os vãos.
pub fn parede(doc: &mut Documento, args: &Value) -> Res<Value> {
    let lista = args.get("paredes").and_then(Value::as_array).ok_or("falta `paredes`: lista de {de, ate, espessura?, vaos?}")?;
    let esp_padrao = match numero_opcional(args, "espessura") {
        Some(e) => e,
        None => metros(doc, 0.15)?,
    };
    let cp = camada_por_nome(doc, texto_opcional(args, "camada").unwrap_or("ARQ-Paredes"))?;
    let ce = camada_por_nome(doc, texto_opcional(args, "camada_esquadrias").unwrap_or("ARQ-Esquadrias"))?;
    let mut ms = Vec::new();
    let (mut cheios, mut n_vaos) = (0usize, 0usize);
    for (k, p) in lista.iter().enumerate() {
        let de = ponto_de(p, "de").map_err(|e| format!("paredes[{k}]: {e}"))?;
        let ate = ponto_de(p, "ate").map_err(|e| format!("paredes[{k}]: {e}"))?;
        let t = numero_opcional(p, "espessura").unwrap_or(esp_padrao);
        let comprimento = de.distancia(ate);
        if comprimento < 1e-9 || t <= 0.0 {
            return Err(format!("paredes[{k}]: comprimento e espessura precisam ser maiores que zero"));
        }
        let d = ((ate.x - de.x) / comprimento, (ate.y - de.y) / comprimento);
        let n = (-d.1, d.0);
        let vaos = vaos_de(p, comprimento, k)?;
        let mut ini = 0.0;
        let pedacos = vaos.iter().map(|v| (v.inicio, v.fim)).chain([(comprimento, comprimento)]);
        for (a, b) in pedacos {
            if a - ini > 1e-9 {
                let (p0, p1) = (mais(de, d, ini), mais(de, d, a));
                ms.push(novo(cp, cheia([mais(p0, n, -t / 2.0), mais(p1, n, -t / 2.0), mais(p1, n, t / 2.0), mais(p0, n, t / 2.0)])));
                cheios += 1;
            }
            ini = b;
        }
        for v in &vaos {
            esquadria(&mut ms, ce, de, d, n, t, v);
        }
        n_vaos += vaos.len();
    }
    let ids = doc.aplicar_mudancas(ms);
    Ok(json!({ "paredes": lista.len(), "trechos_cheios": cheios, "vaos": n_vaos, "entidades": ids.len() }))
}

/// `cad_ambiente`: nome e área no meio do ambiente.
pub fn ambiente(doc: &mut Documento, args: &Value) -> Res<Value> {
    let nome = texto_de(args, "nome")?;
    let pts = if let Some(r) = args.get("retangulo") {
        let v = lista_de_pontos(r, "retangulo")?;
        let [a, b] = v.as_slice() else { return Err("`retangulo` = [[x1, y1], [x2, y2]]".into()) };
        vec![*a, Ponto::novo(b.x, a.y), *b, Ponto::novo(a.x, b.y)]
    } else {
        lista_de_pontos(args.get("pontos").ok_or("falta `retangulo` ou `pontos` (contorno do ambiente)")?, "pontos")?
    };
    if pts.len() < 3 {
        return Err("o contorno precisa de ao menos 3 pontos".into());
    }
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        let x = p.x * q.y - q.x * p.y;
        a2 += x;
        cx += (p.x + q.x) * x;
        cy += (p.y + q.y) * x;
    }
    if a2.abs() < 1e-12 {
        return Err("o contorno não tem área".into());
    }
    let centro = Ponto::novo(cx / (3.0 * a2), cy / (3.0 * a2));
    let un_m = metros(doc, 1.0)?;
    let area_m2 = (a2 / 2.0).abs() / (un_m * un_m);
    let s = mm_de_papel(doc, numero_opcional(args, "escala").filter(|e| *e > 0.0).unwrap_or(50.0))?;
    let h = numero_opcional(args, "altura_texto").filter(|h| *h > 0.0).unwrap_or(2.5) * s;
    let p = args.get("ponto").map(|_| ponto_de(args, "ponto")).transpose()?.unwrap_or(centro);
    let camada = camada_por_nome(doc, texto_opcional(args, "camada").unwrap_or("ARQ-Textos"))?;
    let area_txt = format!("{area_m2:.1} m²").replace('.', ",");
    let ids = doc.aplicar_mudancas(vec![
        novo(camada, Geometria::Texto { ponto: Ponto::novo(p.x, p.y + 0.15 * h), altura: h, rotacao: 0.0, conteudo: nome.to_string(), h: AlinhaH::Centro, v: AlinhaV::Base }),
        novo(camada, Geometria::Texto { ponto: Ponto::novo(p.x, p.y - 0.15 * h), altura: 0.8 * h, rotacao: 0.0, conteudo: area_txt.clone(), h: AlinhaH::Centro, v: AlinhaV::Topo }),
    ]);
    Ok(json!({ "ambiente": nome, "area_m2": arred(area_m2), "texto_area": area_txt, "ids": ids }))
}

/// O contorno de cada parede do desenho: hachura ou polilinha fechada numa camada com PAREDE no nome.
pub fn paredes_do_desenho(doc: &Documento) -> Vec<Vec<Ponto>> {
    doc.entidades()
        .iter()
        .filter(|e| doc.visivel(e) && doc.camadas().get(e.camada).is_some_and(|c| c.nome.to_uppercase().contains("PAREDE")))
        .filter_map(|e| match &e.geometria {
            Geometria::Hachura { contornos, .. } => contornos.first().map(|c| c.iter().map(|v| v.p).collect::<Vec<_>>()),
            Geometria::Polilinha { vertices, fechada: true } => Some(vertices.iter().map(|v| v.p).collect()),
            _ => None,
        })
        .filter(|c| c.len() >= 3)
        .collect()
}

/// A face de parede mais perto de um ponto: onde ela está e para que lado fica a parede (unitário, da face para dentro dela).
struct Face {
    ponto: Ponto,
    para_a_parede: (f64, f64),
    distancia: f64,
}

fn face_mais_proxima(paredes: &[Vec<Ponto>], p: Ponto) -> Option<Face> {
    let mut melhor: Option<Face> = None;
    for c in paredes {
        let meio = Ponto::novo(c.iter().map(|q| q.x).sum::<f64>() / c.len() as f64, c.iter().map(|q| q.y).sum::<f64>() / c.len() as f64);
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let l = a.distancia(b);
            if l < 1e-12 {
                continue;
            }
            let u = (((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / (l * l)).clamp(0.0, 1.0);
            let q = Ponto::novo(a.x + (b.x - a.x) * u, a.y + (b.y - a.y) * u);
            let dist = p.distancia(q);
            if melhor.as_ref().is_some_and(|m| m.distancia <= dist) {
                continue;
            }
            let mut nn = (-(b.y - a.y) / l, (b.x - a.x) / l);
            if nn.0 * (meio.x - q.x) + nn.1 * (meio.y - q.y) < 0.0 {
                nn = (-nn.0, -nn.1);
            }
            melhor = Some(Face { ponto: q, para_a_parede: nn, distancia: dist });
        }
    }
    melhor
}

/// `encostar: true` de `cad_simbolo`: o ponto vai para a face da parede mais próxima e o giro aponta para fora dela.
/// `s` = unidades do desenho por mm de papel.
pub fn encostar(doc: &Documento, nome: &str, p: Ponto, s: f64) -> Res<(Ponto, f64)> {
    let paredes = paredes_do_desenho(doc);
    if paredes.is_empty() {
        return Err("`encostar`: não há parede no desenho (hachura cheia ou polilinha fechada numa camada com PAREDE no nome); desenhe com cad_parede".into());
    }
    let f = face_mais_proxima(&paredes, p).ok_or("`encostar`: nenhuma face de parede")?;
    let limite = metros(doc, 1.0)?;
    if f.distancia > limite {
        return Err(format!("`encostar`: a parede mais próxima está a {:.2} (mais de 1 m) do ponto; confira o ponto", f.distancia));
    }
    let w = f.para_a_parede;
    if nome.starts_with("tomada") && nome != "tomada_piso" {
        // giro 0 = parede ao norte do símbolo; a parede fica em (−sen θ, cos θ)
        Ok((f.ponto, (-w.0).atan2(w.1).to_degrees().rem_euclid(360.0)))
    } else if nome.starts_with("interruptor") {
        let r = (1.1 + 0.35) * s;
        Ok((Ponto::novo(f.ponto.x - w.0 * r, f.ponto.y - w.1 * r), 0.0))
    } else if nome == "arandela" {
        // a arandela é desenhada com a parede a oeste
        Ok((f.ponto, (-w.1).atan2(-w.0).to_degrees().rem_euclid(360.0)))
    } else {
        Err(format!("`encostar` vale para tomadas, interruptores e arandela, não para `{nome}`"))
    }
}

/// Parede reta de eixo horizontal (`h`) ou vertical, lida de um retângulo cheio.
struct Reta {
    horizontal: bool,
    eixo: f64,
    de: f64,
    ate: f64,
    espessura: f64,
}

fn retas(paredes: &[Vec<Ponto>]) -> Vec<Reta> {
    paredes
        .iter()
        .filter_map(|c| {
            let (x0, x1) = c.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
            let (y0, y1) = c.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
            let area: f64 = (0..c.len()).map(|i| c[i].x * c[(i + 1) % c.len()].y - c[(i + 1) % c.len()].x * c[i].y).sum::<f64>().abs() / 2.0;
            // shortcut: só paredes em esquadro com os eixos; parede inclinada ou curva fica de fora do cad_ramais
            (area > 0.98 * (x1 - x0) * (y1 - y0)).then(|| {
                if x1 - x0 >= y1 - y0 {
                    Reta { horizontal: true, eixo: (y0 + y1) / 2.0, de: x0, ate: x1, espessura: y1 - y0 }
                } else {
                    Reta { horizontal: false, eixo: (x0 + x1) / 2.0, de: y0, ate: y1, espessura: x1 - x0 }
                }
            })
        })
        .collect()
}

/// O nome do símbolo de uma inserção `SIMB-<nome>-<n>`.
pub fn nome_do_simbolo(doc: &Documento, g: &Geometria) -> Option<(String, Ponto)> {
    let Geometria::Insercao { bloco, ponto, .. } = g else { return None };
    let n = doc.blocos().get(*bloco)?.nome.strip_prefix(PREFIXO_SIMBOLO)?;
    Some((n.rsplit_once('-').map_or(n, |(a, _)| a).to_string(), *ponto))
}

/// As tomadas/interruptores (pontos de parede) e as luzes de um ambiente, pelos blocos de cad_simbolo.
fn pontos_e_luzes(doc: &Documento, dentro: impl Fn(Ponto) -> bool) -> (Vec<(String, Ponto)>, Vec<Ponto>) {
    let (mut pontos, mut luzes) = (Vec::new(), Vec::new());
    for e in doc.entidades().iter().filter(|e| doc.visivel(e)) {
        if let Some((nome, p)) = nome_do_simbolo(doc, &e.geometria).filter(|(_, p)| dentro(*p)) {
            if (nome.starts_with("tomada") && nome != "tomada_piso") || nome.starts_with("interruptor") {
                pontos.push((nome, p));
            } else if nome.starts_with("ponto_luz") || nome.starts_with("luminaria") {
                luzes.push(p);
            }
        }
    }
    (pontos, luzes)
}

/// Onde há parede cheia na linha `eixo` entre `faixa.0` e `faixa.1` (os vãos são os buracos), em trechos já juntados.
fn trechos_cheios(paredes: &[Reta], horizontal: bool, eixo: f64, faixa: (f64, f64), tol: f64) -> Vec<(f64, f64)> {
    let mut cheio: Vec<(f64, f64)> = paredes.iter().filter(|w| w.horizontal == horizontal && (w.eixo - eixo).abs() <= w.espessura / 2.0).map(|w| (w.de.max(faixa.0), w.ate.min(faixa.1))).filter(|(a, b)| b - a > 1e-9).collect();
    cheio.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut trechos: Vec<(f64, f64)> = Vec::new();
    for (a, b) in cheio {
        match trechos.last_mut() {
            Some(t) if a <= t.1 + tol => t.1 = t.1.max(b),
            _ => trechos.push((a, b)),
        }
    }
    trechos
}

/// `cad_ramais`: o ramal de um ambiente pelas paredes.
pub fn ramais(doc: &mut Documento, args: &Value) -> Res<Value> {
    let r = lista_de_pontos(args.get("ambiente").ok_or("falta `ambiente`: [[x1, y1], [x2, y2]] pelos EIXOS das paredes")?, "ambiente")?;
    let [r0, r1] = r.as_slice() else { return Err("`ambiente` = [[x1, y1], [x2, y2]]".into()) };
    let (x1, y1, x2, y2) = (r0.x.min(r1.x), r0.y.min(r1.y), r0.x.max(r1.x), r0.y.max(r1.y));
    let dentro = |p: Ponto| p.x >= x1 - 1e-9 && p.x <= x2 + 1e-9 && p.y >= y1 - 1e-9 && p.y <= y2 + 1e-9;
    let paredes = retas(&paredes_do_desenho(doc));
    if paredes.is_empty() {
        return Err("não há parede no desenho (desenhe com cad_parede)".into());
    }
    let recuo = match numero_opcional(args, "recuo") {
        Some(v) => v,
        None => metros(doc, 0.03)?,
    };
    let tol = metros(doc, 0.02)?;
    let esp_max = paredes.iter().map(|p| p.espessura).fold(0.0, f64::max);

    let (pontos, mut luzes) = pontos_e_luzes(doc, dentro);
    if let Some(a) = args.get("alimentacao") {
        luzes.extend(lista_de_pontos(a, "alimentacao")?);
    }
    if luzes.is_empty() {
        return Err("o ambiente não tem ponto de luz (símbolo ponto_luz_*/luminaria_* de cad_simbolo): passe `alimentacao` com o ponto de onde o ramal sai".into());
    }

    // (horizontal?, eixo, de, até, para onde fica o ambiente)
    let lados = [(true, y1, x1, x2, 1.0, "sul"), (true, y2, x1, x2, -1.0, "norte"), (false, x1, y1, y2, 1.0, "oeste"), (false, x2, y1, y2, -1.0, "leste")];
    let ao_longo = |h: bool, p: Ponto| if h { (p.x, p.y) } else { (p.y, p.x) };
    let em = |h: bool, s: f64, k: f64| if h { Ponto::novo(s, k) } else { Ponto::novo(k, s) };
    let mut linhas: Vec<Vec<Ponto>> = Vec::new();
    let mut avisos: Vec<String> = Vec::new();
    let mut usados = vec![false; pontos.len()];
    for &(h, eixo, de, ate, lado, nome_lado) in &lados {
        // pontos desta parede: o lado mais perto, e perto o bastante para estar na face
        let mut ss: Vec<f64> = Vec::new();
        for (i, (_, p)) in pontos.iter().enumerate() {
            let (s, k) = ao_longo(h, *p);
            let dist = (k - eixo).abs();
            let mais_perto = lados.iter().all(|&(h2, e2, ..)| (ao_longo(h2, *p).1 - e2).abs() >= dist - 1e-9);
            if mais_perto && dist <= esp_max / 2.0 + metros(doc, 0.15)? && s >= de - tol && s <= ate + tol {
                ss.push(s);
                usados[i] = true;
            }
        }
        if ss.is_empty() {
            continue;
        }
        let trechos = trechos_cheios(&paredes, h, eixo, (de, ate), tol);
        let cc = eixo + lado * recuo;
        for s in &ss {
            if !trechos.iter().any(|(a, b)| *s >= a - tol && *s <= b + tol) {
                avisos.push(format!("parede {nome_lado}: ponto em {s:.2} cai num vão (porta/passagem), sem ramal; mude o ponto de lugar"));
            }
        }
        for &(ta, tb) in &trechos {
            let aqui: Vec<f64> = ss.iter().copied().filter(|s| *s >= ta - tol && *s <= tb + tol).collect();
            if aqui.is_empty() {
                continue;
            }
            let (lo_p, hi_p) = (aqui.iter().copied().fold(f64::MAX, f64::min), aqui.iter().copied().fold(f64::MIN, f64::max));
            // a luz que desce perpendicular neste trecho; senão, a que chega com menos caminho em L
            let custo = |l: &Ponto| {
                let (al, pe) = ao_longo(h, *l);
                let perp = (pe - cc).abs();
                if al >= ta - tol && al <= tb + tol { perp } else { 1e9 + (al - al.clamp(lo_p, hi_p)).abs() + perp }
            };
            let luz = *luzes.iter().min_by(|a, b| custo(a).total_cmp(&custo(b))).expect("há luz");
            let (al, pe) = ao_longo(h, luz);
            let alvo = if al >= ta - tol && al <= tb + tol { al.clamp(ta, tb) } else { al.clamp(lo_p, hi_p) };
            if (alvo - al).abs() < 1e-9 {
                linhas.push(vec![luz, em(h, alvo, cc)]);
            } else {
                linhas.push(vec![luz, em(h, alvo, pe), em(h, alvo, cc)]);
            }
            let (lo, hi) = (lo_p.min(alvo), hi_p.max(alvo));
            if hi - lo > 1e-9 {
                linhas.push(vec![em(h, lo, cc), em(h, hi, cc)]);
            }
        }
    }
    for (i, (nome, p)) in pontos.iter().enumerate() {
        if !usados[i] {
            avisos.push(format!("{nome} em ({:.2}, {:.2}) não está encostado em parede do ambiente (use cad_simbolo com `encostar: true`)", p.x, p.y));
        }
    }
    let camada = camada_por_nome(doc, texto_opcional(args, "camada").unwrap_or("EL-Condutos"))?;
    let comprimento: f64 = linhas.iter().map(|l| l.windows(2).map(|w| w[0].distancia(w[1])).sum::<f64>()).sum();
    let ms = linhas.iter().map(|l| novo(camada, Geometria::Polilinha { vertices: l.iter().map(|p| Vertice { p: *p, bulge: 0.0 }).collect(), fechada: false })).collect();
    let ids = doc.aplicar_mudancas(ms);
    let mut res = json!({ "trechos": ids.len(), "comprimento": arred(comprimento), "pontos": pontos.len(), "luzes": luzes.len(), "ids": ids });
    if !avisos.is_empty() {
        res["avisos"] = json!(avisos);
    }
    Ok(res)
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::documento::Unidade;

    fn sala() -> Documento {
        let mut d = Documento::default().com_unidade(Some(Unidade::Metro));
        // sala 4 x 3 com porta de 0,8 na parede sul (de 1,0 a 1,8)
        parede(&mut d, &json!({ "paredes": [
            { "de": [0, 0], "ate": [4, 0], "vaos": [{ "inicio": 1.0, "largura": 0.8, "tipo": "porta" }] },
            { "de": [4, 0], "ate": [4, 3] },
            { "de": [4, 3], "ate": [0, 3], "vaos": [{ "inicio": 1.0, "largura": 1.5, "tipo": "janela" }] },
            { "de": [0, 3], "ate": [0, 0] }
        ] }))
        .unwrap();
        d
    }

    #[test]
    fn parede_com_porta_deixa_o_vao_aberto() {
        let d = sala();
        let ps = paredes_do_desenho(&d);
        // 4 paredes: a sul em 2 pedaços, a norte em 2: 6 trechos cheios
        assert_eq!(ps.len(), 6);
        let retas = retas(&ps);
        assert!(!retas.iter().any(|r| r.horizontal && r.eixo.abs() < 1e-9 && r.de < 1.4 && r.ate > 1.4), "parede cheia dentro do vão da porta");
        assert!(d.entidades().iter().any(|e| matches!(e.geometria, Geometria::Arco { .. })), "porta sem arco");
    }

    #[test]
    fn ambiente_da_area_em_metros_quadrados() {
        let mut d = sala();
        let r = ambiente(&mut d, &json!({ "nome": "SALA", "retangulo": [[0, 0], [4, 3]] })).unwrap();
        assert_eq!(r["area_m2"], 12.0);
        assert_eq!(r["texto_area"], "12,0 m²");
    }

    #[test]
    fn encostar_poe_a_tomada_na_face_e_vira_para_dentro() {
        let d = sala();
        let s = mm_de_papel(&d, 50.0).unwrap();
        // perto da parede leste (eixo x = 4, face interna x = 3,925)
        let (p, giro) = encostar(&d, "tomada_baixa", Ponto::novo(3.6, 1.5), s).unwrap();
        assert!((p.x - 3.925).abs() < 1e-9 && (p.y - 1.5).abs() < 1e-9, "{p:?}");
        assert!((giro - 270.0).abs() < 1e-9, "parede leste = giro 270, deu {giro}");
        assert!(encostar(&d, "ponto_luz_teto", Ponto::novo(2.0, 1.5), s).is_err());
    }

    #[test]
    fn ramal_corre_na_parede_e_nao_cruza_a_porta() {
        let mut d = sala();
        let s = |d: &mut Documento, nome: &str, p: [f64; 2]| {
            crate::simbolos::simbolo(d, &json!({ "nome": nome, "ponto": p, "encostar": !nome.starts_with("ponto"), "circuito": 1, "potencia": 100, "comando": "a" })).unwrap();
        };
        s(&mut d, "ponto_luz_teto", [2.0, 1.5]);
        s(&mut d, "tomada_baixa", [0.5, 0.2]); // parede sul, antes da porta
        s(&mut d, "tomada_baixa", [3.0, 0.2]); // parede sul, depois da porta
        s(&mut d, "interruptor_simples", [2.0, 0.2]); // parede sul, depois da porta
        let r = ramais(&mut d, &json!({ "ambiente": [[0, 0], [4, 3]] })).unwrap();
        assert!(r.get("avisos").is_none(), "{r}");
        for e in d.entidades() {
            if let Geometria::Polilinha { vertices, .. } = &e.geometria {
                for w in vertices.windows(2) {
                    let (a, b) = (w[0].p, w[1].p);
                    // trecho na linha da parede sul (y = 0,03) não pode passar sobre a porta (x de 1,0 a 1,8)
                    if (a.y - 0.03).abs() < 1e-9 && (b.y - 0.03).abs() < 1e-9 {
                        assert!(a.x.max(b.x) <= 1.0 + 1e-9 || a.x.min(b.x) >= 1.8 - 1e-9, "ramal sobre a porta: {a:?} {b:?}");
                    }
                    // descida da luz é em esquadro
                    assert!((a.x - b.x).abs() < 1e-9 || (a.y - b.y).abs() < 1e-9, "trecho diagonal {a:?} {b:?}");
                }
            }
        }
        // antes da porta: descida em L até a tomada; depois da porta: descida perpendicular e o ramal até a outra tomada
        assert_eq!(r["trechos"], 3, "{r}");
    }

    #[test]
    fn simbolo_em_bloco_sobrevive_ao_dxf_ao_fcdz_e_ao_pdf() {
        let mut d = sala();
        crate::simbolos::simbolo(&mut d, &json!({ "nome": "tomada_media", "ponto": [3.6, 1.5], "encostar": true, "circuito": 7, "potencia": 600, "camada": "EL-Tomadas" })).unwrap();
        let conta = |d: &Documento| d.entidades().iter().filter(|e| nome_do_simbolo(d, &e.geometria).is_some_and(|(n, _)| n == "tomada_media")).count();
        for formato in [feathercad_io::Formato::Dxf, feathercad_io::Formato::Fcdz] {
            let g = feathercad_io::gravar(&d, formato).unwrap();
            let lido = feathercad_io::abrir(&g.bytes).unwrap().doc;
            assert_eq!(conta(&lido), 1, "a tomada sumiu na volta");
            let b = lido.blocos().iter().find(|b| b.nome.starts_with(PREFIXO_SIMBOLO)).unwrap();
            assert!(b.entidades.iter().any(|n| matches!(&n.geometria, Geometria::Texto { conteudo, .. } if conteudo == "7 600W")), "rótulo perdido");
        }
        let (lo, hi) = d.caixa().unwrap();
        let pdf = feathercad_io::pdf::exportar(&d, &[(lo, hi)], &feathercad_io::pdf::Opcoes::default()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
