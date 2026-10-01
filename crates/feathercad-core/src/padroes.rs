//! Biblioteca de padrões de hachura (concreto, areia, terra, brita, tijolo, aço, grade…): famílias de linhas paralelas com
//! traço, vão e ponto, como o `.pat` do AutoCAD. As medidas estão em **unidades de `p`**, o espaço-base do padrão, que sai do
//! tamanho do contorno (ver [`para_contornos`]) vezes a escala do padrão que o usuário escolhe.

use crate::documento::LinhaDePadrao;
use crate::geom::{Ponto, Vertice};
use crate::quadro::tracos_do_padrao;

/// O nome da hachura sólida (não é um padrão de linhas).
pub const SOLIDA: &str = "SOLID";
/// O padrão de fábrica (linhas a 45°).
pub const PADRAO_DE_FABRICA: &str = "ANSI31";

/// Os padrões, na ordem em que aparecem na lista. O texto de cada um está nos arquivos de língua (`hachura.padroes.<NOME>`).
pub const NOMES: [&str; 12] = ["ANSI31", "ANSI32", "ANSI37", "HORIZONTAL", "VERTICAL", "GRADE", "PONTOS", "CONCRETO", "AREIA", "BRITA", "TERRA", "TIJOLO"];

/// As escalas oferecidas (multiplicam o espaço-base) e o texto de cada uma.
pub const ESCALAS: [(f64, &str); 7] = [(0.25, "0,25×"), (0.5, "0,5×"), (0.75, "0,75×"), (1.0, "1×"), (1.5, "1,5×"), (2.0, "2×"), (4.0, "4×")];

/// O nome é um padrão da biblioteca (ou a sólida)? Devolve o nome canônico.
pub fn reconhecer(nome: &str) -> Option<&'static str> {
    let n = nome.trim();
    if n.eq_ignore_ascii_case(SOLIDA) || n.eq_ignore_ascii_case("solida") || n.eq_ignore_ascii_case("sólida") || n.eq_ignore_ascii_case("solid") {
        return Some(SOLIDA);
    }
    NOMES.iter().copied().find(|c| c.eq_ignore_ascii_case(n))
}

/// Uma família: `ang` graus, origem deslocada `(ox, oy)` da base, `along` = deslocamento ao longo da linha a cada linha da
/// família (a fase do traço), `perp` = espaço entre as linhas, `tracos` (vazio = contínua). Tudo em unidades de `p`.
fn fam(base: Ponto, p: f64, ang: f64, (ox, oy): (f64, f64), along: f64, perp: f64, tracos: &[f64]) -> LinhaDePadrao {
    let (s, c) = ang.to_radians().sin_cos();
    LinhaDePadrao {
        angulo: ang,
        base: Ponto::novo(base.x + ox * p, base.y + oy * p),
        dx: (along * c - perp * s) * p,
        dy: (along * s + perp * c) * p,
        tracos: tracos.iter().map(|t| t * p).collect(),
    }
}

