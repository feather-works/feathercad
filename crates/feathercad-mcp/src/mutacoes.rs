//! O que muda o desenho: criar formas, editar (mover, copiar, girar…), camadas, hachura e desfazer.
//! Toda chamada valida **tudo antes** de tocar no desenho e vira **um** passo de desfazer.

use feathercad_core::aparar::hachura_com_padrao;
use feathercad_core::camadas::Propriedade;
use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Espaco, Geometria, Id, Mudanca, NovaEntidade, Propriedades, TipoLinha};
use feathercad_core::edicao::{retangulo, transformar};
use feathercad_core::geom::{arco_por_3_pontos, Ponto, Transf, Vertice};
use serde_json::{json, Value};

use crate::consulta::passa_filtros;
use crate::json::{lista_de_pontos, numero_de, numero_opcional, par, ponto, ponto_de, texto_de, texto_opcional, Res};
use crate::medidas::tipo_de;

fn vertices(pts: &[Ponto]) -> Vec<Vertice> {
    pts.iter().map(|p| Vertice { p: *p, bulge: 0.0 }).collect()
}

/// Um retângulo por dois cantos opostos, ou por canto + largura + altura (+ giro em graus).
fn forma_retangulo(f: &Value) -> Res<Geometria> {
    let base = forma_retangulo_reto(f)?;
    match (numero_opcional(f, "raio"), &base) {
        (Some(r), Geometria::Polilinha { vertices, .. }) if r > 0.0 => Ok(Geometria::Polilinha { vertices: arredondar(vertices, r), fechada: true }),
        _ => Ok(base),
    }
}

/// Cantos arredondados de raio `r` num polígono convexo anti-horário de 4 vértices (arcos por bulge de 90°).
fn arredondar(v: &[Vertice], r: f64) -> Vec<Vertice> {
    let n = v.len();
    let menor = (0..n).map(|i| v[i].p.distancia(v[(i + 1) % n].p)).fold(f64::MAX, f64::min);
    let r = r.min(menor / 2.0);
    let mut fora = Vec::with_capacity(2 * n);
    for i in 0..n {
        let (ant, atual, prox) = (v[(i + n - 1) % n].p, v[i].p, v[(i + 1) % n].p);
        let ao = |q: Ponto| {
            let d = atual.distancia(q).max(1e-12);
            Ponto::novo(atual.x + (q.x - atual.x) / d * r, atual.y + (q.y - atual.y) / d * r)
        };
        fora.push(Vertice { p: ao(ant), bulge: (std::f64::consts::FRAC_PI_2 / 4.0).tan() });
        fora.push(Vertice { p: ao(prox), bulge: 0.0 });
    }
    fora
}

fn forma_retangulo_reto(f: &Value) -> Res<Geometria> {
    if f.get("canto1").is_some() {
        return Ok(retangulo(ponto_de(f, "canto1")?, ponto_de(f, "canto2")?));
    }
    let (c, l, a) = (ponto_de(f, "canto")?, numero_de(f, "largura")?, numero_de(f, "altura")?);
    let giro = numero_opcional(f, "rotacao").unwrap_or(0.0).to_radians();
    let (s, k) = giro.sin_cos();
    let canto = |dx: f64, dy: f64| Ponto::novo(c.x + dx * k - dy * s, c.y + dx * s + dy * k);
    Ok(Geometria::Polilinha { vertices: vertices(&[canto(0.0, 0.0), canto(l, 0.0), canto(l, a), canto(0.0, a)]), fechada: true })
}

