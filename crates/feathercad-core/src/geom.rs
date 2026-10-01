//! Geometria básica do desenho (coordenadas de mundo, em unidades de desenho).

use serde::{Deserialize, Serialize};

/// Um ponto 3D/2D no espaço do desenho.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ponto {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub z: f64,
}

impl Ponto {
    /// Cria um ponto 2D (Z = 0).
    pub fn novo(x: f64, y: f64) -> Self {
        Self { x, y, z: 0.0 }
    }

    /// Cria um ponto 3D (X, Y, Z).
    pub fn novo3d(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Distância 2D até outro ponto.
    pub fn distancia(self, outro: Ponto) -> f64 {
        (self.x - outro.x).hypot(self.y - outro.y)
    }

    /// Distância 3D até outro ponto.
    pub fn distancia_3d(self, outro: Ponto) -> f64 {
        ((self.x - outro.x).powi(2) + (self.y - outro.y).powi(2) + (self.z - outro.z).powi(2)).sqrt()
    }

    /// Ângulo do vetor `self → outro`, em graus de 0 a 360 (anti-horário).
    pub fn angulo_ate(self, outro: Ponto) -> f64 {
        let graus = (outro.y - self.y).atan2(outro.x - self.x).to_degrees();
        if graus < 0.0 {
            graus + 360.0
        } else {
            graus
        }
    }
}

/// Um vértice de polilinha: o ponto e o *bulge* do segmento que **sai** dele
/// (`0` = reto; `tan(θ/4)` do ângulo θ do arco, positivo = anti-horário).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vertice {
    pub p: Ponto,
    #[serde(default)]
    pub bulge: f64,
}

/// Maior número de cordas em que um arco é dividido.
const CORDAS_MAX: usize = 720;

/// Quantas cordas aproximam um arco de raio `raio` e varredura `varredura` (rad) sem afastar mais que `tolerancia`.
fn cordas_do_arco(raio: f64, varredura: f64, tolerancia: f64) -> usize {
    let passo = if tolerancia >= raio { std::f64::consts::FRAC_PI_2 } else { 2.0 * (1.0 - tolerancia / raio).acos() };
    ((varredura.abs() / passo.max(1e-6)).ceil() as usize).clamp(1, CORDAS_MAX)
}

/// Pontos ao longo de um arco de circunferência (inclusive as duas pontas). `inicio` e
/// `varredura` em radianos; varredura negativa gira no sentido horário.
pub fn amostrar_arco(centro: Ponto, raio: f64, inicio: f64, varredura: f64, tolerancia: f64) -> Vec<Ponto> {
    let n = cordas_do_arco(raio, varredura, tolerancia);
    (0..=n)
        .map(|i| {
            let ang = inicio + varredura * i as f64 / n as f64;
            Ponto::novo3d(centro.x + raio * ang.cos(), centro.y + raio * ang.sin(), centro.z)
        })
        .collect()
}

/// Os pontos do segmento `a → b` com o `bulge` dado (inclusive `a` e `b`). Reto
/// (bulge ~0) dá só as duas pontas. O arco é aproximado por cordas que não se
/// afastam mais que `tolerancia` (em unidades do desenho) da curva.
pub fn segmento_com_bulge(a: Ponto, b: Ponto, bulge: f64, tolerancia: f64) -> Vec<Ponto> {
    let corda = a.distancia(b);
    if bulge.abs() < 1e-9 || corda < f64::EPSILON || !bulge.is_finite() {
        return vec![a, b];
    }
    let (ux, uy) = ((b.x - a.x) / corda, (b.y - a.y) / corda);
    // O centro fica sobre a mediatriz, à esquerda de a→b quando o arco é anti-horário.
    let desloc = (corda / 2.0) * (1.0 - bulge * bulge) / (2.0 * bulge);
    let centro = Ponto::novo3d((a.x + b.x) / 2.0 - uy * desloc, (a.y + b.y) / 2.0 + ux * desloc, (a.z + b.z) / 2.0);
    let mut pontos = amostrar_arco(centro, centro.distancia(a), (a.y - centro.y).atan2(a.x - centro.x), 4.0 * bulge.atan(), tolerancia);
    // As pontas exatas, sem o erro de arredondamento do seno/cosseno.
    if let (Some(p0), Some(p1)) = (pontos.first_mut(), Some(a)) {
        *p0 = p1;
    }
    if let Some(u) = pontos.last_mut() {
        *u = b;
    }
    pontos
}

/// Ângulo em graus normalizado para [0, 360).
pub fn normalizar_graus(g: f64) -> f64 {
    g.rem_euclid(360.0)
}

/// A varredura anti-horária, em graus, de `inicio` a `fim` (um arco de mesmo início e fim é a volta toda).
pub fn varredura_graus(inicio: f64, fim: f64) -> f64 {
    let v = normalizar_graus(fim - inicio);
    if v < 1e-9 {
        360.0
    } else {
        v
    }
}