/// As famílias do padrão `nome` ancoradas em `base`, com espaço-base `p` (unidades do desenho). `None` se não existe.
pub fn montar(nome: &str, base: Ponto, p: f64) -> Option<Vec<LinhaDePadrao>> {
    let f = |ang, o, along, perp, tracos: &[f64]| fam(base, p, ang, o, along, perp, tracos);
    Some(match reconhecer(nome)? {
        "ANSI31" => vec![f(45.0, (0.0, 0.0), 0.0, 1.0, &[])],
        // aço: pares de linhas coladas
        "ANSI32" => vec![f(45.0, (0.0, 0.0), 0.0, 3.0, &[]), f(45.0, (-1.0, 1.0), 0.0, 3.0, &[])],
        "ANSI37" => vec![f(45.0, (0.0, 0.0), 0.0, 1.0, &[]), f(135.0, (0.0, 0.0), 0.0, 1.0, &[])],
        "HORIZONTAL" => vec![f(0.0, (0.0, 0.0), 0.0, 1.0, &[])],
        "VERTICAL" => vec![f(90.0, (0.0, 0.0), 0.0, 1.0, &[])],
        "GRADE" => vec![f(0.0, (0.0, 0.0), 0.0, 1.0, &[]), f(90.0, (0.0, 0.0), 0.0, 1.0, &[])],
        "PONTOS" => vec![f(0.0, (0.0, 0.0), 0.0, 1.0, &[0.0, -1.0])],
        // concreto: pontinhos e riscos miúdos em seis direções "sem ordem", como o AR-CONC
        "CONCRETO" => vec![
            f(50.0, (0.0, 0.0), 0.9, 2.3, &[0.0, -1.9]),
            f(355.0, (0.3, 0.1), 0.7, 2.1, &[0.15, -2.6]),
            f(100.4, (0.6, 0.4), 1.3, 2.6, &[0.2, -3.1]),
            f(46.1, (0.2, 0.7), 0.5, 1.9, &[0.0, -2.3]),
            f(96.6, (0.9, 0.2), 1.1, 2.9, &[0.12, -2.2]),
            f(351.2, (0.5, 0.9), 0.3, 2.4, &[0.18, -3.3]),
        ],
        "AREIA" => vec![
            f(37.0, (0.0, 0.0), 0.4, 1.1, &[0.0, -1.7]),
            f(127.0, (0.3, 0.2), 0.9, 1.3, &[0.0, -2.1]),
            f(82.0, (0.6, 0.1), 0.2, 1.7, &[0.0, -2.6]),
            f(172.0, (0.1, 0.5), 0.6, 1.5, &[0.1, -1.9]),
        ],
        "BRITA" => vec![
            f(0.0, (0.0, 0.0), 0.7, 2.5, &[0.5, -2.2]),
            f(60.0, (0.4, 0.3), 1.1, 2.9, &[0.4, -2.8]),
            f(120.0, (0.8, 0.1), 0.3, 2.7, &[0.5, -2.4]),
            f(30.0, (0.2, 0.9), 1.6, 3.1, &[0.35, -3.2]),
            f(90.0, (0.7, 0.6), 0.9, 2.2, &[0.4, -2.6]),
        ],
        // terra (EARTH do AutoCAD, em unidades de 6,35 = 1): três fileiras tracejadas e riscos verticais
        "TERRA" => {
            // o EARTH original é miúdo: aqui cada unidade do desenho dele vale 3 espaços-base
            let g = |ang, o: (f64, f64), tracos: &[f64]| fam(base, p * 3.0, ang, o, 0.0, 1.0, tracos);
            vec![
                g(0.0, (0.0, 0.0), &[1.0, -1.0]),
                g(0.0, (0.0, 0.375), &[1.0, -1.0]),
                g(0.0, (0.0, 0.75), &[1.0, -1.0]),
                g(90.0, (0.125, 0.875), &[0.875, -1.125]),
                g(90.0, (0.375, 0.875), &[0.875, -1.125]),
                g(90.0, (0.625, 0.875), &[0.875, -1.125]),
            ]
        }
        // tijolo em fiadas: juntas horizontais e verticais alternadas
        "TIJOLO" => vec![f(0.0, (0.0, 0.0), 0.0, 2.0, &[]), f(90.0, (0.0, 0.0), 0.0, 4.0, &[2.0, -2.0]), f(90.0, (2.0, 2.0), 0.0, 4.0, &[2.0, -2.0])],
        _ => return None,
    })
}

/// O padrão `nome` para uns contornos: o espaço-base é a maior dimensão ÷ 40 × `escala`; a âncora é o canto do contorno.
/// Devolve as famílias e o nome canônico (`SOLID` = sem famílias). Nome desconhecido cai no padrão de fábrica.
pub fn para_contornos(nome: &str, escala: f64, contornos: &[Vec<Vertice>]) -> (Vec<LinhaDePadrao>, String) {
    let nome = reconhecer(nome).unwrap_or(PADRAO_DE_FABRICA);
    if nome == SOLIDA {
        return (Vec::new(), SOLIDA.to_string());
    }
    let pts = contornos.iter().flatten().map(|v| v.p);
    let (min, max) = pts.fold((Ponto::novo(f64::MAX, f64::MAX), Ponto::novo(f64::MIN, f64::MIN)), |(a, b), q| (Ponto::novo(a.x.min(q.x), a.y.min(q.y)), Ponto::novo(b.x.max(q.x), b.y.max(q.y))));
    let dimensao = (max.x - min.x).max(max.y - min.y);
    let p = (if dimensao.is_finite() && dimensao > 0.0 { dimensao / 40.0 } else { 1.0 }) * if escala.is_finite() && escala > 0.0 { escala } else { 1.0 };
    (montar(nome, min, p.max(1e-9)).unwrap_or_default(), nome.to_string())
}