/// Hachura de um ou mais contornos (`contornos`: listas de pontos, o primeiro é o de fora, os de dentro viram ilhas) ou de um
/// só (`pontos`), com um padrão da biblioteca (`padrao`, padrão `solida`) e `escala`.
fn forma_hachura(f: &Value) -> Res<Geometria> {
    let laços: Vec<Vec<Vertice>> = match (f.get("contornos"), f.get("pontos")) {
        (Some(c), _) => c.as_array().ok_or("`contornos` precisa ser uma lista de listas de pontos")?.iter().map(|l| lista_de_pontos(l, "contornos").map(|p| vertices(&p))).collect::<Res<_>>()?,
        (None, Some(p)) => vec![vertices(&lista_de_pontos(p, "pontos")?)],
        _ => return Err("hachura: falta `pontos` (um contorno) ou `contornos`".into()),
    };
    if laços.is_empty() || laços.iter().any(|l| l.len() < 3) {
        return Err("hachura: cada contorno precisa de ao menos 3 pontos".into());
    }
    let pedido = texto_opcional(f, "padrao").unwrap_or(feathercad_core::padroes::SOLIDA);
    let nome = feathercad_core::padroes::reconhecer(pedido).ok_or_else(|| format!("padrão `{pedido}` não existe (solida ou {})", feathercad_core::padroes::NOMES.join(", ").to_lowercase()))?;
    let (padrao, nome) = feathercad_core::padroes::para_contornos(nome, numero_opcional(f, "escala").unwrap_or(1.0), &laços);
    Ok(Geometria::Hachura { contornos: laços, padrao, nome })
}

/// A linha por dois pontos, ou por ponto + distância + ângulo (polar).
fn forma_linha(f: &Value) -> Res<Geometria> {
    if f.get("a").is_some() {
        return Ok(Geometria::Linha { a: ponto_de(f, "a")?, b: ponto_de(f, "b")? });
    }
    let (de, d, ang) = (ponto_de(f, "de")?, numero_de(f, "distancia")?, numero_de(f, "angulo")?.to_radians());
    Ok(Geometria::Linha { a: de, b: Ponto::novo(de.x + d * ang.cos(), de.y + d * ang.sin()) })
}

fn forma_arco(f: &Value) -> Res<Geometria> {
    if f.get("p1").is_some() {
        let a = arco_por_3_pontos(ponto_de(f, "p1")?, ponto_de(f, "p2")?, ponto_de(f, "p3")?).ok_or("os três pontos do arco estão alinhados")?;
        return Ok(Geometria::Arco { centro: a.centro, raio: a.raio, inicio: a.inicio, fim: a.fim });
    }
    let raio = numero_de(f, "raio")?;
    if raio <= 0.0 {
        return Err("o raio do arco tem que ser maior que zero".into());
    }
    Ok(Geometria::Arco { centro: ponto_de(f, "centro")?, raio, inicio: numero_de(f, "inicio")?, fim: numero_de(f, "fim")? })
}

fn forma_poligono(f: &Value) -> Res<Geometria> {
    let (c, raio, lados) = (ponto_de(f, "centro")?, numero_de(f, "raio")?, numero_de(f, "lados")? as usize);
    if !(3..=1000).contains(&lados) || raio <= 0.0 {
        return Err("polígono: `lados` de 3 a 1000 e `raio` maior que zero".into());
    }
    let giro = numero_opcional(f, "rotacao").unwrap_or(90.0).to_radians();
    let pts: Vec<Ponto> = (0..lados).map(|k| {
        let a = giro + std::f64::consts::TAU * k as f64 / lados as f64;
        Ponto::novo(c.x + raio * a.cos(), c.y + raio * a.sin())
    }).collect();
    Ok(Geometria::Polilinha { vertices: vertices(&pts), fechada: true })
}

