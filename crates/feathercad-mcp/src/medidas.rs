//! O que uma entidade "é" em números: o tipo, a forma e as medidas (comprimento, área, centro, caixa…).
//! É o que dá à IA a noção de tamanho e posição sem precisar adivinhar pela imagem.

use feathercad_core::documento::{Documento, Entidade, Geometria};
use feathercad_core::edicao::{caixa_da_entidade, pontos_da_polilinha};
use feathercad_core::geom::{varredura_graus, Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{arred, par};

/// O comprimento e a área (com sinal: anti-horário positivo) de uma polilinha, arcos por bulge inclusive.
pub fn medidas_da_polilinha(vertices: &[Vertice], fechada: bool) -> (f64, f64) {
    let n = vertices.len();
    let trechos = if fechada { n } else { n.saturating_sub(1) };
    let (mut comprimento, mut area) = (0.0, 0.0);
    for i in 0..trechos {
        let (a, b) = (vertices[i], vertices[(i + 1) % n]);
        let corda = a.p.distancia(b.p);
        area += (a.p.x * b.p.y - b.p.x * a.p.y) / 2.0;
        if a.bulge.abs() < 1e-12 || corda < 1e-12 {
            comprimento += corda;
        } else {
            let theta = 4.0 * a.bulge.atan();
            let raio = corda / (2.0 * (theta / 2.0).sin().abs());
            comprimento += raio * theta.abs();
            area += raio * raio / 2.0 * (theta - theta.sin());
        }
    }
    (comprimento, if fechada { area } else { 0.0 })
}

/// O centro de massa da região do contorno (pelo polígono de pontos).
pub fn centroide(pts: &[Ponto]) -> Ponto {
    let n = pts.len();
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let c = p.x * q.y - q.x * p.y;
        a2 += c;
        cx += (p.x + q.x) * c;
        cy += (p.y + q.y) * c;
    }
    if a2.abs() < 1e-18 {
        let m = n.max(1) as f64;
        return Ponto::novo(pts.iter().map(|p| p.x).sum::<f64>() / m, pts.iter().map(|p| p.y).sum::<f64>() / m);
    }
    Ponto::novo(cx / (3.0 * a2), cy / (3.0 * a2))
}

/// Quatro vértices retos, cantos de 90°: largura e altura do retângulo (em qualquer giro).
fn retangulo(vertices: &[Vertice], fechada: bool) -> Option<(f64, f64)> {
    if !fechada || vertices.len() != 4 || vertices.iter().any(|v| v.bulge != 0.0) {
        return None;
    }
    let p: Vec<Ponto> = vertices.iter().map(|v| v.p).collect();
    let (l1, l2) = (p[0].distancia(p[1]), p[1].distancia(p[2]));
    let reto = |a: Ponto, b: Ponto, c: Ponto| ((b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y)).abs() < 1e-9 * l1.max(l2).powi(2);
    (reto(p[0], p[1], p[2]) && reto(p[1], p[2], p[3]) && reto(p[2], p[3], p[0]) && (p[0].distancia(p[3]) - l2).abs() < 1e-9 * l2.max(1.0)).then_some((l1, l2))
}

/// O nome do tipo, o mesmo que as ferramentas de criação e o filtro usam.
pub fn tipo_de(g: &Geometria) -> &'static str {
    match g {
        Geometria::Linha { .. } => "linha",
        Geometria::Polilinha { .. } => "polilinha",
        Geometria::Circulo { .. } => "circulo",
        Geometria::Arco { .. } => "arco",
        Geometria::Texto { .. } => "texto",
        Geometria::Ponto { .. } => "ponto",
        Geometria::Hachura { .. } => "hachura",
        Geometria::Insercao { .. } => "bloco",
        Geometria::Face3D { .. } => "face3d",
        Geometria::Imagem { .. } => "imagem",
    }
}

fn hex(cor: u32) -> String {
    format!("#{cor:06X}")
}

