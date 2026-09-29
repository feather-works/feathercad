//! Leitura de DXF: bytes → `Documento`.

use std::collections::BTreeMap;

use feathercad_core::documento::{AlinhaH, AlinhaV, Bloco, Camada, Documento, Geometria, LinhaDePadrao, NovaEntidade, Propriedades, TipoLinha, Unidade};
use feathercad_core::geom::{normalizar_graus, Ponto, Vertice};
use feathercad_core::sessao::Mensagem;

use super::{decodificar_texto, decodificar_u, Objeto};
use crate::aci::aci_para_rgb;
use crate::Aberto;

const BINARIO: &str = "AutoCAD Binary DXF";
/// Tipos que acompanham outra entidade e não contam como entidade solta. `ATTDEF` é só o molde
/// do atributo (não aparece no desenho); o texto que se vê é o `ATTRIB` do INSERT, lido como `TEXT`.
const FILHAS: [&str; 3] = ["VERTEX", "SEQEND", "ATTDEF"];
/// `POLYLINE` de 3D (8), malha (16, 64): não são polilinhas 2D.
const POLYLINE_NAO_2D: i32 = 8 | 16 | 64;
/// Vértice de controle de spline numa `POLYLINE` ajustada: não faz parte do traçado.
const VERTICE_DE_CONTROLE: i32 = 16;

/// Bytes → texto. R2007+ é UTF-8; as versões antigas usam a página de código do
/// arquivo (`$DWGCODEPAGE`), e aqui se assume ANSI_1252 (Latin-1 na prática).
fn decodificar(bytes: &[u8]) -> String {
    let sem_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(sem_bom) {
        Ok(t) => t.to_string(),
        Err(_) => sem_bom.iter().map(|b| char::from(*b)).collect(),
    }
}

/// Divide o texto em objetos (cada um começa numa tag `0`). Comentários (`999`)
/// somem aqui. Erro de formato diz a linha.
fn objetos(texto: &str) -> Result<Vec<Objeto<'_>>, String> {
    let mut linhas = texto.lines();
    let mut lista: Vec<Objeto<'_>> = Vec::new();
    let mut numero = 0usize;
    while let Some(codigo) = linhas.next() {
        numero += 2;
        if codigo.trim().is_empty() && linhas.clone().all(|l| l.trim().is_empty()) {
            break;
        }
        let codigo: i32 = codigo.trim().parse().map_err(|_| format!("DXF inválido na linha {}: esperava um código de grupo, achei \"{}\"", numero - 1, codigo.trim()))?;
        let valor = linhas.next().ok_or_else(|| format!("DXF cortado: falta o valor do código {codigo} na linha {numero}"))?;
        match codigo {
            999 => {}
            0 => {
                if valor.trim() == "EOF" {
                    break;
                }
                lista.push(Objeto { tipo: valor.trim(), tags: Vec::new() });
            }
            _ => {
                if let Some(o) = lista.last_mut() {
                    o.tags.push((codigo, valor.trim_end_matches('\r')));
                }
            }
        }
    }
    Ok(lista)
}

fn ponto(o: &Objeto<'_>, x: i32, y: i32) -> Option<Ponto> {
    Some(Ponto::novo(o.numero(x)?, o.numero(y)?))
}

/// A cor da entidade: `None` = `PorCamada`/`PorBloco`. A cor verdadeira (420) ganha do ACI.
fn cor_da_entidade(o: &Objeto<'_>) -> Option<u32> {
    if let Some(v) = o.numero(420) {
        return Some((v as u32) & 0xFF_FF_FF);
    }
    match o.numero(62)? as i32 {
        n @ 1..=255 => Some(aci_para_rgb(n as u16)),
        _ => None,
    }
}

/// Espessura de linha (`370`, em centésimos de mm; negativo = por camada/bloco/padrão) e tipo de linha (`6`).
fn props_da(o: &Objeto<'_>) -> Propriedades {
    Propriedades {
        espessura: o.numero(370).filter(|e| *e >= 0.0).map(|e| (e / 100.0) as f32),
        tipo_linha: o.primeiro(6).and_then(TipoLinha::de_nome_dxf),
        grupo: None,
    }
}

/// A tabela LAYER: nome (sem distinção de maiúsculas) → índice.
struct Camadas {
    lista: Vec<Camada>,
    por_nome: BTreeMap<String, usize>,
}

impl Camadas {
    fn indice(&mut self, nome: &str, cor: u32, ligada: bool) -> usize {
        let chave = nome.to_lowercase();
        if let Some(i) = self.por_nome.get(&chave) {
            return *i;
        }
        self.lista.push(Camada { ligada, ..Camada::nova(nome, cor) });
        self.por_nome.insert(chave, self.lista.len() - 1);
        self.lista.len() - 1
    }

    fn da_tabela(&mut self, o: &Objeto<'_>) {
        let Some(nome) = o.primeiro(2) else { return };
        let aci = o.numero(62).unwrap_or(7.0) as i32;
        let congelada = (o.numero(70).unwrap_or(0.0) as i32) & 1 != 0;
        let cor = o.numero(420).map_or_else(|| aci_para_rgb(aci.unsigned_abs().clamp(1, 255) as u16), |v| (v as u32) & 0xFF_FF_FF);
        // Cor negativa = camada desligada.
        let i = self.indice(&decodificar_u(nome), cor, aci >= 0);
        let c = &mut self.lista[i];
        c.congelada = congelada;
        c.bloqueada = (o.numero(70).unwrap_or(0.0) as i32) & 4 != 0;
        c.plotar = o.numero(290) != Some(0.0);
        c.tipo_linha = o.primeiro(6).and_then(TipoLinha::de_nome_dxf).unwrap_or(TipoLinha::Continua);
        c.espessura = o.numero(370).filter(|e| *e >= 0.0).map(|e| (e / 100.0) as f32);
        // Transparência (440): `0x02000000 + alfa` (255 = opaca).
        c.transparencia = o.numero(440).map_or(0, |v| {
            let v = v as u32;
            if v >> 24 == 2 { (100.0 - f64::from(v & 0xFF) / 255.0 * 100.0).round().clamp(0.0, 90.0) as u8 } else { 0 }
        });
    }
}

