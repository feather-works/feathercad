//! `cad_verificar {leiaute: true}`: o que a IA só descobria olhando a imagem — texto sobre texto ou sobre linha,
//! texto fora da folha ou entre duas folhas, texto de cabeça para baixo. Tudo em números, com id e posição.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria, Id};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{arred, par};

/// Quantos problemas de cada tipo entram na lista (o resto vai só na contagem).
const LIMITE_POR_TIPO: usize = 40;

/// Um texto como retângulo girado: centro, metades e o giro.
struct Caixa {
    id: Id,
    cantos: [Ponto; 4],
    min: Ponto,
    max: Ponto,
    rotacao: f64,
}

/// O retângulo que o texto ocupa de verdade (largura pela métrica do PDF, altura de maiúscula a descendente).
pub fn caixa_do_texto(ponto: Ponto, altura: f64, rotacao: f64, conteudo: &str, h: AlinhaH, v: AlinhaV) -> [Ponto; 4] {
    let largura = feathercad_io::pdf::largura_do_conteudo(conteudo, altura);
    let x0 = match h {
        AlinhaH::Esquerda => 0.0,
        AlinhaH::Centro => -largura / 2.0,
        AlinhaH::Direita => -largura,
    };
    let (y0, y1) = match v {
        AlinhaV::Base => (0.0, altura),
        AlinhaV::Baixo => (0.0, altura),
        AlinhaV::Meio => (-altura / 2.0, altura / 2.0),
        AlinhaV::Topo => (-altura, 0.0),
    };
    let (s, c) = rotacao.to_radians().sin_cos();
    [(x0, y0), (x0 + largura, y0), (x0 + largura, y1), (x0, y1)].map(|(dx, dy)| Ponto::novo(ponto.x + dx * c - dy * s, ponto.y + dx * s + dy * c))
}

fn caixas(d: &Documento) -> Vec<Caixa> {
    d.entidades()
        .iter()
        .filter_map(|e| {
            let Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } = &e.geometria else { return None };
            let cantos = caixa_do_texto(*ponto, *altura, *rotacao, conteudo, *h, *v);
            let (xs, ys): (Vec<f64>, Vec<f64>) = cantos.iter().map(|p| (p.x, p.y)).unzip();
            let min = Ponto::novo(xs.iter().copied().fold(f64::MAX, f64::min), ys.iter().copied().fold(f64::MAX, f64::min));
            let max = Ponto::novo(xs.iter().copied().fold(f64::MIN, f64::max), ys.iter().copied().fold(f64::MIN, f64::max));
            Some(Caixa { id: e.id, cantos, min, max, rotacao: *rotacao })
        })
        .collect()
}

/// Encolhe o retângulo 10 % para o lado (texto que só encosta não conta como sobreposto).
fn encolher(c: &[Ponto; 4]) -> [Ponto; 4] {
    let m = Ponto::novo(c.iter().map(|p| p.x).sum::<f64>() / 4.0, c.iter().map(|p| p.y).sum::<f64>() / 4.0);
    c.map(|p| Ponto::novo(m.x + (p.x - m.x) * 0.9, m.y + (p.y - m.y) * 0.9))
}

/// Teorema do eixo separador entre dois retângulos (convexos).
fn se_cruzam(a: &[Ponto; 4], b: &[Ponto; 4]) -> bool {
    for poli in [a, b] {
        for i in 0..4 {
            let (p, q) = (poli[i], poli[(i + 1) % 4]);
            let eixo = (q.y - p.y, p.x - q.x);
            let proj = |f: &[Ponto; 4]| {
                let v: Vec<f64> = f.iter().map(|r| r.x * eixo.0 + r.y * eixo.1).collect();
                (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max))
            };
            let ((a0, a1), (b0, b1)) = (proj(a), proj(b));
            if a1 <= b0 || b1 <= a0 {
                return false;
            }
        }
    }
    true
}

fn segmento_toca(a: Ponto, b: Ponto, r: &[Ponto; 4]) -> bool {
    let dentro = |p: Ponto| {
        let pos = |i: usize| {
            let (u, w) = (r[i], r[(i + 1) % 4]);
            (w.x - u.x) * (p.y - u.y) - (w.y - u.y) * (p.x - u.x)
        };
        (0..4).all(|i| pos(i) >= 0.0) || (0..4).all(|i| pos(i) <= 0.0)
    };
    if dentro(a) || dentro(b) {
        return true;
    }
    let cruz = |o: Ponto, p: Ponto, q: Ponto| (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x);
    (0..4).any(|i| {
        let (c, e) = (r[i], r[(i + 1) % 4]);
        cruz(a, b, c) * cruz(a, b, e) < 0.0 && cruz(c, e, a) * cruz(c, e, b) < 0.0
    })
}

/// Giro normalizado em [0, 360).
fn normalizado(g: f64) -> f64 {
    g.rem_euclid(360.0)
}

pub fn verificar_leiaute(d: &Documento) -> Value {
    verificar_leiaute_com(d, LIMITE_POR_TIPO)
}

