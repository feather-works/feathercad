//! O cubo de vista (o ViewCube do AutoCAD; o antigo `vista-cubo.ts`): um cubo desenhado com a mesma rotação da vista,
//! com a bússola por baixo. Clicar numa face, aresta ou quina olha dali; arrastar gira (órbita); ⌂ volta à vista de cima.
//! A projeção é a do motor (`Vista`: gira rz em torno de Z, depois rx em torno de X).

use pelicano::{Border, Captured, Color, CursorIcon, Event, FillRule, Font, MouseButton, Path, Point, Program, Rect, Renderer, Stroke, Theme};

/// Quanto o mouse anda para o clique virar órbita.
const LIMIAR_PX: f32 = 3.0;
/// Graus de giro por pixel arrastado.
const GRAUS_POR_PX: f64 = 0.6;

/// O que o cubo pede: olhar de `(rx, rz)` (graus).
pub type Pedido = (f64, f64);

pub struct Cubo<M> {
    pub rx: f64,
    pub rz: f64,
    pub claro: bool,
    pub acento: Color,
    /// TOPO, BAIXO, FRENTE, TRÁS, DIREITA, ESQUERDA (na ordem de `FACES`).
    pub rotulos: [String; 6],
    /// N, S, L, O.
    pub bussola: [String; 4],
    pub msg: fn(Pedido) -> M,
}

/// As faces: normal (de onde se olha) e os quatro cantos.
const FACES: [([f64; 3], [[f64; 3]; 4]); 6] = [
    ([0.0, 0.0, 1.0], [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]]),
    ([0.0, 0.0, -1.0], [[-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, -1.0, -1.0]]),
    ([0.0, -1.0, 0.0], [[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0]]),
    ([0.0, 1.0, 0.0], [[1.0, 1.0, -1.0], [-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0]]),
    ([1.0, 0.0, 0.0], [[1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]]),
    ([-1.0, 0.0, 0.0], [[-1.0, 1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]]),
];

#[derive(Default)]
pub struct EstadoCubo {
    apertado: Option<Point>,
    orbitou: bool,
    /// O alvo sob o mouse (face, aresta ou quina): a direção de onde se olharia.
    sobre: Option<[f64; 3]>,
}

/// A geometria do cubo no retângulo: centro, escala e a rotação.
struct Proj {
    c: Point,
    s: f64,
    cz: f64,
    sz: f64,
    cx: f64,
    sx: f64,
}

impl Proj {
    fn nova(b: Rect, rx: f64, rz: f64) -> Proj {
        let (rx, rz) = (rx.to_radians(), rz.to_radians());
        // O cubo fica no alto; a bússola, embaixo dele.
        let c = Point::new(b.x + b.width / 2.0, b.y + b.width / 2.0 + 6.0);
        Proj { c, s: f64::from(b.width) * 0.24, cz: rz.cos(), sz: rz.sin(), cx: rx.cos(), sx: rx.sin() }
    }
    /// Mundo → tela e a profundidade (positiva = para o observador).
    fn tela(&self, p: [f64; 3]) -> (Point, f64) {
        let x1 = p[0] * self.cz + p[1] * self.sz;
        let y1 = -p[0] * self.sz + p[1] * self.cz;
        let yv = y1 * self.cx + p[2] * self.sx;
        let fundo = -y1 * self.sx + p[2] * self.cx;
        (Point::new(self.c.x + (x1 * self.s) as f32, self.c.y - (yv * self.s) as f32), fundo)
    }
}

/// De onde se olha (direção do observador) → `(rx, rz)` da vista.
pub fn vista_de(d: [f64; 3]) -> Pedido {
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
    let (dx, dy, dz) = (d[0] / n, d[1] / n, d[2] / n);
    let rx = dz.clamp(-1.0, 1.0).acos().to_degrees();
    let rz = if (dx * dx + dy * dy).sqrt() < 1e-6 { 0.0 } else { dx.atan2(-dy).to_degrees().rem_euclid(360.0) };
    (rx, rz)
}

/// Os 26 alvos (6 faces, 12 arestas, 8 quinas) como direções com componentes em {-1, 0, 1}.
fn alvos() -> impl Iterator<Item = [f64; 3]> {
    (-1..=1).flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| [f64::from(x), f64::from(y), f64::from(z)]))).filter(|p| p != &[0.0; 3])
}

