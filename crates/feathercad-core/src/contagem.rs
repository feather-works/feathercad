//! Contar objetos iguais: dado um objeto, quais outros têm o mesmo tipo e a mesma medida (dormentes, tomadas, círculos…).
//! O comando `contar` seleciona os iguais (o realce azul mostra quais são) e diz quantos são e em que camadas estão.

use std::collections::BTreeMap;

use crate::documento::{Documento, Geometria, Id};
use crate::geom::Vertice;

/// Diferença relativa aceita entre duas medidas para valerem como iguais.
const TOLERANCIA: f64 = 0.01;

/// O que faz um objeto igual a outro: o tipo, um texto (conteúdo/nome do bloco) e até três medidas.
#[derive(Clone, Debug, PartialEq)]
struct Assinatura {
    tipo: u8,
    texto: String,
    medidas: [f64; 3],
}

fn parecido(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCIA * a.abs().max(b.abs()).max(1e-9)
}

impl Assinatura {
    fn igual(&self, o: &Assinatura) -> bool {
        self.tipo == o.tipo && self.texto == o.texto && self.medidas.iter().zip(o.medidas.iter()).all(|(a, b)| parecido(*a, *b))
    }
}

fn distancia(a: &Vertice, b: &Vertice) -> f64 {
    a.p.distancia(b.p)
}

/// Retângulo (4 vértices retos, ângulos de 90°): os dois lados, do menor para o maior.
fn como_retangulo(v: &[Vertice], fechada: bool) -> Option<(f64, f64)> {
    if !fechada || v.len() != 4 || v.iter().any(|x| x.bulge != 0.0) {
        return None;
    }
    let (a, b, c) = (distancia(&v[0], &v[1]), distancia(&v[1], &v[2]), distancia(&v[2], &v[3]));
    let d = distancia(&v[3], &v[0]);
    let reto = {
        let (ux, uy) = (v[1].p.x - v[0].p.x, v[1].p.y - v[0].p.y);
        let (wx, wy) = (v[2].p.x - v[1].p.x, v[2].p.y - v[1].p.y);
        (ux * wx + uy * wy).abs() <= 1e-6 * (a * b).max(1e-12)
    };
    (reto && parecido(a, c) && parecido(b, d)).then(|| (a.min(b), a.max(b)))
}

fn assinatura(g: &Geometria) -> Assinatura {
    let sem_texto = String::new();
    match g {
        Geometria::Linha { a, b } => Assinatura { tipo: 1, texto: sem_texto, medidas: [a.distancia(*b), 0.0, 0.0] },
        Geometria::Polilinha { vertices, fechada } => match como_retangulo(vertices, *fechada) {
            Some((menor, maior)) => Assinatura { tipo: 2, texto: sem_texto, medidas: [menor, maior, 0.0] },
            None => {
                let n = vertices.len();
                let trechos = if *fechada { n } else { n.saturating_sub(1) };
                let comprimento: f64 = (0..trechos).map(|i| distancia(&vertices[i], &vertices[(i + 1) % n])).sum();
                Assinatura { tipo: if *fechada { 3 } else { 4 }, texto: sem_texto, medidas: [comprimento, n as f64, vertices.iter().map(|v| v.bulge.abs()).sum()] }
            }
        },
        Geometria::Circulo { raio, .. } => Assinatura { tipo: 5, texto: sem_texto, medidas: [*raio, 0.0, 0.0] },
        Geometria::Arco { raio, inicio, fim, .. } => Assinatura { tipo: 6, texto: sem_texto, medidas: [*raio, (fim - inicio).rem_euclid(360.0), 0.0] },
        Geometria::Texto { conteudo, altura, .. } => Assinatura { tipo: 7, texto: conteudo.clone(), medidas: [*altura, 0.0, 0.0] },
        Geometria::Ponto { .. } => Assinatura { tipo: 8, texto: sem_texto, medidas: [0.0; 3] },
        Geometria::Hachura { nome, contornos, .. } => Assinatura { tipo: 9, texto: nome.clone(), medidas: [contornos.len() as f64, 0.0, 0.0] },
        Geometria::Insercao { bloco, escala_x, escala_y, .. } => Assinatura { tipo: 10, texto: bloco.to_string(), medidas: [escala_x.abs(), escala_y.abs(), 0.0] },
        Geometria::Face3D { p1, p2, p3, p4 } => Assinatura { tipo: 11, texto: sem_texto, medidas: [p1.distancia(*p2), p2.distancia(*p3), p3.distancia(*p4)] },
    }
}

fn parecido_com(a: f64, b: f64, tolerancia: f64) -> bool {
    (a - b).abs() <= tolerancia * a.abs().max(b.abs()).max(1e-9)
}