/// Uma amostra do padrão em SVG (fundo escuro, traço claro), `largura × altura` px, para a caixa de escolha.
pub fn previa_svg(nome: &str, escala: f64, largura: f64, altura: f64) -> String {
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {largura} {altura}\"><rect width=\"100%\" height=\"100%\" fill=\"#1F252C\"/>");
    let quadrado = [vec![Vertice { p: Ponto::novo(4.0, 4.0), bulge: 0.0 }, Vertice { p: Ponto::novo(largura - 4.0, 4.0), bulge: 0.0 }, Vertice { p: Ponto::novo(largura - 4.0, altura - 4.0), bulge: 0.0 }, Vertice { p: Ponto::novo(4.0, altura - 4.0), bulge: 0.0 }]];
    let (familias, canonico) = para_contornos(nome, escala, &[vec![Vertice { p: Ponto::novo(0.0, 0.0), bulge: 0.0 }, Vertice { p: Ponto::novo(largura.max(altura), 0.0), bulge: 0.0 }]]);
    if canonico == SOLIDA {
        svg.push_str(&format!("<rect x=\"4\" y=\"4\" width=\"{}\" height=\"{}\" fill=\"#E0E0E0\"/>", largura - 8.0, altura - 8.0));
    } else {
        // a dimensão do contorno da prévia é a do lado maior (a escala do padrão é a mesma de uma hachura desse tamanho)
        let lacos: Vec<Vec<Ponto>> = quadrado.iter().map(|l| l.iter().map(|v| v.p).collect()).collect();
        let base = Ponto::novo(4.0, 4.0);
        let refeito = montar(&canonico, base, largura.max(altura) / 40.0 * if escala > 0.0 { escala } else { 1.0 }).unwrap_or(familias);
        if let Some(tracos) = tracos_do_padrao(&lacos, &refeito) {
            let mut d = String::new();
            for (a, b) in tracos {
                d.push_str(&format!("M{:.1} {:.1}L{:.1} {:.1}", a.x, a.y, b.x, b.y));
            }
            svg.push_str(&format!("<path d=\"{d}\" stroke=\"#E0E0E0\" stroke-width=\"0.9\" stroke-linecap=\"round\" fill=\"none\"/>"));
        }
    }
    svg.push_str(&format!("<rect x=\"4\" y=\"4\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#8A96A3\"/></svg>", largura - 8.0, altura - 8.0));
    svg
}

#[cfg(test)]
mod testes {
    use super::*;

    fn quadrado(lado: f64) -> Vec<Vec<Vertice>> {
        vec![[(0.0, 0.0), (lado, 0.0), (lado, lado), (0.0, lado)].iter().map(|(x, y)| Vertice { p: Ponto::novo(*x, *y), bulge: 0.0 }).collect()]
    }

    #[test]
    fn todo_padrao_da_lista_existe_e_gera_traco_dentro_do_contorno() {
        let lacos: Vec<Vec<Ponto>> = quadrado(100.0).iter().map(|l| l.iter().map(|v| v.p).collect()).collect();
        for nome in NOMES {
            let (fam, canon) = para_contornos(nome, 1.0, &quadrado(100.0));
            assert_eq!(canon, nome);
            let tracos = tracos_do_padrao(&lacos, &fam).unwrap_or_else(|| panic!("{nome} sem traços"));
            assert!(tracos.len() > 20, "{nome}: só {} traços", tracos.len());
            assert!(tracos.iter().all(|(a, b)| a.x > -1e-6 && a.y > -1e-6 && b.x < 100.0 + 1e-6 && b.y < 100.0 + 1e-6), "{nome} passou do contorno");
        }
    }

    #[test]
    fn ponto_e_traco_cortam_a_linha_e_nome_e_solida_sao_reconhecidos() {
        let lacos: Vec<Vec<Ponto>> = quadrado(40.0).iter().map(|l| l.iter().map(|v| v.p).collect()).collect();
        let cheia = tracos_do_padrao(&lacos, &montar("HORIZONTAL", Ponto::novo(0.0, 0.0), 1.0).unwrap()).unwrap().len();
        let tracejada = tracos_do_padrao(&lacos, &montar("TERRA", Ponto::novo(0.0, 0.0), 1.0).unwrap()).unwrap().len();
        assert!(tracejada > cheia, "o tracejado tem mais pedaços que a linha contínua");
        assert_eq!(reconhecer("concreto"), Some("CONCRETO"));
        assert_eq!(reconhecer("solid"), Some(SOLIDA));
        assert_eq!(reconhecer("xyz"), None);
        assert!(para_contornos("SOLID", 1.0, &quadrado(10.0)).0.is_empty());
    }
}
