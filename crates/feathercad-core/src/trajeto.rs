//! Caminho percorrível (eixo de via, alinhamento, parede curva): distância acumulada, ponto e direção a uma
//! distância, trecho, paralela (miter) e estaqueamento. É a base de tudo que é "ao longo de um eixo".

use crate::documento::Geometria;
use crate::edicao::pontos_da_polilinha;
use crate::geom::Ponto;

/// Limite do miter: a paralela não se afasta mais que isto vezes a distância pedida nas quinas.
const MITER_MAXIMO: f64 = 3.0;

#[derive(Clone, Debug)]
pub struct Trajeto {
    pub pontos: Vec<Ponto>,
    acumulado: Vec<f64>,
}

impl Trajeto {
    /// Os pontos em ordem (repetidos seguidos são descartados). `None` se sobra menos de 2.
    pub fn novo(pontos: &[Ponto]) -> Option<Self> {
        let mut p: Vec<Ponto> = Vec::with_capacity(pontos.len());
        for q in pontos {
            if p.last().map_or(true, |u| u.distancia(*q) > 1e-12) {
                p.push(*q);
            }
        }
        if p.len() < 2 {
            return None;
        }
        let mut acumulado = vec![0.0];
        for j in p.windows(2) {
            acumulado.push(acumulado.last().copied().unwrap_or(0.0) + j[0].distancia(j[1]));
        }
        Some(Self { pontos: p, acumulado })
    }

    /// Linha, polilinha (arcos por bulge viram cordas finas) ou arco.
    pub fn de_geometria(g: &Geometria) -> Option<Self> {
        match g {
            Geometria::Linha { a, b } => Self::novo(&[*a, *b]),
            Geometria::Polilinha { vertices, fechada } => Self::novo(&pontos_da_polilinha(vertices, *fechada).into_iter().chain(fechada.then(|| vertices[0].p)).collect::<Vec<_>>()),
            Geometria::Arco { centro, raio, inicio, fim } => {
                let mut varrido = (fim - inicio).rem_euclid(360.0);
                if varrido == 0.0 {
                    varrido = 360.0;
                }
                let n = (varrido / 2.0).ceil().max(2.0) as usize;
                Self::novo(&(0..=n).map(|i| {
                    let a = (inicio + varrido * i as f64 / n as f64).to_radians();
                    Ponto::novo(centro.x + raio * a.cos(), centro.y + raio * a.sin())
                }).collect::<Vec<_>>())
            }
            _ => None,
        }
    }

    pub fn comprimento(&self) -> f64 {
        self.acumulado.last().copied().unwrap_or(0.0)
    }

    fn trecho_de(&self, s: f64) -> usize {
        self.acumulado.partition_point(|a| *a <= s).clamp(1, self.pontos.len() - 1) - 1
    }

    /// O ponto à distância `s` do início (preso às pontas) e a direção da tangente em graus (anti-horário do eixo X).
    pub fn em(&self, s: f64) -> (Ponto, f64) {
        let s = s.clamp(0.0, self.comprimento());
        let i = self.trecho_de(s);
        let (a, b) = (self.pontos[i], self.pontos[i + 1]);
        let t = (s - self.acumulado[i]) / (self.acumulado[i + 1] - self.acumulado[i]).max(1e-12);
        (Ponto::novo(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), (b.y - a.y).atan2(b.x - a.x).to_degrees())
    }

    /// Projeta `q` no caminho: (afastamento assinado — positivo à esquerda de quem anda —, distância ao longo do caminho).
    pub fn projetar(&self, q: Ponto) -> (f64, f64) {
        let mut melhor = (f64::MAX, 0.0, 0.0);
        for (i, j) in self.pontos.windows(2).enumerate() {
            let (a, b) = (j[0], j[1]);
            let (abx, aby) = (b.x - a.x, b.y - a.y);
            let l2 = (abx * abx + aby * aby).max(1e-24);
            let t = (((q.x - a.x) * abx + (q.y - a.y) * aby) / l2).clamp(0.0, 1.0);
            let d = (a.x + abx * t - q.x).hypot(a.y + aby * t - q.y);
            if d < melhor.0 {
                let cruz = abx * (q.y - a.y) - aby * (q.x - a.x);
                melhor = (d, if cruz >= 0.0 { d } else { -d }, self.acumulado[i] + t * l2.sqrt());
            }
        }
        (melhor.1, melhor.2)
    }