/// Uma LINE do modelo, se estiver completa.
fn linha(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let (a, b) = (ponto(o, 10, 20)?, ponto(o, 11, 21)?);
    let camada = camadas.indice(&decodificar_u(o.primeiro(8).unwrap_or("0")), 0xFF_FF_FF, true);
    Some(NovaEntidade { camada, cor: cor_da_entidade(o), geometria: Geometria::Linha { a, b }, props: props_da(o) })
}

/// Cria a camada da entidade (ou acha a que já existe).
fn camada_da(o: &Objeto<'_>, camadas: &mut Camadas) -> usize {
    camadas.indice(&decodificar_u(o.primeiro(8).unwrap_or("0")), 0xFF_FF_FF, true)
}

fn fechada(o: &Objeto<'_>) -> bool {
    o.numero(70).is_some_and(|f| (f as i32) & 1 != 0)
}

/// Uma `LWPOLYLINE`: cada `10` abre um vértice; o `20` é o Y dele e o `42` o *bulge* dele.
fn lwpolilinha(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let mut vertices: Vec<Vertice> = Vec::new();
    for (c, v) in &o.tags {
        let n = v.trim().parse::<f64>().ok().filter(|n| n.is_finite());
        match (*c, n, vertices.last_mut()) {
            (10, Some(x), _) => vertices.push(Vertice { p: Ponto::novo(x, 0.0), bulge: 0.0 }),
            (20, Some(y), Some(u)) => u.p.y = y,
            (42, Some(b), Some(u)) => u.bulge = b,
            _ => {}
        }
    }
    (vertices.len() >= 2).then(|| NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Polilinha { vertices, fechada: fechada(o) }, props: props_da(o) })
}

/// Uma `POLYLINE` 2D (R12 e depois): os `VERTEX` que a seguem, até o `SEQEND`.
/// Devolve a entidade (se completa) e quantos objetos seguintes foram consumidos.
fn polilinha_pesada(objs: &[Objeto<'_>], i: usize, camadas: &mut Camadas) -> (Option<NovaEntidade>, usize) {
    let o = &objs[i];
    let mut vertices = Vec::new();
    let mut consumidos = 0;
    for v in objs[i + 1..].iter().take_while(|v| v.tipo == "VERTEX") {
        consumidos += 1;
        if v.numero(70).is_some_and(|f| (f as i32) & VERTICE_DE_CONTROLE != 0) {
            continue;
        }
        if let Some(p) = ponto(v, 10, 20) {
            vertices.push(Vertice { p, bulge: v.numero(42).unwrap_or(0.0) });
        }
    }
    if objs.get(i + 1 + consumidos).is_some_and(|f| f.tipo == "SEQEND") {
        consumidos += 1;
    }
    let entidade = (vertices.len() >= 2).then(|| NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Polilinha { vertices, fechada: fechada(o) }, props: props_da(o) });
    (entidade, consumidos)
}

/// Círculo e arco vivem no OCS (o sistema do objeto): com a extrusão apontando para
/// baixo (`230 < 0`), o eixo X é espelhado — o centro troca de lado e os ângulos se refletem.
fn espelhado(o: &Objeto<'_>) -> bool {
    o.numero(230).is_some_and(|z| z < 0.0)
}

/// Um `CIRCLE`.
fn circulo(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let mut centro = ponto(o, 10, 20)?;
    let raio = o.numero(40).filter(|r| *r > 0.0)?;
    if espelhado(o) {
        centro.x = -centro.x;
    }
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Circulo { centro, raio }, props: props_da(o) })
}

/// Um `ARC` (ângulos em graus, anti-horário do `50` ao `51`).
fn arco(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let mut centro = ponto(o, 10, 20)?;
    let raio = o.numero(40).filter(|r| *r > 0.0)?;
    let (mut inicio, mut fim) = (o.numero(50)?, o.numero(51)?);
    if espelhado(o) {
        centro.x = -centro.x;
        (inicio, fim) = (180.0 - fim, 180.0 - inicio);
    }
    let (inicio, fim) = (normalizar_graus(inicio), normalizar_graus(fim));
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Arco { centro, raio, inicio, fim }, props: props_da(o) })
}

/// Os alinhamentos do `TEXT` (`72` horizontal, `73` vertical).
fn alinhamento(o: &Objeto<'_>) -> (AlinhaH, AlinhaV) {
    let h = o.numero(72).map_or(0, |v| v as i32);
    let v = o.numero(73).map_or(0, |v| v as i32);
    let alinha_h = match h {
        1 | 4 => AlinhaH::Centro,
        2 => AlinhaH::Direita,
        _ => AlinhaH::Esquerda,
    };
    let alinha_v = match (h, v) {
        (4, _) | (_, 2) => AlinhaV::Meio,
        (_, 1) => AlinhaV::Baixo,
        (_, 3) => AlinhaV::Topo,
        _ => AlinhaV::Base,
    };
    (alinha_h, alinha_v)
}

/// Um `TEXT`. Com alinhamento diferente do padrão, o ponto que vale é o `11`/`21`.
fn texto(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let (h, v) = alinhamento(o);
    let alinhado = h != AlinhaH::Esquerda || v != AlinhaV::Base;
    let ponto = if alinhado { ponto(o, 11, 21).or_else(|| ponto(o, 10, 20)) } else { ponto(o, 10, 20) }?;
    let altura = o.numero(40).filter(|a| *a > 0.0)?;
    let conteudo = decodificar_texto(o.primeiro(1)?);
    Some(NovaEntidade {
        camada: camada_da(o, camadas),
        cor: cor_da_entidade(o),
        geometria: Geometria::Texto { ponto, altura, rotacao: o.numero(50).unwrap_or(0.0), conteudo, h, v }, props: props_da(o) })
}