/// Um arco calculado: centro, raio e os ângulos (graus, anti-horário do início ao fim).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcoCalculado {
    pub centro: Ponto,
    pub raio: f64,
    pub inicio: f64,
    pub fim: f64,
}

/// O arco que passa pelos três pontos, do primeiro ao terceiro **passando pelo segundo**.
/// `None` se os pontos estão alinhados (ou repetidos).
pub fn arco_por_3_pontos(p1: Ponto, p2: Ponto, p3: Ponto) -> Option<ArcoCalculado> {
    let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let (q1, q2, q3) = (p1.x * p1.x + p1.y * p1.y, p2.x * p2.x + p2.y * p2.y, p3.x * p3.x + p3.y * p3.y);
    let centro = Ponto::novo((q1 * (p2.y - p3.y) + q2 * (p3.y - p1.y) + q3 * (p1.y - p2.y)) / d, (q1 * (p3.x - p2.x) + q2 * (p1.x - p3.x) + q3 * (p2.x - p1.x)) / d);
    let raio = centro.distancia(p1);
    // Sentido do trajeto 1→2→3: anti-horário se o produto vetorial é positivo.
    let anti_horario = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x) > 0.0;
    let (de, ate) = if anti_horario { (p1, p3) } else { (p3, p1) };
    Some(ArcoCalculado { centro, raio, inicio: normalizar_graus(centro.angulo_ate(de)), fim: normalizar_graus(centro.angulo_ate(ate)) })
}

/// Uma transformação afim 2D: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`. É como um
/// `INSERT` leva as entidades do bloco para o desenho (escala, giro e deslocamento).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transf {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Transf {
    /// A que não muda nada.
    pub const IDENTIDADE: Transf = Transf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    /// A transformação de um `INSERT`: tira o ponto base do bloco, escala, gira `rotacao` graus e põe em `ponto`.
    pub fn de_insercao(base: Ponto, ponto: Ponto, escala_x: f64, escala_y: f64, rotacao: f64) -> Self {
        let (s, c) = rotacao.to_radians().sin_cos();
        let (a, b, cc, d) = (escala_x * c, escala_x * s, -escala_y * s, escala_y * c);
        Transf { a, b, c: cc, d, e: ponto.x - (a * base.x + cc * base.y), f: ponto.y - (b * base.x + d * base.y) }
    }

    /// Só deslocar.
    pub fn mover(dx: f64, dy: f64) -> Self {
        Transf { e: dx, f: dy, ..Self::IDENTIDADE }
    }

    /// Girar `graus` (anti-horário) em torno de `base`.
    pub fn girar(base: Ponto, graus: f64) -> Self {
        let (s, c) = graus.to_radians().sin_cos();
        Transf { a: c, b: s, c: -s, d: c, e: base.x - (c * base.x - s * base.y), f: base.y - (s * base.x + c * base.y) }
    }

    /// Escalar por `fator` (o mesmo nos dois eixos) a partir de `base`.
    pub fn escalar(base: Ponto, fator: f64) -> Self {
        Transf { a: fator, b: 0.0, c: 0.0, d: fator, e: base.x * (1.0 - fator), f: base.y * (1.0 - fator) }
    }

    /// Espelhar em relação à reta que passa por `p1` e `p2`. `None` se os pontos coincidem.
    pub fn espelhar(p1: Ponto, p2: Ponto) -> Option<Self> {
        if p1.distancia(p2) < 1e-12 {
            return None;
        }
        let (s2, c2) = (2.0 * p1.angulo_ate(p2).to_radians()).sin_cos();
        Some(Transf { a: c2, b: s2, c: s2, d: -c2, e: p1.x - (c2 * p1.x + s2 * p1.y), f: p1.y - (s2 * p1.x - c2 * p1.y) })
    }

    /// O determinante da parte linear: negativo = a transformação inverte o desenho (espelho).
    pub fn determinante(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// A transformação que desfaz esta; `None` se ela achata o desenho.
    pub fn inversa(&self) -> Option<Self> {
        let det = self.determinante();
        if det.abs() < 1e-18 {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Transf { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) })
    }

    /// Leva um ponto do bloco para o desenho (preservando elevação Z).
    pub fn aplicar(&self, p: Ponto) -> Ponto {
        Ponto::novo3d(self.a * p.x + self.c * p.y + self.e, self.b * p.x + self.d * p.y + self.f, p.z)
    }

    /// `self` aplicada **depois** de `filha` (o bloco de dentro sob o de fora).
    pub fn depois_de(&self, filha: &Transf) -> Transf {
        Transf {
            a: self.a * filha.a + self.c * filha.b,
            b: self.b * filha.a + self.d * filha.b,
            c: self.a * filha.c + self.c * filha.d,
            d: self.b * filha.c + self.d * filha.d,
            e: self.a * filha.e + self.c * filha.f + self.e,
            f: self.b * filha.e + self.d * filha.f + self.f,
        }
    }

    /// Não muda nada?
    pub fn eh_identidade(&self) -> bool {
        *self == Self::IDENTIDADE
    }

    /// O quanto a transformação estica, em média (raiz do determinante).
    pub fn escala_media(&self) -> f64 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }

    /// O quanto ela estica, no pior eixo (para calcular a tolerância dos arcos).
    pub fn escala_maxima(&self) -> f64 {
        self.a.hypot(self.b).max(self.c.hypot(self.d))
    }

    /// O giro do eixo X, em graus.
    pub fn rotacao_graus(&self) -> f64 {
        self.b.atan2(self.a).to_degrees()
    }
}