/// Uma forma pedida pela IA como geometria do motor.
fn forma(f: &Value) -> Res<Geometria> {
    match texto_de(f, "tipo")? {
        "linha" => forma_linha(f),
        "retangulo" => forma_retangulo(f),
        "poligono" => forma_poligono(f),
        "arco" => forma_arco(f),
        "circulo" => {
            let (centro, raio) = (ponto_de(f, "centro")?, numero_de(f, "raio")?);
            (raio > 0.0).then_some(Geometria::Circulo { centro, raio }).ok_or_else(|| "o raio do círculo tem que ser maior que zero".to_string())
        }
        "polilinha" => {
            let pts = lista_de_pontos(f.get("pontos").ok_or("falta `pontos`")?, "pontos")?;
            (pts.len() >= 2).then(|| Geometria::Polilinha { vertices: vertices(&pts), fechada: f.get("fechada").and_then(Value::as_bool).unwrap_or(false) }).ok_or_else(|| "a polilinha precisa de ao menos 2 pontos".to_string())
        }
        "hachura" => forma_hachura(f),
        "texto" => Ok(Geometria::Texto { ponto: ponto_de(f, "ponto")?, altura: numero_de(f, "altura")?, rotacao: numero_opcional(f, "rotacao").unwrap_or(0.0), conteudo: texto_de(f, "conteudo")?.to_string(), h: AlinhaH::Esquerda, v: AlinhaV::Base }),
        outro => Err(format!("forma desconhecida `{outro}` (use linha, retangulo, poligono, polilinha, circulo, arco, hachura ou texto)")),
    }
}

/// O índice da camada de nome `nome`, criando-a se não existe.
pub fn camada_por_nome(doc: &mut Documento, nome: &str) -> Res<usize> {
    if let Some(i) = doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(nome)) {
        return Ok(i);
    }
    doc.nova_camada(nome, None).map_err(|e| format!("camada `{nome}` inválida ({})", e.chave()))
}

/// Cor `#RRGGBB` (ou `RRGGBB`).
pub fn cor_hex(s: &str) -> Res<u32> {
    u32::from_str_radix(s.trim_start_matches('#'), 16).ok().filter(|c| *c <= 0xFF_FF_FF && s.trim_start_matches('#').len() == 6).ok_or_else(|| format!("cor `{s}` inválida: use #RRGGBB"))
}

/// Cria as formas de `args.formas` de uma vez (um Ctrl+Z desfaz todas).
pub fn criar(doc: &mut Documento, args: &Value) -> Res<Value> {
    let lista = args.get("formas").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`formas` precisa ser uma lista com ao menos uma forma")?;
    let geos: Vec<Geometria> = lista.iter().enumerate().map(|(i, f)| forma(f).map_err(|e| format!("forma {i}: {e}"))).collect::<Res<_>>()?;
    let cor = texto_opcional(args, "cor").map(cor_hex).transpose()?;
    let camada = match texto_opcional(args, "camada") {
        Some(n) => camada_por_nome(doc, n)?,
        None => doc.camada_atual(),
    };
    let ms = geos.iter().map(|g| Mudanca::Novo(NovaEntidade { camada, cor, geometria: g.clone(), props: Propriedades::default(), espaco: Espaco::default() })).collect();
    let ids = doc.aplicar_mudancas(ms);
    let criadas: Vec<Value> = ids.iter().zip(&geos).map(|(id, g)| json!({ "id": id, "tipo": tipo_de(g) })).collect();
    Ok(json!({ "criadas": criadas, "camada": doc.camadas()[camada].nome }))
}

/// A transformação de uma operação de edição, ou `None` para as que não transformam (apagar, propriedades).
fn transformacao(op: &str, args: &Value) -> Res<Option<Transf>> {
    Ok(Some(match op {
        "mover" | "copiar" => match args.get("de") {
            Some(de) => {
                let (a, b) = (ponto(de, "de")?, ponto_de(args, "para")?);
                Transf::mover(b.x - a.x, b.y - a.y)
            }
            None => Transf::mover(numero_de(args, "dx")?, numero_de(args, "dy")?),
        },
        "girar" => Transf::girar(ponto_de(args, "base")?, numero_de(args, "graus")?),
        "escala" => {
            let f = numero_de(args, "fator")?;
            if f <= 0.0 {
                return Err("o `fator` de escala tem que ser maior que zero".into());
            }
            Transf::escalar(ponto_de(args, "base")?, f)
        }
        "espelhar" => Transf::espelhar(ponto_de(args, "p1")?, ponto_de(args, "p2")?).ok_or("os pontos do eixo de espelho são iguais")?,
        _ => return Ok(None),
    }))
}