/// Como [`verificar_leiaute`], com `limite` problemas listados por tipo (a contagem é sempre a total).
pub fn verificar_leiaute_com(d: &Documento, limite: usize) -> Value {
    let textos = caixas(d);
    let folhas = d.folhas_inseridas();
    let mut lista: Vec<Value> = Vec::new();
    let mut contagem = std::collections::BTreeMap::<&str, usize>::new();
    let mut anotar = |tipo: &'static str, v: Value, lista: &mut Vec<Value>| {
        let n = contagem.entry(tipo).or_insert(0);
        *n += 1;
        if *n <= limite {
            lista.push(v);
        }
    };

    for t in &textos {
        let g = normalizado(t.rotacao);
        if g > 90.0 + 1e-6 && g < 270.0 - 1e-6 {
            anotar("texto_de_cabeca_para_baixo", json!({ "tipo": "texto_de_cabeca_para_baixo", "entidade": t.id, "giro": arred(g), "posicao": par(t.cantos[0]) }), &mut lista);
        }
        let dentro: Vec<_> = folhas.iter().filter(|f| t.min.x < f.max.x && t.max.x > f.min.x && t.min.y < f.max.y && t.max.y > f.min.y).collect();
        let inteiro = |f: &&feathercad_core::folha::InfoFolha| t.min.x >= f.min.x && t.max.x <= f.max.x && t.min.y >= f.min.y && t.max.y <= f.max.y;
        match dentro.len() {
            0 => {}
            1 if !inteiro(&dentro[0]) => anotar("texto_fora_da_folha", json!({ "tipo": "texto_fora_da_folha", "entidade": t.id, "folha": dentro[0].nome, "posicao": par(t.cantos[0]) }), &mut lista),
            1 => {}
            _ => anotar("texto_entre_folhas", json!({ "tipo": "texto_entre_folhas", "entidade": t.id, "folhas": dentro.iter().map(|f| f.nome.clone()).collect::<Vec<_>>(), "posicao": par(t.cantos[0]) }), &mut lista),
        }
    }

    let encolhidas: Vec<[Ponto; 4]> = textos.iter().map(|t| encolher(&t.cantos)).collect();
    for i in 0..textos.len() {
        for j in i + 1..textos.len() {
            let (a, b) = (&textos[i], &textos[j]);
            if a.max.x <= b.min.x || b.max.x <= a.min.x || a.max.y <= b.min.y || b.max.y <= a.min.y {
                continue;
            }
            if se_cruzam(&encolhidas[i], &encolhidas[j]) {
                anotar("texto_sobre_texto", json!({ "tipo": "texto_sobre_texto", "entidades": [a.id, b.id], "posicao": par(a.cantos[0]) }), &mut lista);
            }
        }
    }

    // Texto sobre traço: só linhas e trechos retos de polilinha (arcos e blocos ficam de fora).
    for e in d.entidades() {
        let mut segs: Vec<(Ponto, Ponto)> = Vec::new();
        match &e.geometria {
            Geometria::Linha { a, b } => segs.push((*a, *b)),
            Geometria::Polilinha { vertices, fechada } => {
                let n = vertices.len();
                for k in 0..if *fechada { n } else { n.saturating_sub(1) } {
                    if vertices[k].bulge == 0.0 {
                        segs.push((vertices[k].p, vertices[(k + 1) % n].p));
                    }
                }
            }
            _ => continue,
        }
        for (a, b) in segs {
            let (lx0, lx1, ly0, ly1) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
            for (t, r) in textos.iter().zip(&encolhidas) {
                if t.max.x < lx0 || t.min.x > lx1 || t.max.y < ly0 || t.min.y > ly1 {
                    continue;
                }
                if segmento_toca(a, b, r) {
                    anotar("texto_sobre_linha", json!({ "tipo": "texto_sobre_linha", "entidade": t.id, "linha": e.id, "posicao": par(t.cantos[0]) }), &mut lista);
                }
            }
        }
    }

    json!({
        "ok": lista.is_empty(), "textos_conferidos": textos.len(), "folhas": folhas.len(),
        "contagem": contagem, "problemas": lista,
        "nota": "texto = retângulo pela métrica do PDF; sobre linha só conta traço reto cruzando o miolo do texto; peça a imagem só onde houver problema",
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn texto(d: &mut Documento, x: f64, y: f64, giro: f64, c: &str) {
        d.adicionar_geometria(Geometria::Texto { ponto: Ponto::novo(x, y), altura: 2.0, rotacao: giro, conteudo: c.into(), h: AlinhaH::Esquerda, v: AlinhaV::Base });
    }

    #[test]
    fn acha_sobreposicao_linha_cruzando_e_cabeca_para_baixo() {
        let mut d = Documento::default();
        texto(&mut d, 0.0, 0.0, 0.0, "Sala de estar");
        texto(&mut d, 3.0, 0.5, 0.0, "Cozinha");
        texto(&mut d, 0.0, 20.0, 0.0, "Limpo");
        texto(&mut d, 0.0, 40.0, 180.0, "Virado");
        d.adicionar_linha(Ponto::novo(-5.0, 21.0), Ponto::novo(30.0, 21.0));
        let r = verificar_leiaute(&d);
        let c = &r["contagem"];
        assert_eq!(c["texto_sobre_texto"], 1, "{r}");
        assert_eq!(c["texto_sobre_linha"], 1, "{r}");
        assert_eq!(c["texto_de_cabeca_para_baixo"], 1, "{r}");
    }

    #[test]
    fn texto_afastado_nao_acusa() {
        let mut d = Documento::default();
        texto(&mut d, 0.0, 0.0, 0.0, "A");
        texto(&mut d, 0.0, 5.0, 0.0, "B");
        assert_eq!(verificar_leiaute(&d)["ok"], true);
    }
}