/// Triangula um polígono simples (sem ilhas) por "corte de orelhas". `None` se degenerado ou grande demais.
pub fn triangular(laco: &[Ponto]) -> Option<Vec<[Ponto; 3]>> {
    const MAXIMO: usize = 4000;
    if laco.len() < 3 || laco.len() > MAXIMO {
        return None;
    }
    let cruz = |a: Ponto, b: Ponto, c: Ponto| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let area2: f64 = (0..laco.len()).map(|i| {
        let (a, b) = (laco[i], laco[(i + 1) % laco.len()]);
        a.x * b.y - b.x * a.y
    }).sum();
    if area2.abs() < 1e-18 {
        return None;
    }
    // Sempre anti-horário, para "convexo" ser cruz > 0.
    let mut ix: Vec<usize> = if area2 > 0.0 { (0..laco.len()).collect() } else { (0..laco.len()).rev().collect() };
    let mut fora = Vec::new();
    let dentro = |a: Ponto, b: Ponto, c: Ponto, p: Ponto| cruz(a, b, p) >= 0.0 && cruz(b, c, p) >= 0.0 && cruz(c, a, p) >= 0.0;
    while ix.len() > 3 {
        let n = ix.len();
        let orelha = (0..n).find(|&i| {
            let (a, b, c) = (laco[ix[(i + n - 1) % n]], laco[ix[i]], laco[ix[(i + 1) % n]]);
            cruz(a, b, c) > 0.0 && !ix.iter().any(|&j| {
                let p = laco[j];
                p != a && p != b && p != c && dentro(a, b, c, p)
            })
        })?;
        fora.push([laco[ix[(orelha + n - 1) % n]], laco[ix[orelha]], laco[ix[(orelha + 1) % n]]]);
        ix.remove(orelha);
    }
    fora.push([laco[ix[0]], laco[ix[1]], laco[ix[2]]]);
    Some(fora)
}

#[cfg(test)]
mod testes_triangular {
    use super::*;

    #[test]
    fn triangula_um_l_em_quatro_triangulos_com_a_mesma_area_e_recusa_o_degenerado() {
        let p = Ponto::novo;
        let l = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 1.0), p(1.0, 1.0), p(1.0, 2.0), p(0.0, 2.0)];
        let t = triangular(&l).unwrap();
        assert_eq!(t.len(), 4);
        let area: f64 = t.iter().map(|[a, b, c]| ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0).sum();
        assert!((area - 3.0).abs() < 1e-9, "{area}");
        assert!(triangular(&[p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)]).is_none());
    }
}

#[cfg(test)]
mod testes_transf {
    use super::*;

    fn perto(a: Ponto, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
    }

    #[test]
    fn mover_girar_escalar_espelhar_e_inversa() {
        let p = Ponto::novo;
        assert!(perto(Transf::mover(2.0, -1.0).aplicar(p(1.0, 1.0)), 3.0, 0.0));
        assert!(perto(Transf::girar(p(1.0, 1.0), 90.0).aplicar(p(2.0, 1.0)), 1.0, 2.0));
        assert!(perto(Transf::escalar(p(1.0, 1.0), 3.0).aplicar(p(2.0, 2.0)), 4.0, 4.0));
        // Espelho no eixo Y (x = 3): (5, 2) → (1, 2).
        let e = Transf::espelhar(p(3.0, 0.0), p(3.0, 5.0)).unwrap();
        assert!(perto(e.aplicar(p(5.0, 2.0)), 1.0, 2.0) && e.determinante() < 0.0);
        // Espelho na diagonal y = x: (1, 4) → (4, 1).
        assert!(perto(Transf::espelhar(p(0.0, 0.0), p(1.0, 1.0)).unwrap().aplicar(p(1.0, 4.0)), 4.0, 1.0));
        let t = Transf::girar(p(1.0, 2.0), 30.0).depois_de(&Transf::escalar(p(0.0, 0.0), 2.0));
        assert!(perto(t.inversa().unwrap().aplicar(t.aplicar(p(3.0, -4.0))), 3.0, -4.0));
        assert!(Transf::espelhar(p(1.0, 1.0), p(1.0, 1.0)).is_none());
    }
}