/// Os ids sobre os quais agir: `ids` ou o resultado dos filtros (`tipo`, `camada`, `janela`); bloqueadas ficam de fora.
fn alvos(doc: &Documento, args: &Value) -> Res<(Vec<Id>, usize)> {
    let sem_filtro = args.get("ids").is_none() && args.get("tipo").is_none() && args.get("camada").is_none() && args.get("janela").is_none();
    if sem_filtro {
        return Err("diga em quem agir: `ids`, ou filtros `tipo`, `camada`, `janela`".into());
    }
    let achadas = passa_filtros(doc, args)?;
    let livres: Vec<Id> = achadas.iter().copied().filter(|id| doc.entidade(*id).is_some_and(|e| !doc.bloqueada(e))).collect();
    let bloqueadas = achadas.len() - livres.len();
    Ok((livres, bloqueadas))
}

/// Mover, copiar (`quantidade` de cópias em passos iguais), girar, escala, espelhar (`manter_original`, padrão sim), apagar, propriedades.
pub fn editar(doc: &mut Documento, args: &Value) -> Res<Value> {
    let op = texto_de(args, "operacao")?;
    let (ids, bloqueadas) = alvos(doc, args)?;
    if ids.is_empty() {
        return Err("nenhuma entidade (livre) encontrada para essa seleção".into());
    }
    let t = transformacao(op, args)?;
    let mut ms: Vec<Mudanca> = Vec::new();
    let quantidade = if op == "copiar" { numero_opcional(args, "quantidade").map_or(1, |n| n.clamp(1.0, 500.0) as usize) } else { 1 };
    let copia_espelho = op == "espelhar" && args.get("manter_original").and_then(Value::as_bool).unwrap_or(true);
    let copia_girar = op == "girar" && args.get("copiar").and_then(Value::as_bool).unwrap_or(false);
    for id in &ids {
        let Some(e) = doc.entidade(*id) else { continue };
        match (op, &t) {
            ("apagar", _) => ms.push(Mudanca::Remover(*id)),
            ("propriedades", _) => ms.push(propriedades(doc, e.id, args)?),
            ("texto", _) => {
                if let Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } = &e.geometria {
                    let novo = texto_opcional(args, "conteudo").map_or_else(|| conteudo.clone(), str::to_string);
                    let alt = numero_opcional(args, "altura").filter(|a| *a > 0.0).unwrap_or(*altura);
                    ms.push(Mudanca::Trocar(*id, Geometria::Texto { ponto: *ponto, altura: alt, rotacao: *rotacao, conteudo: novo, h: *h, v: *v }));
                }
            }
            (_, Some(t)) if op == "copiar" => {
                let mut acumulada = *t;
                for _ in 0..quantidade {
                    ms.push(Mudanca::Novo(NovaEntidade { camada: e.camada, cor: e.cor, geometria: transformar(&e.geometria, &acumulada), props: e.props.clone(), espaco: e.espaco.clone() }));
                    acumulada = acumulada.depois_de(t);
                }
            }
            (_, Some(t)) if copia_espelho || copia_girar => ms.push(Mudanca::Novo(NovaEntidade { camada: e.camada, cor: e.cor, geometria: transformar(&e.geometria, t), props: e.props.clone(), espaco: e.espaco.clone() })),
            (_, Some(t)) => ms.push(Mudanca::Trocar(*id, transformar(&e.geometria, t))),
            _ => return Err(format!("operação desconhecida `{op}` (mover, copiar, girar, escala, espelhar, apagar, propriedades, texto)")),
        }
    }
    let novos = doc.aplicar_mudancas(ms);
    Ok(json!({ "operacao": op, "entidades_afetadas": ids.len(), "ignoradas_por_camada_bloqueada": bloqueadas, "novas": novos }))
}

