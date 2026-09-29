//! Edição de entidades: só geometria, sem sessão. Transformar (mover, girar, escalar,
//! espelhar), esticar, descobrir o que o clique ou a janela pegou, matriz, e as formas
//! prontas (retângulo, elipse). Aparar, concordância e hachura ficam em `aparar.rs`.

use crate::documento::{Documento, Entidade, Geometria, Id, LinhaDePadrao, PROFUNDIDADE_MAXIMA};
use crate::geom::{amostrar_arco, normalizar_graus, segmento_com_bulge, varredura_graus, Ponto, Transf, Vertice};

/// Aplica uma transformação de semelhança (deslocar, girar, escalar por igual, espelhar) a uma forma.
/// Espelhar mantém o texto legível (só a posição reflete), como o `MIRRTEXT = 0` do AutoCAD.
pub fn transformar(geo: &Geometria, t: &Transf) -> Geometria {
    let k = t.escala_media();
    let refletida = t.determinante() < 0.0;
    let ang = t.rotacao_graus();
    let vertice = |v: &Vertice| Vertice { p: t.aplicar(v.p), bulge: if refletida { -v.bulge } else { v.bulge } };
    let giro = |r: f64| normalizar_graus(if refletida { ang - r } else { r + ang });
    match geo {
        Geometria::Linha { a, b } => Geometria::Linha { a: t.aplicar(*a), b: t.aplicar(*b) },
        Geometria::Polilinha { vertices, fechada } => Geometria::Polilinha { vertices: vertices.iter().map(vertice).collect(), fechada: *fechada },
        Geometria::Circulo { centro, raio } => Geometria::Circulo { centro: t.aplicar(*centro), raio: raio * k },
        Geometria::Arco { centro, raio, inicio, fim } => {
            let (inicio, fim) = if refletida { (giro(*fim), giro(*inicio)) } else { (giro(*inicio), giro(*fim)) };
            Geometria::Arco { centro: t.aplicar(*centro), raio: raio * k, inicio, fim }
        }
        Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } => Geometria::Texto {
            ponto: t.aplicar(*ponto),
            altura: altura * k,
            rotacao: if refletida { *rotacao } else { giro(*rotacao) },
            conteudo: conteudo.clone(),
            h: *h,
            v: *v,
        },
        Geometria::Ponto { p } => Geometria::Ponto { p: t.aplicar(*p) },
        Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => Geometria::Insercao {
            bloco: *bloco,
            ponto: t.aplicar(*ponto),
            escala_x: escala_x * k,
            escala_y: escala_y * k * if refletida { -1.0 } else { 1.0 },
            rotacao: giro(*rotacao),
        },
        Geometria::Hachura { contornos, padrao, nome } => Geometria::Hachura {
            contornos: contornos.iter().map(|c| c.iter().map(vertice).collect()).collect(),
            padrao: padrao
                .iter()
                .map(|l| LinhaDePadrao {
                    angulo: giro(l.angulo),
                    base: t.aplicar(l.base),
                    dx: t.a * l.dx + t.c * l.dy,
                    dy: t.b * l.dx + t.d * l.dy,
                })
                .collect(),
            nome: nome.clone(),
        },
    }
}

fn dentro(p: Ponto, (min, max): (Ponto, Ponto)) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}