/// Um `POINT`.
fn ponto_solto(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Ponto { p: ponto(o, 10, 20)? }, props: props_da(o) })
}

/// Os cantos `10/20`…`13/23` de um `SOLID`/`3DFACE` (o 4º pode faltar ou repetir o 3º).
fn cantos(o: &Objeto<'_>) -> Vec<Ponto> {
    let mut v: Vec<Ponto> = (0..4).filter_map(|i| ponto(o, 10 + i, 20 + i)).collect();
    if v.len() == 4 && v[3] == v[2] {
        v.pop();
    }
    v
}

/// Um `SOLID` (ou `TRACE`): área cheia. A ordem dos cantos no DXF é 1, 2, 4, 3 (em "Z").
fn solido(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let mut c = cantos(o);
    if c.len() < 3 {
        return None;
    }
    if c.len() == 4 {
        c.swap(2, 3);
    }
    let laco = c.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect();
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Hachura { contornos: vec![laco], padrao: Vec::new(), nome: "SOLID".into() }, props: props_da(o) })
}

/// Um `3DFACE`: só o contorno, projetado no plano (Z some).
fn face3d(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let c = cantos(o);
    (c.len() >= 3).then(|| NovaEntidade {
        camada: camada_da(o, camadas),
        cor: cor_da_entidade(o),
        geometria: Geometria::Polilinha { vertices: c.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada: true }, props: props_da(o) })
}

/// Uma `ELLIPSE`: centro, eixo maior (vetor), razão e parâmetros inicial/final em radianos; vira polilinha.
fn elipse(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let (centro, eixo) = (ponto(o, 10, 20)?, ponto(o, 11, 21)?);
    let razao = o.numero(40).filter(|r| *r > 0.0)?;
    let (inicio, fim) = (o.numero(41).unwrap_or(0.0), o.numero(42).unwrap_or(std::f64::consts::TAU));
    let mut varredura = (fim - inicio).rem_euclid(std::f64::consts::TAU);
    let completa = varredura < 1e-9;
    if completa {
        varredura = std::f64::consts::TAU;
    }
    let n = ((varredura.to_degrees() / 4.0).ceil() as usize).max(8);
    // Numa elipse completa o último ponto repete o primeiro: `fechada` cuida do fecho.
    let passos = if completa { n } else { n + 1 };
    let vertices = (0..passos)
        .map(|k| {
            let (s, c) = (inicio + varredura * k as f64 / n as f64).sin_cos();
            Vertice { p: Ponto::novo(centro.x + eixo.x * c - razao * eixo.y * s, centro.y + eixo.y * c + razao * eixo.x * s), bulge: 0.0 }
        })
        .collect();
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Polilinha { vertices, fechada: completa }, props: props_da(o) })
}

/// O ponto da B-spline (racional) em `u`, pelo algoritmo de de Boor. `pesos` = 1 se não racional.
fn de_boor(grau: usize, nos: &[f64], controles: &[Ponto], pesos: &[f64], u: f64) -> Ponto {
    let n = controles.len();
    // O intervalo `k` com nos[k] <= u < nos[k+1] (o último intervalo vale para u no fim).
    let k = (grau..n).rev().find(|k| nos[*k] <= u).unwrap_or(grau);
    let mut d: Vec<(f64, f64, f64)> = (0..=grau).map(|j| {
        let i = k + j - grau;
        (controles[i].x * pesos[i], controles[i].y * pesos[i], pesos[i])
    }).collect();
    for r in 1..=grau {
        for j in (r..=grau).rev() {
            let i = k + j - grau;
            let denom = nos[i + grau + 1 - r] - nos[i];
            let a = if denom.abs() < 1e-14 { 0.0 } else { (u - nos[i]) / denom };
            d[j] = ((1.0 - a) * d[j - 1].0 + a * d[j].0, (1.0 - a) * d[j - 1].1 + a * d[j].1, (1.0 - a) * d[j - 1].2 + a * d[j].2);
        }
    }
    let (x, y, w) = d[grau];
    if w.abs() < 1e-14 { Ponto::novo(x, y) } else { Ponto::novo(x / w, y / w) }
}

/// Uma `SPLINE`: a curva B-spline amostrada (sem pontos de controle suficientes, passa pelos pontos de ajuste).
fn spline(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let (mut nos, mut pesos): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    let (mut controles, mut ajuste): (Vec<Ponto>, Vec<Ponto>) = (Vec::new(), Vec::new());
    for (c, v) in &o.tags {
        let Some(n) = v.trim().parse::<f64>().ok().filter(|n| n.is_finite()) else { continue };
        match (*c, controles.last_mut(), ajuste.last_mut()) {
            (40, ..) => nos.push(n),
            (41, ..) => pesos.push(n),
            (10, ..) => controles.push(Ponto::novo(n, 0.0)),
            (20, Some(p), _) => p.y = n,
            (11, ..) => ajuste.push(Ponto::novo(n, 0.0)),
            (21, _, Some(p)) => p.y = n,
            _ => {}
        }
    }
    let grau = o.numero(71).map_or(3, |g| g as usize).max(1);
    let fechada = o.numero(70).is_some_and(|f| (f as i32) & 1 != 0);
    let n = controles.len();
    let vertices: Vec<Ponto> = if n > grau && nos.len() == n + grau + 1 {
        if pesos.len() != n {
            pesos = vec![1.0; n];
        }
        let (u0, u1) = (nos[grau], nos[n]);
        let amostras = (n * 12).clamp(32, 3000);
        (0..=amostras).map(|k| de_boor(grau, &nos, &controles, &pesos, u0 + (u1 - u0) * k as f64 / amostras as f64 * (1.0 - 1e-12))).collect()
    } else if ajuste.len() >= 2 {
        ajuste
    } else if n >= 2 {
        controles
    } else {
        return None;
    };
    Some(NovaEntidade {
        camada: camada_da(o, camadas),
        cor: cor_da_entidade(o),
        geometria: Geometria::Polilinha { vertices: vertices.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada }, props: props_da(o) })
}