fn dentro(p: Point, poli: &[Point]) -> bool {
    let mut d = false;
    let mut j = poli.len() - 1;
    for i in 0..poli.len() {
        let (a, b) = (poli[i], poli[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            d = !d;
        }
        j = i;
    }
    d
}

impl<M> Cubo<M> {
    /// O alvo sob o ponto: o mais próximo dentre os visíveis, desde que o ponto caia numa face visível.
    fn alvo_em(&self, b: Rect, p: Point) -> Option<[f64; 3]> {
        let pr = Proj::nova(b, self.rx, self.rz);
        let sobre_o_cubo = FACES.iter().any(|(n, cantos)| pr.tela(*n).1 > 1e-3 && dentro(p, &cantos.map(|c| pr.tela(c).0)));
        if !sobre_o_cubo {
            return None;
        }
        alvos()
            // Só o que se vê: a aresta lateral, de cima, fica de perfil (profundidade 0) e não conta.
            .filter(|a| pr.tela(*a).1 > 0.05)
            .min_by(|a, c| {
                let d = |q: &[f64; 3]| {
                    let t = pr.tela([q[0] * 0.85, q[1] * 0.85, q[2] * 0.85]).0;
                    (t.x - p.x).hypot(t.y - p.y)
                };
                d(a).total_cmp(&d(c))
            })
    }

    /// As letras da bússola e onde ficam (no chão, um pouco além do cubo).
    fn letras(&self, b: Rect) -> [(Point, usize, f64); 4] {
        let pr = Proj::nova(b, self.rx, self.rz);
        let r = 1.75;
        // N = +Y, S = −Y, L = +X, O = −X; o clique olha de lá (rz), mantendo a inclinação.
        [([0.0, r, -1.0], 0, 180.0), ([0.0, -r, -1.0], 1, 0.0), ([r, 0.0, -1.0], 2, 90.0), ([-r, 0.0, -1.0], 3, 270.0)].map(|(p, i, rz)| (pr.tela(p).0, i, rz))
    }

    fn casa(b: Rect) -> Rect {
        Rect { x: b.x + 2.0, y: b.y + 2.0, width: 18.0, height: 18.0 }
    }
}

impl<M> Program<M> for Cubo<M> {
    type State = EstadoCubo;

    fn draw(&self, st: &EstadoCubo, r: &mut Renderer, b: Rect, _: &Theme, _: Option<Point>) {
        let pr = Proj::nova(b, self.rx, self.rz);
        let (face, borda, texto) = if self.claro {
            (Color::rgb8(0xf4, 0xf6, 0xf8), Color::rgb8(0x9a, 0xa0, 0xa6), Color::rgb8(0x3c, 0x40, 0x43))
        } else {
            (Color::rgb8(0x3a, 0x3f, 0x47), Color::rgb8(0x80, 0x88, 0x92), Color::rgb8(0xe8, 0xea, 0xed))
        };
        // A bússola: um anel no chão (z = −1), por baixo do cubo.
        let anel: Vec<Point> = (0..=48).map(|i| {
            let a = f64::from(i) / 48.0 * std::f64::consts::TAU;
            pr.tela([1.75 * a.cos(), 1.75 * a.sin(), -1.0]).0
        }).collect();
        if let Some(p) = Path::polyline(&anel, true) {
            r.stroke_path(&p, &Stroke::new(5.0), borda.alpha(0.35));
        }
        for (pt, i, _) in self.letras(b) {
            let w = r.measure(&self.bussola[i], 10.0, Font::SEMIBOLD, None);
            let cor = if i == 0 { Color::rgb8(0xe0, 0x4b, 0x4b) } else { texto };
            r.draw_text(&self.bussola[i], 10.0, Font::SEMIBOLD, cor, Point::new(pt.x - w.width / 2.0, pt.y - w.height / 2.0), None);
        }
        // As faces de trás para a frente; só as viradas para o observador.
        let mut visiveis: Vec<(f64, usize)> = FACES.iter().enumerate().map(|(i, (n, _))| (pr.tela(*n).1, i)).filter(|(d, _)| *d > 1e-3).collect();
        visiveis.sort_by(|a, c| a.0.total_cmp(&c.0));
        for (prof, i) in visiveis {
            let (n, cantos) = FACES[i];
            let pts = cantos.map(|c| pr.tela(c).0);
            let Some(p) = Path::polyline(&pts, true) else { continue };
            // Mais de frente = mais claro: o cubo ganha volume.
            let luz = (0.55 + 0.45 * prof) as f32;
            let cor = face.mix(if self.claro { Color::WHITE } else { Color::rgb8(0x5a, 0x61, 0x6b) }, luz - 0.55);
            let sobre = st.sobre == Some(n);
            r.fill_path(&p, if sobre { cor.mix(self.acento, 0.45) } else { cor }, FillRule::NonZero);
            r.stroke_path(&p, &Stroke::new(1.0), borda);
            // O rótulo só quando a face está bem de frente (não vira risco ilegível).
            if prof > 0.5 {
                let c = pr.tela(n).0;
                // Cabe na largura que a face tem na tela (de lado ela estreita).
                let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
                let tam = ((x1 - x0) * 0.8 / (self.rotulos[i].chars().count().max(1) as f32 * 0.62)).clamp(6.0, 11.0);
                let w = r.measure(&self.rotulos[i], tam, Font::SEMIBOLD, None);
                r.draw_text(&self.rotulos[i], tam, Font::SEMIBOLD, texto.alpha((prof as f32 * 1.4).min(1.0)), Point::new(c.x - w.width / 2.0, c.y - w.height / 2.0), None);
            }
        }
        // Aresta ou quina sob o mouse: um ponto de destaque.
        if let Some(a) = st.sobre.filter(|a| a.iter().filter(|v| **v != 0.0).count() > 1) {
            let p = pr.tela(a.map(|v| v * 0.9)).0;
            if let Some(c) = Path::circle(p, 4.0) {
                r.fill_path(&c, self.acento, FillRule::NonZero);
            }
        }
        let casa = Self::casa(b);
        r.fill_quad(casa, Some(&pelicano::Background::Color(face)), Border::rounded(4.0).color(borda, 1.0), None);
        r.draw_text("⌂", 12.0, Font::DEFAULT, texto, Point::new(casa.x + 3.0, casa.y + 1.0), None);
    }

    fn update(&self, st: &mut EstadoCubo, ev: &Event, b: Rect, cursor: Option<Point>, publicar: &mut dyn FnMut(M)) -> Captured {
        let dentro_b = cursor.is_some_and(|p| b.contains(p));
        match ev {
            Event::CursorMoved(p) => {
                if let Some(a) = st.apertado {
                    let (dx, dy) = (p.x - a.x, p.y - a.y);
                    if st.orbitou || dx.hypot(dy) > LIMIAR_PX {
                        st.orbitou = true;
                        st.apertado = Some(*p);
                        publicar((self.msg)((self.rx - f64::from(dy) * GRAUS_POR_PX, self.rz - f64::from(dx) * GRAUS_POR_PX)));
                    }
                    return Captured::Yes;
                }
                st.sobre = if dentro_b { self.alvo_em(b, *p) } else { None };
            }
            Event::MousePressed(MouseButton::Left) if dentro_b => {
                st.apertado = cursor;
                st.orbitou = false;
                return Captured::Yes;
            }
            Event::MouseReleased(MouseButton::Left) if st.apertado.is_some() => {
                st.apertado = None;
                if st.orbitou {
                    return Captured::Yes;
                }
                let Some(p) = cursor else { return Captured::Yes };
                if Self::casa(b).contains(p) {
                    publicar((self.msg)((0.0, 0.0)));
                } else if let Some(a) = self.alvo_em(b, p) {
                    publicar((self.msg)(vista_de(a)));
                } else if let Some((_, _, rz)) = self.letras(b).into_iter().find(|(l, _, _)| (l.x - p.x).hypot(l.y - p.y) < 10.0) {
                    publicar((self.msg)((if self.rx.abs() < 1.0 { 60.0 } else { self.rx }, rz)));
                }
                return Captured::Yes;
            }
            _ => {}
        }
        Captured::No
    }

    fn mouse_interaction(&self, st: &EstadoCubo, _: Rect, _: Option<Point>) -> CursorIcon {
        if st.apertado.is_some() && st.orbitou {
            CursorIcon::Grabbing
        } else if st.sobre.is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        }
    }
}

#[cfg(test)]
mod testes {
    use super::vista_de;

    #[test]
    fn cada_face_olha_de_onde_o_motor_espera() {
        let perto = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
        assert!(perto(vista_de([0.0, 0.0, 1.0]), (0.0, 0.0)), "topo");
        assert!(perto(vista_de([0.0, -1.0, 0.0]), (90.0, 0.0)), "frente");
        assert!(perto(vista_de([1.0, 0.0, 0.0]), (90.0, 90.0)), "direita");
        assert!(perto(vista_de([0.0, 1.0, 0.0]), (90.0, 180.0)), "trás");
        assert!(perto(vista_de([-1.0, 0.0, 0.0]), (90.0, 270.0)), "esquerda");
        assert!(perto(vista_de([0.0, 0.0, -1.0]), (180.0, 0.0)), "baixo");
        let (rx, rz) = vista_de([1.0, -1.0, 1.0]);
        assert!((rz - 45.0).abs() < 1e-6 && (rx - 54.7356).abs() < 1e-3, "quina iso: {rx} {rz}");
    }
}