/// Esticar: os pontos que caem dentro de `janela` vão por `(dx, dy)`; os de fora ficam. Círculo,
/// arco, texto, ponto e inserção andam inteiros se o ponto de referência (centro, posição) está na janela.
/// `None` se a janela não pegou nada da forma.
pub fn esticar(geo: &Geometria, janela: (Ponto, Ponto), dx: f64, dy: f64) -> Option<Geometria> {
    let mover = |p: Ponto| if dentro(p, janela) { Ponto::novo(p.x + dx, p.y + dy) } else { p };
    let inteira = |ref_: Ponto| dentro(ref_, janela).then(|| transformar(geo, &Transf::mover(dx, dy)));
    match geo {
        Geometria::Linha { a, b } => (dentro(*a, janela) || dentro(*b, janela)).then(|| Geometria::Linha { a: mover(*a), b: mover(*b) }),
        Geometria::Polilinha { vertices, fechada } => vertices
            .iter()
            .any(|v| dentro(v.p, janela))
            .then(|| Geometria::Polilinha { vertices: vertices.iter().map(|v| Vertice { p: mover(v.p), bulge: v.bulge }).collect(), fechada: *fechada }),
        Geometria::Hachura { contornos, padrao, nome } => contornos
            .iter()
            .flatten()
            .any(|v| dentro(v.p, janela))
            .then(|| Geometria::Hachura { contornos: contornos.iter().map(|c| c.iter().map(|v| Vertice { p: mover(v.p), bulge: v.bulge }).collect()).collect(), padrao: padrao.clone(), nome: nome.clone() }),
        Geometria::Circulo { centro, .. } | Geometria::Arco { centro, .. } => inteira(*centro),
        Geometria::Texto { ponto, .. } | Geometria::Insercao { ponto, .. } | Geometria::Ponto { p: ponto } => inteira(*ponto),
    }
}

fn distancia_ao_segmento(p: Ponto, a: Ponto, b: Ponto) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let comprimento2 = dx * dx + dy * dy;
    if comprimento2 < 1e-18 {
        return p.distancia(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / comprimento2).clamp(0.0, 1.0);
    p.distancia(Ponto::novo(a.x + t * dx, a.y + t * dy))
}

/// A polilinha como pontos (arcos por bulge viram cordas finas, relativas ao tamanho de cada trecho).
pub fn pontos_da_polilinha(vertices: &[Vertice], fechada: bool) -> Vec<Ponto> {
    let n = vertices.len();
    let trechos = if fechada { n } else { n.saturating_sub(1) };
    let mut pts = Vec::new();
    for i in 0..trechos {
        let (de, ate) = (&vertices[i], &vertices[(i + 1) % n]);
        let tolerancia = (de.p.distancia(ate.p) / 200.0).max(1e-12);
        let trecho = segmento_com_bulge(de.p, ate.p, de.bulge, tolerancia);
        pts.extend(&trecho[..trecho.len().saturating_sub(1)]);
    }
    if !fechada {
        if let Some(u) = vertices.last() {
            pts.push(u.p);
        }
    }
    pts
}