/// A mudança de `camada` e/ou `cor` de uma entidade.
fn propriedades(doc: &Documento, id: Id, args: &Value) -> Res<Mudanca> {
    let camada = match texto_opcional(args, "para_camada") {
        Some(n) => Some(doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(n)).ok_or_else(|| format!("não existe a camada `{n}`"))?),
        None => None,
    };
    let cor = texto_opcional(args, "para_cor").map(|c| if c == "por_camada" { Ok(None) } else { cor_hex(c).map(Some) }).transpose()?;
    Ok(Mudanca::Alterar { id, camada, cor, props: None })
}

/// Camadas: `listar`, `criar` (nome, cor?), `atual` (nome), `propriedade` (nome + ligada/congelada/bloqueada/cor).
pub fn camadas(doc: &mut Documento, args: &Value) -> Res<Value> {
    let acao = texto_opcional(args, "acao").unwrap_or("listar");
    let nome = texto_opcional(args, "nome");
    match (acao, nome) {
        ("listar", _) => {}
        ("criar", Some(n)) => {
            let i = camada_por_nome(doc, n)?;
            if let Some(c) = texto_opcional(args, "cor") {
                doc.definir_propriedade_de_camadas(&[i], Propriedade::Cor(cor_hex(c)?)).map_err(|e| e.chave().to_string())?;
            }
            for p in estilo_de_camada(args)? {
                doc.definir_propriedade_de_camadas(&[i], p).map_err(|e| e.chave().to_string())?;
            }
        }
        ("atual", Some(n)) => {
            let i = doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(n)).ok_or_else(|| format!("não existe a camada `{n}`"))?;
            doc.tornar_camada_atual(i).map_err(|e| e.chave().to_string())?;
        }
        ("propriedade", Some(n)) => propriedade_de_camada(doc, n, args)?,
        _ => return Err("`acao` precisa ser listar, criar, atual ou propriedade (as três últimas pedem `nome`)".into()),
    }
    let lista: Vec<Value> = doc.camadas().iter().enumerate().map(|(i, c)| json!({ "nome": c.nome, "cor": format!("#{:06X}", c.cor), "ligada": c.ligada, "congelada": c.congelada, "bloqueada": c.bloqueada, "atual": i == doc.camada_atual() })).collect();
    Ok(json!({ "camadas": lista }))
}

/// `espessura` (mm; 0 = padrão) e `tipo_linha` (continua, tracejada, oculta, eixo, pontilhada, fantasma) de uma camada.
fn estilo_de_camada(args: &Value) -> Res<Vec<Propriedade>> {
    let mut v = Vec::new();
    if let Some(e) = numero_opcional(args, "espessura") {
        if e < 0.0 {
            return Err("`espessura` não pode ser negativa".into());
        }
        v.push(Propriedade::Espessura((e > 0.0).then_some(e as f32)));
    }
    if let Some(t) = texto_opcional(args, "tipo_linha") {
        let tipo = match t.to_lowercase().as_str() {
            "continua" | "contínua" => TipoLinha::Continua,
            "tracejada" => TipoLinha::Tracejada,
            "oculta" => TipoLinha::Oculta,
            "eixo" => TipoLinha::Eixo,
            "pontilhada" => TipoLinha::Pontilhada,
            "fantasma" => TipoLinha::Fantasma,
            outro => return Err(format!("`tipo_linha` `{outro}` não vale (continua, tracejada, oculta, eixo, pontilhada, fantasma)")),
        };
        v.push(Propriedade::TipoLinha(tipo));
    }
    Ok(v)
}