/// A forma e as medidas específicas de cada tipo.
fn detalhes(doc: &Documento, g: &Geometria) -> Value {
    match g {
        Geometria::Linha { a, b } => json!({ "de": par(*a), "ate": par(*b), "comprimento": arred(a.distancia(*b)), "angulo": arred(a.angulo_ate(*b)), "ponto_medio": par(Ponto::novo((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)) }),
        Geometria::Polilinha { vertices, fechada } => {
            let (comprimento, area) = medidas_da_polilinha(vertices, *fechada);
            let pts = pontos_da_polilinha(vertices, *fechada);
            let mut v = json!({ "vertices": vertices.iter().map(|v| if v.bulge == 0.0 { json!(par(v.p)) } else { json!([arred(v.p.x), arred(v.p.y), { "arco_bulge": arred(v.bulge) }]) }).collect::<Vec<_>>(), "fechada": fechada, "comprimento": arred(comprimento) });
            if *fechada {
                v["area"] = json!(arred(area.abs()));
                v["centro"] = json!(par(centroide(&pts)));
                v["sentido"] = json!(if area >= 0.0 { "anti-horario" } else { "horario" });
            }
            if let Some((l, a)) = retangulo(vertices, *fechada) {
                v["retangulo"] = json!({ "largura": arred(l), "altura": arred(a) });
            }
            v
        }
        Geometria::Circulo { centro, raio } => json!({ "centro": par(*centro), "raio": arred(*raio), "diametro": arred(raio * 2.0), "circunferencia": arred(std::f64::consts::TAU * raio), "area": arred(std::f64::consts::PI * raio * raio) }),
        Geometria::Arco { centro, raio, inicio, fim } => {
            let v = varredura_graus(*inicio, *fim);
            let em = |g: f64| Ponto::novo(centro.x + raio * g.to_radians().cos(), centro.y + raio * g.to_radians().sin());
            json!({ "centro": par(*centro), "raio": arred(*raio), "angulo_inicial": arred(*inicio), "angulo_final": arred(*fim), "varredura": arred(v), "comprimento": arred(raio * v.to_radians()), "ponto_inicial": par(em(*inicio)), "ponto_final": par(em(*fim)), "ponto_medio": par(em(inicio + v / 2.0)) })
        }
        Geometria::Texto { ponto, altura, rotacao, conteudo, .. } => json!({ "ponto": par(*ponto), "altura": arred(*altura), "rotacao": arred(*rotacao), "conteudo": conteudo }),
        Geometria::Ponto { p } => json!({ "ponto": par(*p) }),
        Geometria::Hachura { contornos, nome, padrao } => json!({ "padrao": nome, "solida": padrao.is_empty(), "contornos": contornos.len() }),
        Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => json!({ "bloco": doc.blocos().get(*bloco).map(|b| b.nome.clone()), "ponto": par(*ponto), "escala": [arred(*escala_x), arred(*escala_y)], "rotacao": arred(*rotacao) }),
        Geometria::Face3D { p1, p2, p3, p4 } => json!({ "p1": [arred(p1.x), arred(p1.y), arred(p1.z)], "p2": [arred(p2.x), arred(p2.y), arred(p2.z)], "p3": [arred(p3.x), arred(p3.y), arred(p3.z)], "p4": [arred(p4.x), arred(p4.y), arred(p4.z)] }),
        Geometria::Imagem { origem, u, v, arquivo } => json!({ "origem": par(*origem), "largura": arred(u.x.hypot(u.y)), "altura": arred(v.x.hypot(v.y)), "arquivo": arquivo }),
    }
}

/// A entidade inteira como JSON: id, tipo, camada, cor, caixa e os detalhes do tipo.
pub fn descrever(doc: &Documento, e: &Entidade) -> Value {
    let camada = doc.camadas().get(e.camada).map_or("?", |c| c.nome.as_str());
    let mut v = json!({ "id": e.id, "tipo": tipo_de(&e.geometria), "camada": camada, "cor": hex(doc.cor_resolvida(e)) });
    if let Some((min, max)) = caixa_da_entidade(doc, &e.geometria) {
        v["caixa"] = json!({ "min": par(min), "max": par(max), "largura": arred(max.x - min.x), "altura": arred(max.y - min.y) });
    }
    if let Value::Object(d) = detalhes(doc, &e.geometria) {
        if let Some(o) = v.as_object_mut() {
            o.extend(d);
        }
    }
    v
}

/// Um ponto "em cima" da entidade, onde faz sentido escrever o id na imagem.
pub fn ponto_de_rotulo(g: &Geometria) -> Option<Ponto> {
    match g {
        Geometria::Linha { a, b } => Some(Ponto::novo((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)),
        Geometria::Polilinha { vertices, .. } => {
            let pts: Vec<Ponto> = vertices.iter().map(|v| v.p).collect();
            (pts.len() >= 2).then(|| Ponto::novo((pts[0].x + pts[1].x) / 2.0, (pts[0].y + pts[1].y) / 2.0))
        }
        Geometria::Circulo { centro, raio } => Some(Ponto::novo(centro.x + raio * std::f64::consts::FRAC_1_SQRT_2, centro.y + raio * std::f64::consts::FRAC_1_SQRT_2)),
        Geometria::Arco { centro, raio, inicio, fim } => {
            let m = (inicio + varredura_graus(*inicio, *fim) / 2.0).to_radians();
            Some(Ponto::novo(centro.x + raio * m.cos(), centro.y + raio * m.sin()))
        }
        Geometria::Texto { ponto, .. } | Geometria::Insercao { ponto, .. } | Geometria::Ponto { p: ponto } => Some(*ponto),
        Geometria::Hachura { contornos, .. } => contornos.first().and_then(|c| c.first()).map(|v| v.p),
        Geometria::Face3D { p1, p2, p3, p4 } => Some(Ponto::novo((p1.x + p2.x + p3.x + p4.x) / 4.0, (p1.y + p2.y + p3.y + p4.y) / 4.0)),
        Geometria::Imagem { origem, .. } => Some(*origem),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn v(x: f64, y: f64) -> Vertice {
        Vertice { p: Ponto::novo(x, y), bulge: 0.0 }
    }

    #[test]
    fn retangulo_de_polilinha_tem_area_e_largura_e_altura() {
        let vs = [v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0), v(0.0, 3.0)];
        let (c, a) = medidas_da_polilinha(&vs, true);
        assert!((c - 14.0).abs() < 1e-9 && (a - 12.0).abs() < 1e-9);
        assert_eq!(retangulo(&vs, true), Some((4.0, 3.0)));
        assert!(retangulo(&[v(0.0, 0.0), v(4.0, 0.0), v(5.0, 3.0), v(0.0, 3.0)], true).is_none(), "trapézio não é retângulo");
    }

    #[test]
    fn meio_circulo_por_bulge_tem_a_area_e_o_comprimento_certos() {
        // Dois vértices, bulge 1 = meia volta: o círculo de diâmetro 2 (raio 1) fechado.
        let vs = [Vertice { p: Ponto::novo(-1.0, 0.0), bulge: 1.0 }, Vertice { p: Ponto::novo(1.0, 0.0), bulge: 1.0 }];
        let (c, a) = medidas_da_polilinha(&vs, true);
        assert!((c - std::f64::consts::TAU).abs() < 1e-9, "{c}");
        assert!((a - std::f64::consts::PI).abs() < 1e-9, "{a}");
    }

    #[test]
    fn centroide_do_quadrado_e_o_centro() {
        let c = centroide(&[Ponto::novo(0.0, 0.0), Ponto::novo(2.0, 0.0), Ponto::novo(2.0, 2.0), Ponto::novo(0.0, 2.0)]);
        assert!((c.x - 1.0).abs() < 1e-9 && (c.y - 1.0).abs() < 1e-9);
    }
}