fn dentro_do_laco(p: Ponto, laco: &[Ponto]) -> bool {
    let mut dentro = false;
    for i in 0..laco.len() {
        let (a, b) = (laco[i], laco[(i + 1) % laco.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            dentro = !dentro;
        }
    }
    dentro
}

/// A distância de `p` à entidade (0 = em cima). Inserção: o mais perto entre as formas do bloco.
pub fn distancia(doc: &Documento, geo: &Geometria, p: Ponto) -> f64 {
    distancia_no_nivel(doc, geo, p, 0)
}

fn distancia_no_nivel(doc: &Documento, geo: &Geometria, p: Ponto, nivel: u8) -> f64 {
    match geo {
        Geometria::Linha { a, b } => distancia_ao_segmento(p, *a, *b),
        Geometria::Circulo { centro, raio } => (centro.distancia(p) - raio).abs(),
        Geometria::Arco { centro, raio, inicio, fim } => {
            let varredura = varredura_graus(*inicio, *fim);
            if varredura_graus(*inicio, centro.angulo_ate(p)) <= varredura {
                (centro.distancia(p) - raio).abs()
            } else {
                let ponta = |ang: f64| Ponto::novo(centro.x + raio * ang.to_radians().cos(), centro.y + raio * ang.to_radians().sin());
                p.distancia(ponta(*inicio)).min(p.distancia(ponta(*fim)))
            }
        }
        Geometria::Polilinha { vertices, fechada } => {
            let pts = pontos_da_polilinha(vertices, *fechada);
            let mut m = f64::MAX;
            for w in pts.windows(2) {
                m = m.min(distancia_ao_segmento(p, w[0], w[1]));
            }
            if *fechada && pts.len() > 2 {
                m = m.min(distancia_ao_segmento(p, pts[pts.len() - 1], pts[0]));
            }
            m
        }
        Geometria::Texto { ponto, altura, rotacao, conteudo, .. } => {
            let largura = altura * crate::documento::LARGURA_DO_CARACTERE * conteudo.chars().count() as f64;
            let r = rotacao.to_radians();
            let fim = Ponto::novo(ponto.x + largura * r.cos(), ponto.y + largura * r.sin());
            (distancia_ao_segmento(p, *ponto, fim) - altura / 2.0).max(0.0)
        }
        Geometria::Ponto { p: q } => p.distancia(*q),
        Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } => {
            let Some(def) = doc.blocos().get(*bloco).filter(|_| nivel < PROFUNDIDADE_MAXIMA) else { return f64::MAX };
            let t = Transf::de_insercao(def.base, *ponto, *escala_x, *escala_y, *rotacao);
            let Some(inv) = t.inversa() else { return f64::MAX };
            let dentro = inv.aplicar(p);
            let k = t.escala_media();
            def.entidades.iter().map(|n| distancia_no_nivel(doc, &n.geometria, dentro, nivel + 1) * k).fold(f64::MAX, f64::min)
        }
        Geometria::Hachura { contornos, .. } => {
            let lacos: Vec<Vec<Ponto>> = contornos.iter().map(|c| pontos_da_polilinha(c, true)).collect();
            // Sólida ou de padrão, clicar dentro da área conta como estar em cima.
            let cheia = lacos.iter().filter(|l| dentro_do_laco(p, l)).count() % 2 == 1;
            if cheia {
                return 0.0;
            }
            lacos.iter().flat_map(|l| (0..l.len()).map(move |i| (l[i], l[(i + 1) % l.len()]))).map(|(a, b)| distancia_ao_segmento(p, a, b)).fold(f64::MAX, f64::min)
        }
    }
}

/// A caixa (mínimo, máximo) de uma entidade, descendo nos blocos.
pub fn caixa_da_entidade(doc: &Documento, geo: &Geometria) -> Option<(Ponto, Ponto)> {
    let mut caixa: Option<(Ponto, Ponto)> = None;
    let mut juntar = |p: Ponto| {
        caixa = Some(match caixa {
            None => (p, p),
            Some((min, max)) => (Ponto::novo(min.x.min(p.x), min.y.min(p.y)), Ponto::novo(max.x.max(p.x), max.y.max(p.y))),
        });
    };
    if let Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } = geo {
        let (Some((min, max)), Some(def)) = (doc.caixa_guardada_do_bloco(*bloco), doc.blocos().get(*bloco)) else { return None };
        let t = Transf::de_insercao(def.base, *ponto, *escala_x, *escala_y, *rotacao);
        for c in [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)] {
            juntar(t.aplicar(c));
        }
    } else if let Geometria::Polilinha { vertices, fechada } = geo {
        pontos_da_polilinha(vertices, *fechada).into_iter().for_each(juntar);
    } else {
        geo.para_cada_ponto_de_caixa(&mut juntar);
    }
    caixa
}

/// O segmento `a–b` cruza o retângulo `janela`?
fn segmento_cruza(a: Ponto, b: Ponto, (min, max): (Ponto, Ponto)) -> bool {
    if dentro(a, (min, max)) || dentro(b, (min, max)) {
        return true;
    }
    let cantos = [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)];
    let cruz = |o: Ponto, p: Ponto, q: Ponto| (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x);
    (0..4).any(|i| {
        let (c, d) = (cantos[i], cantos[(i + 1) % 4]);
        cruz(a, b, c) * cruz(a, b, d) <= 0.0 && cruz(c, d, a) * cruz(c, d, b) <= 0.0
    })
}