/// Uma chamada de um `MULTILEADER`: linhas até o ponto de pouso e o pedaço horizontal (dogleg).
#[derive(Default)]
struct Chamada {
    pouso: Option<Ponto>,
    direcao: (f64, f64),
    comprimento: f64,
    linhas: Vec<Vec<Ponto>>,
}

/// O que se coleta de um `MULTILEADER`.
#[derive(Default)]
struct Coleta {
    chamadas: Vec<Chamada>,
    altura: Option<f64>,
    lugar: (Option<f64>, Option<f64>),
    conteudo: Option<String>,
}

fn numero(v: &str) -> Option<f64> {
    v.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Uma tag dentro de `LEADER{ … }`, fora das linhas: pouso (`10`/`20`), direção (`11`/`21`), comprimento (`40`).
fn tag_da_chamada(a: &mut Chamada, codigo: i32, v: &str) -> Option<()> {
    let n = numero(v)?;
    match codigo {
        10 => a.pouso = Some(Ponto::novo(n, 0.0)),
        20 => a.pouso.as_mut()?.y = n,
        11 => a.direcao.0 = n,
        21 => a.direcao.1 = n,
        40 => a.comprimento = n,
        _ => {}
    }
    Some(())
}

/// Percorre as tags do `MULTILEADER` (estado: dentro de uma chamada? dentro de uma linha dela?).
fn coletar_multileader(o: &Objeto<'_>) -> Option<Coleta> {
    let mut c = Coleta::default();
    let (mut atual, mut linha): (Option<Chamada>, Option<Vec<Ponto>>) = (None, None);
    for (codigo, v) in &o.tags {
        let v = v.trim();
        match (*codigo, v) {
            (302, "LEADER{") => atual = Some(Chamada::default()),
            (303, _) => c.chamadas.extend(atual.take()),
            (304, "LEADER_LINE{") => linha = Some(Vec::new()),
            (305, _) => {
                if let (Some(l), Some(a)) = (linha.take(), atual.as_mut()) {
                    a.linhas.push(l);
                }
            }
            (10, _) if linha.is_some() => linha.as_mut()?.push(Ponto::novo(numero(v)?, 0.0)),
            (20, _) if linha.is_some() => linha.as_mut()?.last_mut()?.y = numero(v)?,
            (_, _) if atual.is_some() => tag_da_chamada(atual.as_mut()?, *codigo, v)?,
            (41, _) if c.altura.is_none() => c.altura = numero(v),
            (12, _) => c.lugar.0 = numero(v),
            (22, _) => c.lugar.1 = numero(v),
            (304, texto) if c.conteudo.is_none() => c.conteudo = Some(texto.to_string()),
            _ => {}
        }
    }
    Some(c)
}

/// Um `MULTILEADER` (chamada): as linhas de cada seta até o ponto de pouso, o dogleg e o texto.
/// Sem a ponta de seta e sem estilo; chamada com bloco no lugar do texto só mostra as linhas.
fn multileader(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<Vec<NovaEntidade>> {
    let Coleta { chamadas, altura, lugar, conteudo } = coletar_multileader(o)?;
    let camada = camada_da(o, camadas);
    let cor = cor_da_entidade(o);
    let mut feitas = Vec::new();
    for ch in chamadas {
        for mut traco in ch.linhas {
            if let Some(pouso) = ch.pouso {
                traco.push(pouso);
                if ch.comprimento > 0.0 {
                    traco.push(Ponto::novo(pouso.x + ch.direcao.0 * ch.comprimento, pouso.y + ch.direcao.1 * ch.comprimento));
                }
            }
            if traco.len() >= 2 {
                feitas.push(NovaEntidade { camada, cor, geometria: Geometria::Polilinha { vertices: traco.into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect(), fechada: false }, props: props_da(o) });
            }
        }
    }
    if let (Some(texto), Some(altura), (Some(x), Some(y))) = (conteudo, altura.filter(|a| *a > 0.0), lugar) {
        let passo = altura * ENTRELINHA_MTEXT;
        for (i, conteudo) in limpar_mtext(&decodificar_texto(&texto)).into_iter().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            feitas.push(NovaEntidade {
                camada,
                cor,
                geometria: Geometria::Texto { ponto: Ponto::novo(x, y - i as f64 * passo), altura, rotacao: 0.0, conteudo, h: AlinhaH::Esquerda, v: AlinhaV::Meio }, props: props_da(o) });
        }
    }
    (!feitas.is_empty()).then_some(feitas)
}

/// Quem lê as tags de um `HATCH` na ordem em que vêm (o DXF repete os mesmos códigos em
/// contextos diferentes, então a leitura é sequencial, não por busca).
struct Leitor<'a, 'b> {
    tags: &'b [(i32, &'a str)],
    i: usize,
}

impl Leitor<'_, '_> {
    /// O próximo valor, se a próxima tag tem este código.
    fn pegar(&mut self, codigo: i32) -> Option<f64> {
        let (c, v) = self.tags.get(self.i)?;
        if *c != codigo {
            return None;
        }
        self.i += 1;
        v.trim().parse::<f64>().ok().filter(|n| n.is_finite())
    }

    fn espiar(&self) -> Option<i32> {
        self.tags.get(self.i).map(|t| t.0)
    }

    /// Avança até a próxima tag com este código (e passa por ela). `None` se não há mais.
    fn ate(&mut self, codigo: i32) -> Option<f64> {
        while self.i < self.tags.len() && self.tags[self.i].0 != codigo {
            self.i += 1;
        }
        self.pegar(codigo)
    }

    fn ponto(&mut self, x: i32, y: i32) -> Option<Ponto> {
        Some(Ponto::novo(self.pegar(x)?, self.pegar(y)?))
    }
}