    /// O caminho entre as distâncias `de` e `ate` (pontas interpoladas).
    pub fn trecho(&self, de: f64, ate: f64) -> Vec<Ponto> {
        let (de, ate) = (de.clamp(0.0, self.comprimento()), ate.clamp(0.0, self.comprimento()));
        let mut v = vec![self.em(de).0];
        v.extend(self.pontos.iter().zip(&self.acumulado).filter(|(_, a)| **a > de + 1e-12 && **a < ate - 1e-12).map(|(p, _)| *p));
        v.push(self.em(ate).0);
        v
    }

    /// Paralela à distância `d` (positiva = à esquerda de quem anda), com quinas em miter.
    pub fn paralela(&self, d: f64) -> Vec<Ponto> {
        let normal = |a: Ponto, b: Ponto| {
            let l = a.distancia(b).max(1e-12);
            ((a.y - b.y) / l, (b.x - a.x) / l)
        };
        let n = self.pontos.len();
        (0..n)
            .map(|i| {
                let p = self.pontos[i];
                let n1 = if i + 1 < n { normal(p, self.pontos[i + 1]) } else { normal(self.pontos[i - 1], p) };
                let n0 = if i > 0 { normal(self.pontos[i - 1], p) } else { n1 };
                let soma = (n0.0 + n1.0, n0.1 + n1.1);
                let k = 1.0 + n0.0 * n1.0 + n0.1 * n1.1;
                let (vx, vy) = if k < 2.0 / (MITER_MAXIMO * MITER_MAXIMO) { n0 } else { (soma.0 / k, soma.1 / k) };
                Ponto::novo(p.x + vx * d, p.y + vy * d)
            })
            .collect()
    }
}

/// Estaca no formato brasileiro: `117+12,66` (progressiva `s` em metros; `passo` = comprimento da estaca, 20 m).
pub fn estaca_texto(s: f64, passo: f64) -> String {
    let mut n = (s / passo).floor();
    let mut resto = s - n * passo;
    if (resto - passo).abs() < 0.005 {
        n += 1.0;
        resto = 0.0;
    }
    format!("{}+{:05.2}", n as i64, resto.max(0.0)).replace('.', ",")
}

/// Quilometragem: `6+246` (km + metros).
pub fn km_texto(s: f64) -> String {
    let mut km = (s / 1000.0).floor();
    let mut m = (s - km * 1000.0).round();
    if m >= 1000.0 {
        km += 1.0;
        m = 0.0;
    }
    format!("{}+{:03}", km as i64, m as i64)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(x: f64, y: f64) -> Ponto {
        Ponto::novo(x, y)
    }

    #[test]
    fn comprimento_ponto_e_direcao() {
        let t = Trajeto::novo(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)]).unwrap();
        assert!((t.comprimento() - 20.0).abs() < 1e-9);
        let (q, ang) = t.em(15.0);
        assert!((q.x - 10.0).abs() < 1e-9 && (q.y - 5.0).abs() < 1e-9 && (ang - 90.0).abs() < 1e-9);
        assert_eq!(t.trecho(5.0, 15.0).len(), 3);
    }

    #[test]
    fn paralela_a_esquerda_com_quina_em_miter() {
        let t = Trajeto::novo(&[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)]).unwrap();
        let v = t.paralela(1.0);
        assert!((v[0].y - 1.0).abs() < 1e-9 && v[0].x.abs() < 1e-9);
        assert!((v[1].x - 9.0).abs() < 1e-9 && (v[1].y - 1.0).abs() < 1e-9, "{:?}", v[1]);
        assert!((v[2].x - 9.0).abs() < 1e-9);
    }

    #[test]
    fn estaca_e_km() {
        assert_eq!(estaca_texto(2352.66, 20.0), "117+12,66");
        assert_eq!(estaca_texto(40.0, 20.0), "2+00,00");
        assert_eq!(km_texto(6246.1), "6+246");
        assert_eq!(km_texto(999.6), "1+000");
    }
}