/// A janela pegou a entidade? `cruzando`: basta tocar; senão ela tem que caber inteira.
pub fn na_janela(doc: &Documento, geo: &Geometria, janela: (Ponto, Ponto), cruzando: bool) -> bool {
    let Some((min, max)) = caixa_da_entidade(doc, geo) else { return false };
    let inteira = min.x >= janela.0.x && max.x <= janela.1.x && min.y >= janela.0.y && max.y <= janela.1.y;
    if !cruzando {
        return inteira;
    }
    if inteira {
        return true;
    }
    match geo {
        Geometria::Linha { a, b } => segmento_cruza(*a, *b, janela),
        Geometria::Polilinha { vertices, fechada } => {
            let pts = pontos_da_polilinha(vertices, *fechada);
            pts.windows(2).any(|w| segmento_cruza(w[0], w[1], janela)) || (*fechada && pts.len() > 2 && segmento_cruza(pts[pts.len() - 1], pts[0], janela))
        }
        // Círculo e arco: só vale se o **traço** toca a janela (uma janela dentro do círculo, sem tocar a linha, não pega).
        Geometria::Circulo { centro, raio } => {
            let pts = amostrar_arco(*centro, *raio, 0.0, std::f64::consts::TAU, raio / 500.0);
            pts.windows(2).any(|w| segmento_cruza(w[0], w[1], janela))
        }
        Geometria::Arco { centro, raio, inicio, fim } => {
            let pts = amostrar_arco(*centro, *raio, inicio.to_radians(), varredura_graus(*inicio, *fim).to_radians(), raio / 500.0);
            pts.windows(2).any(|w| segmento_cruza(w[0], w[1], janela))
        }
        _ => min.x <= janela.1.x && max.x >= janela.0.x && min.y <= janela.1.y && max.y >= janela.0.y,
    }
}

/// A entidade visível e livre (camada não bloqueada) mais perto de `p`, a até `tolerancia` (a de cima ganha o empate).
pub fn escolher(doc: &Documento, p: Ponto, tolerancia: f64) -> Option<Id> {
    escolher_com(doc, p, tolerancia, false)
}

/// Como `escolher`; com `incluir_bloqueadas` também pega objetos de camada bloqueada (para só *olhar* a camada ou as propriedades).
pub fn escolher_com(doc: &Documento, p: Ponto, tolerancia: f64, incluir_bloqueadas: bool) -> Option<Id> {
    let mut melhor: Option<(f64, Id)> = None;
    for e in doc.entidades().iter().filter(|e| doc.visivel(e) && (incluir_bloqueadas || !doc.bloqueada(e))) {
        let d = distancia(doc, &e.geometria, p);
        if d <= tolerancia && melhor.map_or(true, |(m, _)| d <= m) {
            melhor = Some((d, e.id));
        }
    }
    melhor.map(|(_, id)| id)
}

/// As entidades visíveis que a janela pega.
pub fn escolher_na_janela(doc: &Documento, canto1: Ponto, canto2: Ponto, cruzando: bool) -> Vec<Id> {
    let janela = (Ponto::novo(canto1.x.min(canto2.x), canto1.y.min(canto2.y)), Ponto::novo(canto1.x.max(canto2.x), canto1.y.max(canto2.y)));
    doc.entidades().iter().filter(|e| doc.visivel(e) && !doc.bloqueada(e) && na_janela(doc, &e.geometria, janela, cruzando)).map(|e| e.id).collect()
}

/// Cópias de `entidades` deslocadas em grade: `linhas × colunas`, com o espaço entre elas (pode ser
/// negativo). A cópia (0, 0) é a própria entidade e não entra. No máximo `MAXIMO_DA_MATRIZ` cópias.
pub fn matriz(entidades: &[&Entidade], linhas: usize, colunas: usize, espaco_linha: f64, espaco_coluna: f64) -> Vec<(usize, Geometria)> {
    let mut fora = Vec::new();
    'grade: for i in 0..linhas {
        for j in 0..colunas {
            if (i, j) == (0, 0) {
                continue;
            }
            for (k, e) in entidades.iter().enumerate() {
                if fora.len() >= MAXIMO_DA_MATRIZ {
                    break 'grade;
                }
                fora.push((k, transformar(&e.geometria, &Transf::mover(j as f64 * espaco_coluna, i as f64 * espaco_linha))));
            }
        }
    }
    fora
}