fn propriedade_de_camada(doc: &mut Documento, nome: &str, args: &Value) -> Res<()> {
    let i = doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(nome)).ok_or_else(|| format!("não existe a camada `{nome}`"))?;
    let mut pedidos: Vec<Propriedade> = Vec::new();
    for (campo, faz) in [("ligada", Propriedade::Ligada as fn(bool) -> Propriedade), ("congelada", Propriedade::Congelada), ("bloqueada", Propriedade::Bloqueada)] {
        if let Some(b) = args.get(campo).and_then(Value::as_bool) {
            pedidos.push(faz(b));
        }
    }
    if let Some(c) = texto_opcional(args, "cor") {
        pedidos.push(Propriedade::Cor(cor_hex(c)?));
    }
    pedidos.extend(estilo_de_camada(args)?);
    if pedidos.is_empty() {
        return Err("diga o que mudar: ligada, congelada, bloqueada, cor, espessura ou tipo_linha".into());
    }
    for p in pedidos {
        doc.definir_propriedade_de_camadas(&[i], p).map_err(|e| e.chave().to_string())?;
    }
    Ok(())
}

/// Hachura a região fechada que contém `ponto` (linhas soltas, polilinhas e arcos que se fecham servem).
pub fn hachurar(doc: &mut Documento, args: &Value) -> Res<Value> {
    let p = ponto_de(args, "ponto")?;
    let solida = args.get("solida").and_then(Value::as_bool).unwrap_or(true);
    let escala = numero_opcional(args, "escala").unwrap_or(1.0);
    let pedido = match texto_opcional(args, "padrao") {
        Some(n) => feathercad_core::padroes::reconhecer(n).ok_or_else(|| format!("padrão `{n}` não existe (solida ou {})", feathercad_core::padroes::NOMES.join(", ").to_lowercase()))?,
        None if solida => feathercad_core::padroes::SOLIDA,
        None => feathercad_core::padroes::PADRAO_DE_FABRICA,
    };
    let g = hachura_com_padrao(doc, p, pedido, escala).map_err(|_| "não há área fechada em volta desse ponto (falta fechar o contorno? use cad_verificar)".to_string())?;
    let id = doc.adicionar_geometria(g);
    Ok(json!({ "criada": { "id": id, "tipo": "hachura", "solida": pedido == feathercad_core::padroes::SOLIDA, "padrao": pedido, "escala": escala }, "ponto": par(p) }))
}

/// Desfaz ou refaz o último passo.
pub fn desfazer(doc: &mut Documento, refazer: bool) -> Value {
    let ok = if refazer { doc.refazer() } else { doc.desfazer() };
    json!({ "feito": ok, "pode_desfazer": doc.pode_desfazer(), "pode_refazer": doc.pode_refazer() })
}

/// Aplica um texto compacto lido: declara camadas (criando/ajustando) e cria as formas, num passo só.
pub fn aplicar_compacto(doc: &mut Documento, lido: &crate::compacto::Lido) -> Res<Vec<Id>> {
    for c in &lido.camadas {
        let i = camada_por_nome(doc, &c.nome)?;
        for p in c.cor.map(Propriedade::Cor).into_iter().chain([Propriedade::Ligada(c.ligada), Propriedade::Bloqueada(c.bloqueada)]) {
            doc.definir_propriedade_de_camadas(&[i], p).map_err(|e| format!("camada `{}`: {}", c.nome, e.chave()))?;
        }
        if c.congelada {
            let _ = doc.definir_propriedade_de_camadas(&[i], Propriedade::Congelada(true));
        }
    }
    let mut ms = Vec::with_capacity(lido.formas.len());
    for (nome, cor, g) in &lido.formas {
        let camada = camada_por_nome(doc, nome)?;
        ms.push(Mudanca::Novo(NovaEntidade { camada, cor: *cor, geometria: g.clone(), props: Propriedades::default(), espaco: Espaco::default() }));
    }
    Ok(doc.aplicar_mudancas(ms))
}