/// Uma aresta de contorno de hachura, amostrada em pontos (arco, elipse; spline pelo polígono de controle).
fn aresta_da_hachura(r: &mut Leitor<'_, '_>, tipo: i32, pts: &mut Vec<Vertice>) -> Option<()> {
    let mut poe = |p: Ponto| pts.push(Vertice { p, bulge: 0.0 });
    match tipo {
        1 => {
            poe(r.ponto(10, 20)?);
            poe(r.ponto(11, 21)?);
        }
        2 | 3 => {
            let centro = r.ponto(10, 20)?;
            // Círculo é elipse de eixo maior `raio` e razão 1.
            let (eixo, razao) = if tipo == 3 {
                let e = r.ponto(11, 21)?;
                (e, r.pegar(40)?)
            } else {
                (Ponto::novo(r.pegar(40)?, 0.0), 1.0)
            };
            let (a1, a2) = (r.pegar(50)?, r.pegar(51)?);
            let anti_horario = r.pegar(73) != Some(0.0);
            let mut varredura = if anti_horario { normalizar_graus(a2 - a1) } else { -normalizar_graus(a1 - a2) };
            if varredura == 0.0 {
                varredura = if anti_horario { 360.0 } else { -360.0 };
            }
            let n = ((varredura.abs() / 5.0).ceil() as usize).max(4);
            for k in 0..=n {
                let ang = (a1 + varredura * k as f64 / n as f64).to_radians();
                let (s, c) = ang.sin_cos();
                poe(Ponto::novo(centro.x + eixo.x * c - razao * eixo.y * s, centro.y + eixo.y * c + razao * eixo.x * s));
            }
        }
        4 => {
            let (nos, controles) = ({ r.pegar(94)?; r.pegar(73)?; r.pegar(74)?; r.pegar(95)? as usize }, r.pegar(96)? as usize);
            for _ in 0..nos {
                r.pegar(40)?;
            }
            for _ in 0..controles {
                poe(r.ponto(10, 20)?);
                r.pegar(42);
            }
            // Pontos de ajuste e tangentes: não usados.
            let ajuste = r.pegar(97).unwrap_or(0.0) as usize;
            for _ in 0..ajuste {
                r.ponto(11, 21)?;
            }
            while matches!(r.espiar(), Some(12 | 22 | 13 | 23)) {
                r.i += 1;
            }
        }
        _ => return None,
    }
    Some(())
}