/// Os objetos (visíveis, livres) de um tipo e uma medida: `retangulo` (`a` × `b`, em qualquer giro), `circulo` (raio `a`)
/// ou `linha` (comprimento `a`), com folga relativa `tolerancia` e, se `camada` vem, só nela.
pub fn por_medida(doc: &Documento, tipo: &str, a: f64, b: f64, tolerancia: f64, camada: Option<&str>) -> Result<Contagem, String> {
    let (menor, maior) = (a.min(b), a.max(b));
    let mut ids = Vec::new();
    let mut por_camada: BTreeMap<String, usize> = BTreeMap::new();
    for e in doc.entidades().iter().filter(|e| doc.visivel(e) && !doc.bloqueada(e)) {
        let nome = doc.camadas().get(e.camada).map_or_else(|| "?".to_string(), |c| c.nome.clone());
        if camada.is_some_and(|c| !c.eq_ignore_ascii_case(&nome)) {
            continue;
        }
        let bate = match (tipo, &e.geometria) {
            ("retangulo", Geometria::Polilinha { vertices, fechada }) => como_retangulo(vertices, *fechada).is_some_and(|(m, g)| parecido_com(m, menor, tolerancia) && parecido_com(g, maior, tolerancia)),
            ("circulo", Geometria::Circulo { raio, .. }) => parecido_com(*raio, a, tolerancia),
            ("linha", Geometria::Linha { a: p, b: q }) => parecido_com(p.distancia(*q), a, tolerancia),
            ("retangulo" | "circulo" | "linha", _) => false,
            _ => return Err(format!("tipo `{tipo}` não tem contagem por medida (use retangulo, circulo ou linha)")),
        };
        if bate {
            ids.push(e.id);
            *por_camada.entry(nome).or_default() += 1;
        }
    }
    let mut por_camada: Vec<(String, usize)> = por_camada.into_iter().collect();
    por_camada.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    Ok(Contagem { ids, por_camada })
}

/// O resultado de uma contagem: os ids iguais ao objeto de referência (ele incluído) e quantos há em cada camada.
pub struct Contagem {
    pub ids: Vec<Id>,
    pub por_camada: Vec<(String, usize)>,
}

/// Os objetos (visíveis, livres) iguais a `id`: mesmo tipo e mesma medida, com 1 % de folga. `None` se o objeto não existe.
pub fn iguais_a(doc: &Documento, id: Id) -> Option<Contagem> {
    let modelo = assinatura(&doc.entidade(id)?.geometria);
    let mut ids = Vec::new();
    let mut por_camada: BTreeMap<String, usize> = BTreeMap::new();
    for e in doc.entidades().iter().filter(|e| doc.visivel(e) && !doc.bloqueada(e)) {
        if assinatura(&e.geometria).igual(&modelo) {
            ids.push(e.id);
            let nome = doc.camadas().get(e.camada).map_or_else(|| "?".to_string(), |c| c.nome.clone());
            *por_camada.entry(nome).or_default() += 1;
        }
    }
    let mut por_camada: Vec<(String, usize)> = por_camada.into_iter().collect();
    por_camada.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Some(Contagem { ids, por_camada })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::documento::Documento;
    use crate::geom::Ponto;

    fn retangulo(d: &mut Documento, x: f64, y: f64, l: f64, a: f64) -> Id {
        let v = |px: f64, py: f64| Vertice { p: Ponto::novo(px, py), bulge: 0.0 };
        d.adicionar_geometria(Geometria::Polilinha { vertices: vec![v(x, y), v(x + l, y), v(x + l, y + a), v(x, y + a)], fechada: true })
    }

    #[test]
    fn conta_os_retangulos_de_mesma_medida_mesmo_girados_e_deixa_os_diferentes() {
        let mut d = Documento::default();
        let modelo = retangulo(&mut d, 0.0, 0.0, 0.24, 2.0);
        for i in 1..10 {
            retangulo(&mut d, f64::from(i), 0.0, 0.24, 2.0);
        }
        retangulo(&mut d, 0.0, 5.0, 2.0, 0.24); // deitado: mesmo retângulo
        retangulo(&mut d, 0.0, 9.0, 0.5, 2.0); // outra medida
        d.adicionar_circulo(Ponto::novo(0.0, 0.0), 1.0);
        let c = iguais_a(&d, modelo).unwrap();
        assert_eq!(c.ids.len(), 11, "10 em pé + 1 deitado");
        assert_eq!(c.por_camada, vec![("0".to_string(), 11)]);
    }

    #[test]
    fn circulo_texto_e_linha_comparam_o_que_importa() {
        let mut d = Documento::default();
        let c1 = d.adicionar_circulo(Ponto::novo(0.0, 0.0), 5.0).unwrap();
        d.adicionar_circulo(Ponto::novo(9.0, 9.0), 5.04);
        d.adicionar_circulo(Ponto::novo(1.0, 1.0), 6.0);
        assert_eq!(iguais_a(&d, c1).unwrap().ids.len(), 2);
        let l = d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(3.0, 4.0));
        d.adicionar_linha(Ponto::novo(10.0, 0.0), Ponto::novo(10.0, 5.0));
        d.adicionar_linha(Ponto::novo(10.0, 0.0), Ponto::novo(10.0, 6.0));
        assert_eq!(iguais_a(&d, l).unwrap().ids.len(), 2, "5 e 5, não o 6");
        assert!(iguais_a(&d, 999_999).is_none());
    }
}