/// Teto de cópias de uma matriz.
pub const MAXIMO_DA_MATRIZ: usize = 10_000;

/// O retângulo de cantos opostos `a` e `b` como polilinha fechada.
pub fn retangulo(a: Ponto, b: Ponto) -> Geometria {
    let vertices = [a, Ponto::novo(b.x, a.y), b, Ponto::novo(a.x, b.y)].into_iter().map(|p| Vertice { p, bulge: 0.0 }).collect();
    Geometria::Polilinha { vertices, fechada: true }
}

/// A elipse de centro `centro`, com o semi-eixo maior até `ponta` e o semi-eixo menor de comprimento
/// `menor`, como polilinha fechada de 72 lados. `None` se o eixo ou o menor for nulo.
pub fn elipse(centro: Ponto, ponta: Ponto, menor: f64) -> Option<Geometria> {
    let (mx, my) = (ponta.x - centro.x, ponta.y - centro.y);
    let maior = mx.hypot(my);
    if maior < 1e-12 || menor < 1e-12 {
        return None;
    }
    let (nx, ny) = (-my / maior * menor, mx / maior * menor);
    let vertices = (0..72)
        .map(|k| {
            let (s, c) = (std::f64::consts::TAU * f64::from(k) / 72.0).sin_cos();
            Vertice { p: Ponto::novo(centro.x + mx * c + nx * s, centro.y + my * c + ny * s), bulge: 0.0 }
        })
        .collect();
    Some(Geometria::Polilinha { vertices, fechada: true })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::documento::Documento;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    #[test]
    fn transformar_gira_arco_e_texto_e_espelha_sem_espelhar_o_texto() {
        let arco = Geometria::Arco { centro: p(0.0, 0.0), raio: 2.0, inicio: 0.0, fim: 90.0 };
        let Geometria::Arco { centro, inicio, fim, .. } = transformar(&arco, &Transf::girar(p(0.0, 0.0), 90.0)) else { panic!() };
        assert!((centro.x).abs() < 1e-9 && (inicio - 90.0).abs() < 1e-9 && (fim - 180.0).abs() < 1e-9);
        // Espelho no eixo Y (x = 0): o arco de 0° a 90° vira o de 90° a 180°.
        let esp = Transf::espelhar(p(0.0, 0.0), p(0.0, 1.0)).unwrap();
        let Geometria::Arco { inicio, fim, .. } = transformar(&arco, &esp) else { panic!() };
        assert!((inicio - 90.0).abs() < 1e-9 && (fim - 180.0).abs() < 1e-9, "{inicio} {fim}");
        let texto = Geometria::Texto { ponto: p(1.0, 0.0), altura: 1.0, rotacao: 10.0, conteudo: "a".into(), h: crate::documento::AlinhaH::Esquerda, v: crate::documento::AlinhaV::Base };
        let Geometria::Texto { ponto, rotacao, .. } = transformar(&texto, &esp) else { panic!() };
        assert!((ponto.x + 1.0).abs() < 1e-9 && (rotacao - 10.0).abs() < 1e-9, "o texto espelha de lugar mas não vira ao contrário");
        let pl = Geometria::Polilinha { vertices: vec![Vertice { p: p(0.0, 0.0), bulge: 0.5 }, Vertice { p: p(1.0, 0.0), bulge: 0.0 }], fechada: false };
        let Geometria::Polilinha { vertices, .. } = transformar(&pl, &esp) else { panic!() };
        assert_eq!(vertices[0].bulge, -0.5);
    }

    #[test]
    fn transformar_escala_o_raio_a_altura_e_a_insercao() {
        let t = Transf::escalar(p(0.0, 0.0), 2.0);
        assert!(matches!(transformar(&Geometria::Circulo { centro: p(1.0, 1.0), raio: 3.0 }, &t), Geometria::Circulo { raio, .. } if raio == 6.0));
        let ins = Geometria::Insercao { bloco: 0, ponto: p(1.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 };
        let Geometria::Insercao { ponto, escala_x, escala_y, .. } = transformar(&ins, &t) else { panic!() };
        assert_eq!((ponto.x, escala_x, escala_y), (2.0, 2.0, 2.0));
        let esp = Transf::espelhar(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let Geometria::Insercao { escala_y, rotacao, .. } = transformar(&Geometria::Insercao { bloco: 0, ponto: p(0.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 30.0 }, &esp) else { panic!() };
        assert_eq!(escala_y, -1.0);
        assert!((rotacao - 330.0).abs() < 1e-9, "{rotacao}");
    }

    #[test]
    fn esticar_move_so_o_que_esta_na_janela() {
        let janela = (p(-1.0, -1.0), p(1.0, 1.0));
        let l = Geometria::Linha { a: p(0.0, 0.0), b: p(5.0, 0.0) };
        assert_eq!(esticar(&l, janela, 2.0, 3.0), Some(Geometria::Linha { a: p(2.0, 3.0), b: p(5.0, 0.0) }));
        assert_eq!(esticar(&Geometria::Linha { a: p(3.0, 3.0), b: p(5.0, 0.0) }, janela, 2.0, 3.0), None);
        let c = Geometria::Circulo { centro: p(0.5, 0.5), raio: 1.0 };
        assert_eq!(esticar(&c, janela, 2.0, 0.0), Some(Geometria::Circulo { centro: p(2.5, 0.5), raio: 1.0 }));
    }

    #[test]
    fn escolher_pega_a_mais_perto_dentro_da_tolerancia_e_janela_distingue_inteira_de_cruzando() {
        let mut d = Documento::default();
        let a = d.adicionar_linha(p(0.0, 0.0), p(10.0, 0.0));
        let b = d.adicionar_circulo(p(20.0, 0.0), 2.0).unwrap();
        assert_eq!(escolher(&d, p(5.0, 0.3), 0.5), Some(a));
        assert_eq!(escolher(&d, p(5.0, 3.0), 0.5), None);
        assert_eq!(escolher(&d, p(20.0, 2.1), 0.5), Some(b));
        // Janela sobre a metade da linha: só cruzando pega; a janela que cobre o círculo inteiro pega nos dois modos.
        assert_eq!(escolher_na_janela(&d, p(-1.0, -1.0), p(5.0, 1.0), false), Vec::<Id>::new());
        assert_eq!(escolher_na_janela(&d, p(5.0, 1.0), p(-1.0, -1.0), true), vec![a]);
        assert_eq!(escolher_na_janela(&d, p(17.0, -3.0), p(23.0, 3.0), false), vec![b]);
    }

    #[test]
    fn matriz_copia_em_grade_sem_repetir_a_original_e_respeita_o_teto() {
        let mut d = Documento::default();
        d.adicionar_linha(p(0.0, 0.0), p(1.0, 0.0));
        let e: Vec<&Entidade> = d.entidades().iter().collect();
        let m = matriz(&e, 2, 3, 5.0, 10.0);
        assert_eq!(m.len(), 5, "2×3 menos a original");
        assert!(m.iter().any(|(_, g)| *g == Geometria::Linha { a: p(20.0, 5.0), b: p(21.0, 5.0) }));
        assert_eq!(matriz(&e, 1000, 1000, 1.0, 1.0).len(), MAXIMO_DA_MATRIZ);
    }

    #[test]
    fn retangulo_e_elipse() {
        assert!(matches!(retangulo(p(0.0, 0.0), p(4.0, 3.0)), Geometria::Polilinha { ref vertices, fechada: true } if vertices.len() == 4 && vertices[2].p == p(4.0, 3.0)));
        let Some(Geometria::Polilinha { vertices, .. }) = elipse(p(0.0, 0.0), p(4.0, 0.0), 2.0) else { panic!() };
        let maxy = vertices.iter().map(|v| v.p.y).fold(f64::MIN, f64::max);
        assert!((maxy - 2.0).abs() < 1e-9 && (vertices[0].p.x - 4.0).abs() < 1e-9);
        assert!(elipse(p(0.0, 0.0), p(0.0, 0.0), 2.0).is_none());
    }
}