/// Um `HATCH`: os contornos (par-ímpar) e, se não for sólido, as linhas do padrão.
/// Devolve `None` se o contorno não pôde ser lido.
fn hachura(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<NovaEntidade> {
    let mut r = Leitor { tags: &o.tags, i: 0 };
    let nome = o.primeiro(2).map_or_else(String::new, decodificar_u);
    let solido = o.numero(70).is_some_and(|f| f as i32 == 1);
    let caminhos = r.ate(91)? as usize;
    let mut contornos: Vec<Vec<Vertice>> = Vec::new();
    for _ in 0..caminhos {
        let bandeira = r.pegar(92)? as i32;
        let mut laco: Vec<Vertice> = Vec::new();
        if bandeira & 2 != 0 {
            let com_bulge = r.pegar(72)? != 0.0;
            r.pegar(73);
            let n = r.pegar(93)? as usize;
            for _ in 0..n {
                let p = r.ponto(10, 20)?;
                let bulge = if com_bulge { r.pegar(42).unwrap_or(0.0) } else { 0.0 };
                laco.push(Vertice { p, bulge });
            }
        } else {
            let arestas = r.pegar(93)? as usize;
            for _ in 0..arestas {
                let tipo = r.pegar(72)? as i32;
                aresta_da_hachura(&mut r, tipo, &mut laco)?;
            }
        }
        // Objetos de origem (`97` + `330`…): não usados.
        r.pegar(97);
        while r.pegar(330).is_some() {}
        laco.dedup_by(|b, a| (a.p.x - b.p.x).abs() < 1e-12 && (a.p.y - b.p.y).abs() < 1e-12 && a.bulge == 0.0);
        // Dois vértices com bulge formam um círculo (duas meias-voltas).
        if laco.len() >= 3 || (laco.len() == 2 && laco.iter().any(|v| v.bulge != 0.0)) {
            contornos.push(laco);
        }
    }
    if contornos.is_empty() {
        return None;
    }
    let mut padrao = Vec::new();
    if !solido && r.ate(78).is_some_and(|n| n > 0.0) {
        r.i -= 1;
        let n = r.pegar(78)? as usize;
        for _ in 0..n {
            let angulo = r.pegar(53)?;
            let base = r.ponto(43, 44)?;
            let (dx, dy) = (r.pegar(45)?, r.pegar(46)?);
            let mut tracos = Vec::new();
            for _ in 0..r.pegar(79)? as usize {
                tracos.extend(r.pegar(49));
            }
            padrao.push(LinhaDePadrao { angulo, base, dx, dy, tracos });
        }
    }
    Some(NovaEntidade { camada: camada_da(o, camadas), cor: cor_da_entidade(o), geometria: Geometria::Hachura { contornos, padrao, nome }, props: props_da(o) })
}

/// Uma matriz de INSERT (`MINSERT`) tem no máximo tantas cópias (proteção contra arquivo absurdo).
const MATRIZ_MAXIMA: i64 = 5000;

/// Espaço entre linhas de um `MTEXT`, em alturas de texto (o "espaçamento simples" do AutoCAD).
const ENTRELINHA_MTEXT: f64 = 5.0 / 3.0;

/// O texto de um `MTEXT` sem os códigos de formatação, uma `String` por linha (`\P`).
/// Tira fonte/cor/altura (`\f…;`, `\C…;`), grupos `{}`, sublinhado etc.; fração empilhada `\S1/2;` vira `1/2`.
fn limpar_mtext(bruto: &str) -> Vec<String> {
    let mut linhas = vec![String::new()];
    let mut it = bruto.chars().peekable();
    while let Some(c) = it.next() {
        let linha = linhas.last_mut().expect("sempre há uma linha");
        match c {
            '{' | '}' => {}
            '\\' => match it.next() {
                Some('P' | 'N') => linhas.push(String::new()),
                Some('~') => linha.push(' '),
                Some(c @ ('\\' | '{' | '}')) => linha.push(c),
                Some('S') => {
                    let mut pilha = String::new();
                    for c in it.by_ref() {
                        if c == ';' {
                            break;
                        }
                        pilha.push(c);
                    }
                    linha.push_str(&pilha.replace(['^', '#'], "/"));
                }
                // Códigos com argumento até `;`: fonte, cor, altura, largura, inclinação, espaçamento, alinhamento.
                Some('f' | 'F' | 'C' | 'c' | 'H' | 'h' | 'W' | 'w' | 'Q' | 'q' | 'T' | 't' | 'A' | 'a' | 'p') => {
                    for c in it.by_ref() {
                        if c == ';' {
                            break;
                        }
                    }
                }
                // Liga/desliga (sublinhado, tachado, sobrelinha) e o `\X` de continuação: sem efeito no texto.
                Some('L' | 'l' | 'O' | 'o' | 'K' | 'k' | 'X') => {}
                Some(outro) => {
                    linha.push('\\');
                    linha.push(outro);
                }
                None => {}
            },
            // `^I` é a tabulação (notação com circunflexo): vira espaço.
            '^' if it.peek() == Some(&'I') => {
                it.next();
                linha.push(' ');
            }
            c => linha.push(c),
        }
    }
    linhas
}

/// Um `MTEXT` vira uma entidade `Texto` por linha (o motor só tem texto de uma linha).
/// O ponto de ancoragem (`71`: 1–9) decide como o bloco de linhas se apoia no ponto `10`/`20`.
fn mtext(o: &Objeto<'_>, camadas: &mut Camadas) -> Option<Vec<NovaEntidade>> {
    let (p, altura) = (ponto(o, 10, 20)?, o.numero(40).filter(|a| *a > 0.0)?);
    // O texto vem em pedaços de até 250 letras (`3`, na ordem) e o último em `1`.
    let mut bruto = String::new();
    for (c, v) in &o.tags {
        if *c == 3 || *c == 1 {
            bruto.push_str(v);
        }
    }
    let linhas = limpar_mtext(&decodificar_texto(&bruto));
    let rotacao = o.numero(50).or_else(|| Some(o.numero(21)?.atan2(o.numero(11)?).to_degrees())).unwrap_or(0.0);
    let ancora = (o.numero(71).map_or(1, |a| a as i32) - 1).clamp(0, 8);
    let h = [AlinhaH::Esquerda, AlinhaH::Centro, AlinhaH::Direita][(ancora % 3) as usize];
    let v = [AlinhaV::Topo, AlinhaV::Meio, AlinhaV::Baixo][(ancora / 3) as usize];
    let passo = altura * ENTRELINHA_MTEXT;
    let (seno, cosseno) = rotacao.to_radians().sin_cos();
    let n = linhas.len() as f64;
    let camada = camada_da(o, camadas);
    let cor = cor_da_entidade(o);
    let feitas = linhas
        .into_iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, conteudo)| {
            // Quanto esta linha fica abaixo do ponto (para baixo do texto = (sen, -cos)).
            let abaixo = match v {
                AlinhaV::Meio => i as f64 - (n - 1.0) / 2.0,
                AlinhaV::Baixo => i as f64 - (n - 1.0),
                _ => i as f64,
            } * passo;
            let ponto = Ponto::novo(p.x + seno * abaixo, p.y - cosseno * abaixo);
            NovaEntidade { camada, cor, geometria: Geometria::Texto { ponto, altura, rotacao, conteudo, h, v }, props: props_da(o) }
        })
        .collect();
    Some(feitas)
}

/// O que foi visto e não entra no documento, para virar aviso.
#[derive(Default)]
struct Ignorados {
    por_tipo: BTreeMap<String, usize>,
    papel: usize,
}

/// Tudo que a leitura vai acumulando: camadas, o mapa dos blocos e o que foi deixado de fora.
struct Leitura {
    camadas: Camadas,
    /// Nome do bloco (minúsculo) → índice em `Documento::blocos()`.
    blocos: BTreeMap<String, usize>,
    ignorados: Ignorados,
    /// O que foi ignorado **dentro** de definições de bloco (aviso à parte).
    ignorados_de_blocos: Ignorados,
}

/// Blocos do próprio espaço modelo/papel: suas entidades já vêm em ENTITIES.
fn bloco_de_espaco(nome: &str) -> bool {
    let n = nome.to_lowercase();
    n.starts_with("*model_space") || n.starts_with("*paper_space") || n == "$model_space" || n == "$paper_space"
}

/// 1ª passada: numera cada `BLOCK` da seção BLOCKS (fora os do espaço modelo/papel),
/// para que um INSERT ache um bloco definido mais adiante.
fn numerar_blocos(objs: &[Objeto<'_>]) -> BTreeMap<String, usize> {
    let mut mapa = BTreeMap::new();
    let (mut secao, mut n) = ("", 0usize);
    for o in objs {
        match o.tipo {
            "SECTION" => secao = o.primeiro(2).unwrap_or(""),
            "ENDSEC" => secao = "",
            "BLOCK" if secao == "BLOCKS" => {
                let nome = decodificar_u(o.primeiro(2).unwrap_or(""));
                if !bloco_de_espaco(&nome) {
                    mapa.entry(nome.to_lowercase()).or_insert(n);
                    n += 1;
                }
            }
            _ => {}
        }
    }
    mapa
}

/// Um `INSERT` (ou o bloco anônimo de uma `DIMENSION`, em `nome` e sem transformação).
fn insercao(o: &Objeto<'_>, l: &mut Leitura, nome: &str, cota: bool) -> Option<NovaEntidade> {
    insercoes(o, l, nome, cota).and_then(|v| v.into_iter().next())
}

/// Uma tabela do AutoCAD: o desenho já vem pronto no bloco anônimo, posto no ponto da tabela.
fn tabela(o: &Objeto<'_>, l: &mut Leitura, nome: &str) -> Option<NovaEntidade> {
    let bloco = *l.blocos.get(&decodificar_u(nome).to_lowercase())?;
    let camada = camada_da(o, &mut l.camadas);
    let ponto = ponto(o, 10, 20).unwrap_or(Ponto::novo(0.0, 0.0));
    Some(NovaEntidade { camada, cor: cor_da_entidade(o), geometria: Geometria::Insercao { bloco, ponto, escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: props_da(o) })
}

/// O `INSERT` — ou, num `MINSERT` (`70` colunas × `71` linhas, espaços `44`/`45`), a matriz toda.
fn insercoes(o: &Objeto<'_>, l: &mut Leitura, nome: &str, cota: bool) -> Option<Vec<NovaEntidade>> {
    let bloco = *l.blocos.get(&decodificar_u(nome).to_lowercase())?;
    let camada = camada_da(o, &mut l.camadas);
    let cor = cor_da_entidade(o);
    if cota {
        return Some(vec![NovaEntidade { camada, cor, geometria: Geometria::Insercao { bloco, ponto: Ponto::novo(0.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: props_da(o) }]);
    }
    let base = ponto(o, 10, 20).unwrap_or(Ponto::novo(0.0, 0.0));
    let (escala_x, escala_y, rotacao) = (o.numero(41).unwrap_or(1.0), o.numero(42).unwrap_or(1.0), o.numero(50).unwrap_or(0.0));
    let (colunas, linhas) = (o.numero(70).map_or(1, |n| (n as i64).clamp(1, MATRIZ_MAXIMA)), o.numero(71).map_or(1, |n| (n as i64).clamp(1, MATRIZ_MAXIMA)));
    let (espaco_c, espaco_l) = (o.numero(44).unwrap_or(0.0), o.numero(45).unwrap_or(0.0));
    let (seno, cosseno) = rotacao.to_radians().sin_cos();
    let mut v = Vec::new();
    'matriz: for r in 0..linhas {
        for c in 0..colunas {
            if v.len() as i64 >= MATRIZ_MAXIMA {
                break 'matriz;
            }
            // O deslocamento acompanha o giro do próprio INSERT.
            let (dx, dy) = (c as f64 * espaco_c, r as f64 * espaco_l);
            let ponto = Ponto::novo(base.x + dx * cosseno - dy * seno, base.y + dx * seno + dy * cosseno);
            v.push(NovaEntidade { camada, cor, geometria: Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao }, props: props_da(o) });
        }
    }
    Some(v)
}

fn avisos(ig: &Ignorados, de_blocos: &Ignorados) -> Vec<Mensagem> {
    let mut v = Vec::new();
    let nb: usize = de_blocos.por_tipo.values().sum();
    if nb > 0 {
        let tipos: Vec<String> = de_blocos.por_tipo.iter().map(|(t, q)| format!("{q} {t}")).collect();
        v.push(Mensagem::nova("aviso.dentroDeBlocos").com("n", &nb.to_string()).com("tipos", &tipos.join(", ")));
    }
    let n: usize = ig.por_tipo.values().sum();
    if n > 0 {
        let tipos: Vec<String> = ig.por_tipo.iter().map(|(t, q)| format!("{q} {t}")).collect();
        v.push(Mensagem::nova("aviso.entidadesIgnoradas").com("n", &n.to_string()).com("tipos", &tipos.join(", ")));
    }
    if ig.papel > 0 {
        v.push(Mensagem::nova("aviso.espacoPapelIgnorado").com("n", &ig.papel.to_string()));
    }
    v
}

/// O `$ACADVER` (`AC1009`…) das variáveis do cabeçalho.
/// O `$INSUNITS` do cabeçalho, se for uma unidade que o app trata (mm, cm, m).
fn unidade_do_cabecalho(o: &Objeto<'_>) -> Option<Unidade> {
    let mut atual = "";
    for (c, v) in &o.tags {
        match *c {
            9 => atual = v,
            70 if atual == "$INSUNITS" => return v.trim().parse().ok().and_then(Unidade::de_codigo_dxf),
            _ => {}
        }
    }
    None
}

fn versao_do_cabecalho(o: &Objeto<'_>) -> Option<String> {
    let mut atual = "";
    for (c, v) in &o.tags {
        match *c {
            9 => atual = v,
            1 if atual == "$ACADVER" => return Some(v.trim().to_string()),
            _ => {}
        }
    }
    None
}

fn ignorar(ig: &mut Ignorados, tipo: &str) {
    *ig.por_tipo.entry(tipo.to_string()).or_default() += 1;
}

/// Trata a entidade `objs[i]`: acrescenta ao desenho (ou ao bloco, se `dentro`) ou conta
/// como ignorada. Devolve quantos objetos **seguintes** ela consumiu (os `VERTEX` de uma `POLYLINE`).
fn entidade(objs: &[Objeto<'_>], i: usize, l: &mut Leitura, saida: &mut Vec<NovaEntidade>, dentro: bool) -> usize {
    let o = &objs[i];
    if !dentro && o.numero(67).is_some_and(|v| v as i32 == 1) {
        l.ignorados.papel += 1;
        return if o.tipo == "POLYLINE" { polilinha_pesada(objs, i, &mut l.camadas).1 } else { 0 };
    }
    let camadas = &mut l.camadas;
    let (feita, consumidos) = match o.tipo {
        "LINE" => (linha(o, camadas), 0),
        "LWPOLYLINE" => (lwpolilinha(o, camadas), 0),
        "TEXT" => (texto(o, camadas), 0),
        "HATCH" => (hachura(o, camadas), 0),
        "MTEXT" => {
            // Sem ponto ou altura é incompleto; texto vazio (coluna em branco) não é erro.
            match mtext(o, camadas) {
                Some(linhas) => saida.extend(linhas),
                None => {
                    let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
                    ignorar(ig, "MTEXT (incompleta)");
                }
            }
            return 0;
        }
        // Atributo: o valor já está no mundo (não no bloco), então vira um texto solto. Invisível (70 bit 1) fica de fora.
        "ATTRIB" if o.numero(70).is_some_and(|f| (f as i32) & 1 != 0) => return 0,
        "ATTRIB" => (texto(o, camadas), 0),
        "CIRCLE" => (circulo(o, camadas), 0),
        "ARC" => (arco(o, camadas), 0),
        "INSERT" => match o.primeiro(2).and_then(|nome| insercoes(o, l, nome, false)) {
            Some(v) => {
                saida.extend(v);
                return 0;
            }
            None => {
                let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
                ignorar(ig, "INSERT (bloco inexistente)");
                return 0;
            }
        },
        "ACAD_TABLE" => match o.primeiro(2).and_then(|nome| tabela(o, l, nome)) {
            Some(e) => (Some(e), 0),
            None => {
                let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
                ignorar(ig, "ACAD_TABLE (sem bloco)");
                return 0;
            }
        },
        "DIMENSION" | "ARC_DIMENSION" | "LARGE_RADIAL_DIMENSION" => match o.primeiro(2).and_then(|nome| insercao(o, l, nome, true)) {
            Some(e) => (Some(e), 0),
            None => {
                let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
                ignorar(ig, &format!("{} (sem bloco)", o.tipo));
                return 0;
            }
        },
        "POINT" => (ponto_solto(o, camadas), 0),
        "SOLID" | "TRACE" => (solido(o, camadas), 0),
        "3DFACE" => (face3d(o, camadas), 0),
        "ELLIPSE" => (elipse(o, camadas), 0),
        "SPLINE" => (spline(o, camadas), 0),
        "MULTILEADER" => {
            match multileader(o, camadas) {
                Some(v) => saida.extend(v),
                None => {
                    let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
                    ignorar(ig, "MULTILEADER (incompleta)");
                }
            }
            return 0;
        }
        "POLYLINE" if o.numero(70).map_or(true, |f| (f as i32) & POLYLINE_NAO_2D == 0) => polilinha_pesada(objs, i, camadas),
        "POLYLINE" => {
            let consumidos = polilinha_pesada(objs, i, camadas).1;
            let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
            ignorar(ig, "POLYLINE (3D ou malha)");
            return consumidos;
        }
        outro => {
            let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
            ignorar(ig, outro);
            return 0;
        }
    };
    match feita {
        Some(e) => saida.push(e),
        None => {
            let ig = if dentro { &mut l.ignorados_de_blocos } else { &mut l.ignorados };
            ignorar(ig, &format!("{} (incompleta)", o.tipo));
        }
    }
    consumidos
}

/// Lê um DXF de texto.
pub fn ler(bytes: &[u8]) -> Result<Aberto, String> {
    if bytes.starts_with(BINARIO.as_bytes()) {
        return Err("Este é um DXF binário; por enquanto só abro DXF de texto. Salve-o como DXF ASCII no programa de origem.".into());
    }
    let texto = decodificar(bytes);
    let objs = objetos(&texto)?;
    if objs.is_empty() {
        return Err("O arquivo não parece um DXF (nenhuma seção encontrada).".into());
    }
    let mut l = Leitura {
        camadas: Camadas { lista: Vec::new(), por_nome: BTreeMap::new() },
        blocos: numerar_blocos(&objs),
        ignorados: Ignorados::default(),
        ignorados_de_blocos: Ignorados::default(),
    };
    let mut entidades = Vec::new();
    let mut blocos: Vec<Bloco> = Vec::new();
    // Dentro de um BLOCK: `Some(true)` = definição que entra em `blocos`; `Some(false)` = do espaço modelo/papel.
    let mut em_bloco: Option<bool> = None;
    let mut secao = "";
    let mut versao = None;
    let mut unidade = None;
    let mut i = 0;
    while i < objs.len() {
        let o = &objs[i];
        i += 1;
        match o.tipo {
            "SECTION" => {
                secao = o.primeiro(2).unwrap_or("");
                // O HEADER não tem objetos `0`: suas variáveis (`9 $NOME` + valor) ficam na própria SECTION.
                if secao == "HEADER" {
                    versao = versao_do_cabecalho(o);
                    unidade = unidade_do_cabecalho(o);
                }
            }
            "ENDSEC" => {
                secao = "";
                em_bloco = None;
            }
            "LAYER" if secao == "TABLES" => l.camadas.da_tabela(o),
            "BLOCK" if secao == "BLOCKS" => {
                let nome = decodificar_u(o.primeiro(2).unwrap_or(""));
                em_bloco = Some(!bloco_de_espaco(&nome));
                if em_bloco == Some(true) {
                    let base = ponto(o, 10, 20).unwrap_or(Ponto::novo(0.0, 0.0));
                    blocos.push(Bloco { nome, base, entidades: Vec::new() });
                }
            }
            "ENDBLK" if secao == "BLOCKS" => em_bloco = None,
            t if secao == "BLOCKS" && !FILHAS.contains(&t) => {
                if em_bloco == Some(true) {
                    let mut dentro = Vec::new();
                    i += entidade(objs.as_slice(), i - 1, &mut l, &mut dentro, true);
                    if let Some(b) = blocos.last_mut() {
                        b.entidades.append(&mut dentro);
                    }
                }
            }
            t if secao == "ENTITIES" && !FILHAS.contains(&t) => {
                i += entidade(objs.as_slice(), i - 1, &mut l, &mut entidades, false);
            }
            _ => {}
        }
    }
    let atual = l.camadas.por_nome.get("0").copied().unwrap_or(0);
    let avisos = avisos(&l.ignorados, &l.ignorados_de_blocos);
    Ok(Aberto { doc: Documento::de_partes_com_blocos(l.camadas.lista, atual, blocos, entidades).com_unidade(unidade), avisos, versao_dxf: versao })
}
