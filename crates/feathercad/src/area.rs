//! A área de desenho: pinta o quadro do motor (grade, preenchimentos, traços, textos) e, por cima, o cursor, as guias, a
//! prévia e a janela de seleção. **Só desenha o que o motor mandou**: o mouse vai cru ao app, que pergunta ao motor.

use std::cell::RefCell;

use pelicano::{
    Border, Captured, Color, CursorIcon, Event, FillRule, Font, Modifiers, MouseButton, Path, PathBuilder, Point, Program, Rect, Renderer,
    Size, Stroke, Theme, Transform,
};

use crate::motor::{Cursor, Segmento};
use crate::quadro::{Imagem, Quadro, Texto};

const TAMANHO_DA_MIRA: f32 = 10.0;

/// As cores da área no tema (as do antigo `base.ts`).
#[derive(Clone, Copy)]
pub struct Cores {
    pub fundo: Color,
    pub grade: Color,
    pub grade_principal: Color,
    pub ponto: Color,
    pub cursor: Color,
    pub previa: Color,
    pub guia: Color,
    pub osnap: Color,
    pub janela_dentro: Color,
    pub janela_toca: Color,
    pub caixa: Color,
}

impl Cores {
    pub fn do_tema(claro: bool) -> Cores {
        let h = |s: &str| Color::hex(s).unwrap_or(Color::BLACK);
        if claro {
            Cores {
                fundo: Color::WHITE,
                grade: Color::rgba(0.0, 0.0, 0.0, 0.07),
                grade_principal: Color::rgba(0.0, 0.0, 0.0, 0.17),
                ponto: Color::rgba(0.0, 0.0, 0.0, 0.38),
                cursor: Color::BLACK,
                previa: h("#1a6fd1"),
                guia: h("#1f9d3a"),
                osnap: h("#c98a00"),
                janela_dentro: h("#1a6fd1"),
                janela_toca: h("#1f9d3a"),
                caixa: Color::WHITE,
            }
        } else {
            Cores {
                fundo: h("#212830"),
                grade: Color::rgba(1.0, 1.0, 1.0, 0.055),
                grade_principal: Color::rgba(1.0, 1.0, 1.0, 0.13),
                ponto: Color::rgba(1.0, 1.0, 1.0, 0.28),
                cursor: Color::WHITE,
                previa: h("#9bd1ff"),
                guia: h("#66d17a"),
                osnap: h("#ffd84a"),
                janela_dentro: h("#4d9cf0"),
                janela_toca: h("#5fd07e"),
                caixa: h("#0d1520"),
            }
        }
    }
}

fn cor(c: u32) -> Color {
    Color::rgb8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// O quadro pronto para pintar: os caminhos montados uma vez quando ele chega (não a cada repintura).
#[derive(Default)]
pub struct Cena {
    pub versao: Option<u64>,
    tracos: Vec<(Color, f32, Path)>,
    preenchimentos: Vec<(Color, Path)>,
    grade: Vec<(bool, Path)>,
    pontos: Option<Path>,
    textos: Vec<Texto>,
    /// Os contornos dos textos, montados na primeira pintura (precisam do `Renderer`).
    textos_prontos: RefCell<Option<Vec<(Color, Path)>>>,
    pub imagens: Vec<Imagem>,
}

impl Cena {
    /// `modo` = o fundo pedido (`nada`, `pontos`, `grade`); `tamanho` = o da área.
    pub fn nova(q: Quadro, modo: &str, tamanho: Size, vista: &feathercad_core::vista::Vista) -> Cena {
        let mut tracos: Vec<(Color, f32, PathBuilder)> = Vec::new();
        // Um caminho por cor e espessura (como o `stroke` por cor do canvas): poucos caminhos, muitos segmentos.
        for ([x1, y1, x2, y2], c, e) in &q.segmentos {
            let (c, e) = (cor(*c), e.max(1.0));
            let i = match tracos.iter().position(|(k, w, _)| *k == c && *w == e) {
                Some(i) => i,
                None => {
                    tracos.push((c, e, PathBuilder::new()));
                    tracos.len() - 1
                }
            };
            tracos[i].2.move_to(Point::new(*x1, *y1)).line_to(Point::new(*x2, *y2));
        }
        let preenchimentos = q
            .preenchimentos
            .iter()
            .filter_map(|p| {
                let mut b = PathBuilder::new();
                for laco in &p.lacos {
                    let mut pts = laco.iter();
                    let Some(&(x, y)) = pts.next() else { continue };
                    b.move_to(Point::new(x, y));
                    for &(x, y) in pts {
                        b.line_to(Point::new(x, y));
                    }
                    b.close();
                }
                Some((cor(p.cor), b.build()?))
            })
            .collect();
        let (grade, pontos) = if vista.rotacao_x.abs() > 1e-4 || vista.rotacao_z.abs() > 1e-4 { (grade_3d(modo, tamanho, vista), None) } else { fundo(&q, modo, tamanho) };
        Cena {
            versao: Some(q.versao),
            tracos: tracos.into_iter().filter_map(|(c, e, b)| Some((c, e, b.build()?))).collect(),
            preenchimentos,
            grade,
            pontos,
            textos: q.textos,
            textos_prontos: RefCell::new(None),
            imagens: q.imagens,
        }
    }

    fn pintar(&self, r: &mut Renderer) {
        for (c, p) in &self.preenchimentos {
            r.fill_path(p, *c, FillRule::EvenOdd);
        }
        for (c, e, p) in &self.tracos {
            r.stroke_path(p, &Stroke::new(*e), *c);
        }
        let mut prontos = self.textos_prontos.borrow_mut();
        let textos = prontos.get_or_insert_with(|| self.textos.iter().filter_map(|t| contorno_do_texto(r, t)).collect());
        for (c, p) in textos.iter() {
            r.fill_path(p, *c, FillRule::NonZero);
        }
    }
}

/// A grade no plano Z = 0 com a vista girada (o ramo 3D do antigo `pintura.ts`): linhas a cada potência de 10 que caiba,
/// principais a cada 5. Vista rasante (de frente/lado): só o horizonte e as verticais.
fn grade_3d(modo: &str, t: Size, v: &feathercad_core::vista::Vista) -> Vec<(bool, Path)> {
    if modo == "nada" {
        return Vec::new();
    }
    let (w, h, es) = (f64::from(t.width), f64::from(t.height), v.escala.max(1e-9));
    let (rx, rz) = (v.rotacao_x.to_radians(), v.rotacao_z.to_radians());
    let (cz, sz, cx, sx) = (rz.cos(), rz.sin(), rx.cos(), rx.sin());
    let cxp = v.centro.x * cz + v.centro.y * sz;
    let cyp = (-v.centro.x * sz + v.centro.y * cz) * cx + v.centro.z * sx;
    let tela = |px: f64, py: f64| {
        let (x1, y1) = (px * cz + py * sz, -px * sz + py * cz);
        Point::new(((x1 - cxp) * es + w / 2.0) as f32, (h / 2.0 - (y1 * cx - cyp) * es) as f32)
    };
    if cx.abs() < 0.08 {
        let horizonte = tela(v.centro.x, v.centro.y).y;
        let passo = 10f64.max(10f64.powf(((w / es) / 20.0).max(1.0).log10().floor()));
        let n = (w / (passo * es)).ceil() as i64 + 2;
        let mut b = PathBuilder::new();
        for k in -n..=n {
            let x = (w / 2.0 + k as f64 * passo * es) as f32;
            b.move_to(Point::new(x, 0.0)).line_to(Point::new(x, t.height));
        }
        return [(false, b.build()), (true, Path::new(|b| { b.move_to(Point::new(0.0, horizonte)).line_to(Point::new(t.width, horizonte)); }))].into_iter().filter_map(|(p, c)| Some((p, c?))).collect();
    }
    let mundo = |sxp: f64, syp: f64| {
        let xv = cxp + (sxp - w / 2.0) / es;
        let y1 = (cyp + (h / 2.0 - syp) / es) / cx;
        (xv * cz - y1 * sz, xv * sz + y1 * cz)
    };
    let cantos = [mundo(0.0, 0.0), mundo(w, 0.0), mundo(w, h), mundo(0.0, h)];
    let (x0, x1) = cantos.iter().fold((f64::MAX, f64::MIN), |(a, b), c| (a.min(c.0), b.max(c.0)));
    let (y0, y1) = cantos.iter().fold((f64::MAX, f64::MIN), |(a, b), c| (a.min(c.1), b.max(c.1)));
    let passo = 10f64.max(10f64.powf(((x1 - x0).max(y1 - y0).max(10.0) / 20.0).log10().floor()));
    let (i0, j0) = ((x0 / passo).floor() as i64 - 2, (y0 / passo).floor() as i64 - 2);
    let (ni, nj) = (((x1 / passo).ceil() as i64 + 2 - i0).min(150), ((y1 / passo).ceil() as i64 + 2 - j0).min(150));
    [false, true]
        .into_iter()
        .filter_map(|principal| {
            let mut b = PathBuilder::new();
            for i in (i0..=i0 + ni).filter(|i| (i.abs() % 5 == 0) == principal) {
                let x = i as f64 * passo;
                b.move_to(tela(x, j0 as f64 * passo)).line_to(tela(x, (j0 + nj) as f64 * passo));
            }
            for j in (j0..=j0 + nj).filter(|j| (j.abs() % 5 == 0) == principal) {
                let y = j as f64 * passo;
                b.move_to(tela(i0 as f64 * passo, y)).line_to(tela((i0 + ni) as f64 * passo, y));
            }
            Some((principal, b.build()?))
        })
        .collect()
}

/// O fundo nas posições que o motor mandou: linhas (normais e principais) ou pontos.
fn fundo(q: &Quadro, modo: &str, t: Size) -> (Vec<(bool, Path)>, Option<Path>) {
    match modo {
        "grade" => {
            let linhas = [false, true].into_iter().filter_map(|principal| {
                let mut b = PathBuilder::new();
                for &(x, _) in q.fundo_x.iter().filter(|(_, p)| *p == principal) {
                    let x = x.round() + 0.5;
                    b.move_to(Point::new(x, 0.0)).line_to(Point::new(x, t.height));
                }
                for &(y, _) in q.fundo_y.iter().filter(|(_, p)| *p == principal) {
                    let y = y.round() + 0.5;
                    b.move_to(Point::new(0.0, y)).line_to(Point::new(t.width, y));
                }
                Some((principal, b.build()?))
            });
            (linhas.collect(), None)
        }
        "pontos" => {
            let mut b = PathBuilder::new();
            for &(x, _) in &q.fundo_x {
                for &(y, _) in &q.fundo_y {
                    b.rect(Rect { x: x.round(), y: y.round(), width: 1.5, height: 1.5 });
                }
            }
            (Vec::new(), b.build())
        }
        _ => (Vec::new(), None),
    }
}

/// O contorno de um texto na posição, giro e alinhamento do motor (o `at` do pelicano é o topo da linha).
fn contorno_do_texto(r: &mut Renderer, t: &Texto) -> Option<(Color, Path)> {
    if t.altura < 1.0 || t.conteudo.trim().is_empty() {
        return None; // menor que um pixel não se lê; não vale o custo do contorno
    }
    let largura = r.measure(&t.conteudo, t.altura, Font::DEFAULT, None).width;
    let dx = match t.h {
        1 => -largura / 2.0,
        2 => -largura,
        _ => 0.0,
    };
    // Da linha de base (como no canvas) para o topo da linha, que é onde o pelicano começa.
    let dy = match t.base {
        1 => -1.2,
        2 => -0.6,
        3 => 0.0,
        _ => -0.93,
    } * t.altura;
    let p = r.text_path(&t.conteudo, t.altura, Font::DEFAULT, Point::new(dx, dy), None)?;
    let giro = Transform::rotate(t.giro.to_degrees()).then(Transform::translate(t.x, t.y));
    Some((cor(t.cor), p.transform(giro)?))
}

/// O que a área avisa ao app (posições já relativas à área).
#[derive(Debug, Clone)]
pub enum EventoArea {
    Mover(Point),
    /// Botão, posição na área, posição na janela (para o menu) e Shift.
    Apertar(MouseButton, Point, Point, bool),
    Soltar(MouseButton, Point),
    Roda(f32, Point),
    Saiu,
    Tamanho(Size),
}

#[derive(Default)]
pub struct EstadoArea {
    mods: Modifiers,
    ultimo: Point,
    /// Algum botão desceu aqui dentro: o soltar vale mesmo fora da área.
    apertado: bool,
}

pub struct Area<'a> {
    pub cena: &'a Cena,
    /// As imagens já carregadas, pelo caminho (`None` = não carregou).
    pub imagens: &'a std::collections::HashMap<String, Option<pelicano::Image>>,
    pub arrastados: Option<&'a Cena>,
    pub cursor: Option<&'a Cursor>,
    pub cores: Cores,
    /// O tamanho com que o app pediu o quadro: diferente do da área = avisar.
    pub tamanho: Size,
    pub textos: [String; 2],
}

impl<M> Program<M> for Area<'_>
where
    M: From<EventoArea>,
{
    type State = EstadoArea;

    fn draw(&self, _: &EstadoArea, r: &mut Renderer, b: Rect, _: &Theme, _: Option<Point>) {
        r.fill_quad(b, Some(&pelicano::Background::Color(self.cores.fundo)), Border::default(), None);
        r.push_transform(Transform::translate(b.x, b.y));
        for (principal, p) in &self.cena.grade {
            r.stroke_path(p, &Stroke::new(1.0), if *principal { self.cores.grade_principal } else { self.cores.grade });
        }
        if let Some(p) = &self.cena.pontos {
            r.fill_path(p, self.cores.ponto, FillRule::NonZero);
        }
        for im in &self.cena.imagens {
            let Some(Some(img)) = self.imagens.get(&im.caminho) else { continue };
            let t = img.size();
            // O retângulo da imagem esticado no paralelogramo do motor: (x, y) + u·s + v·t.
            r.push_transform(Transform::from_row(im.u.0 / t.width, im.u.1 / t.width, im.v.0 / t.height, im.v.1 / t.height, im.x, im.y));
            r.draw_image(img, Rect { x: 0.0, y: 0.0, width: t.width, height: t.height }, [0.0; 4]);
            r.pop_transform();
        }
        self.cena.pintar(r);
        if let Some(a) = self.arrastados {
            a.pintar(r);
        }
        if let Some(c) = self.cursor {
            self.pintar_cursor(r, c);
        }
        r.pop_transform();
        if let Some(c) = self.cursor {
            self.caixas(r, b, c);
        }
    }

    fn update(&self, st: &mut EstadoArea, ev: &Event, b: Rect, cursor: Option<Point>, publicar: &mut dyn FnMut(M)) -> Captured {
        if (b.width - self.tamanho.width).abs() > 0.5 || (b.height - self.tamanho.height).abs() > 0.5 {
            publicar(EventoArea::Tamanho(b.size()).into());
        }
        let local = |p: Point| Point::new(p.x - b.x, p.y - b.y);
        let dentro = cursor.is_some_and(|p| b.contains(p));
        match ev {
            Event::ModifiersChanged(m) => st.mods = *m,
            Event::CursorMoved(p) if b.contains(*p) || st.apertado => {
                st.ultimo = local(*p);
                publicar(EventoArea::Mover(st.ultimo).into());
            }
            Event::CursorMoved(_) | Event::CursorLeft => publicar(EventoArea::Saiu.into()),
            Event::MousePressed(botao) if dentro => {
                st.apertado = true;
                publicar(EventoArea::Apertar(*botao, st.ultimo, Point::new(st.ultimo.x + b.x, st.ultimo.y + b.y), st.mods.shift_key()).into());
                return Captured::Yes;
            }
            Event::MouseReleased(botao) if st.apertado => {
                st.apertado = false;
                publicar(EventoArea::Soltar(*botao, st.ultimo).into());
            }
            Event::Wheel { dy, .. } if dentro => {
                publicar(EventoArea::Roda(*dy, st.ultimo).into());
                return Captured::Yes;
            }
            _ => {}
        }
        Captured::No
    }

    fn mouse_interaction(&self, _: &EstadoArea, _: Rect, _: Option<Point>) -> CursorIcon {
        CursorIcon::Crosshair
    }
}

impl Area<'_> {
    fn linha(r: &mut Renderer, s: &Segmento, c: Color, tracejada: bool) {
        let traco = if tracejada { Stroke::new(1.0).dash(vec![5.0, 4.0]) } else { Stroke::new(1.0) };
        r.stroke_path(&Path::line(Point::new(s.x1, s.y1), Point::new(s.x2, s.y2)), &traco, c);
    }

    fn pintar_cursor(&self, r: &mut Renderer, c: &Cursor) {
        let k = &self.cores;
        match c.janela.as_deref() {
            Some(tipo) if !c.previa.is_empty() => {
                let xs = c.previa.iter().flat_map(|s| [s.x1, s.x2]);
                let ys = c.previa.iter().flat_map(|s| [s.y1, s.y2]);
                let (x0, x1) = xs.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
                let (y0, y1) = ys.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
                let cor = if tipo == "dentro" { k.janela_dentro } else { k.janela_toca };
                if let Some(p) = Path::rect(Rect { x: x0 + 0.5, y: y0 + 0.5, width: x1 - x0, height: y1 - y0 }) {
                    r.fill_path(&p, cor.alpha(0.16), FillRule::NonZero);
                    let traco = if tipo == "toca" { Stroke::new(1.0).dash(vec![6.0, 4.0]) } else { Stroke::new(1.0) };
                    r.stroke_path(&p, &traco, cor);
                }
            }
            _ => c.previa.iter().for_each(|s| Self::linha(r, s, k.previa, false)),
        }
        for g in &c.guias {
            Self::linha(r, g, k.guia, true);
        }
        let (x, y, m) = (c.x, c.y, TAMANHO_DA_MIRA);
        let mira = Path::new(|b| {
            b.move_to(Point::new(x - m, y)).line_to(Point::new(x + m, y)).move_to(Point::new(x, y - m)).line_to(Point::new(x, y + m));
        });
        if let Some(p) = mira {
            r.stroke_path(&p, &Stroke::new(1.0), k.cursor);
        }
        if c.marca {
            if let Some(p) = marca_de_snap(c.snap.as_deref(), x, y) {
                r.stroke_path(&p, &Stroke::new(1.5), k.osnap);
            }
        }
    }

    /// As caixinhas da entrada dinâmica (coordenadas; distância e ângulo), ao lado do cursor.
    fn caixas(&self, r: &mut Renderer, b: Rect, c: &Cursor) {
        let mut linhas = vec![format!("{}  {}", c.texto_x, c.texto_y)];
        if let Some(d) = &c.texto_distancia {
            linhas.push(format!("{} {d}  {} {}", self.textos[0], self.textos[1], c.texto_angulo.as_deref().unwrap_or("")));
        }
        for (i, l) in linhas.iter().enumerate() {
            let tam = r.measure(l, 11.0, Font::DEFAULT, None);
            let caixa = Rect { x: b.x + c.x + 16.0, y: b.y + c.y + 16.0 + i as f32 * 22.0, width: tam.width + 10.0, height: tam.height + 2.0 };
            r.fill_quad(caixa, Some(&pelicano::Background::Color(self.cores.caixa)), Border::default().color(self.cores.previa, 1.0), None);
            r.draw_text(l, 11.0, Font::DEFAULT, self.cores.cursor, Point::new(caixa.x + 5.0, caixa.y + 1.0), None);
        }
    }
}

/// A marca de cada tipo de snap, como no AutoCAD: quadrado, triângulo, círculo, losango, X…
fn marca_de_snap(tipo: Option<&str>, x: f32, y: f32) -> Option<Path> {
    let r = 6.0;
    let p = |a: f32, b: f32| Point::new(x + a, y + b);
    Path::new(|c| {
        match tipo {
            Some("meio") => {
                c.move_to(p(-r, r - 1.0)).line_to(p(r, r - 1.0)).line_to(p(0.0, -r)).close();
            }
            Some("centro") => {
                c.circle(p(0.0, 0.0), r);
            }
            Some("quadrante") => {
                c.move_to(p(0.0, -r - 1.0)).line_to(p(r + 1.0, 0.0)).line_to(p(0.0, r + 1.0)).line_to(p(-r - 1.0, 0.0)).close();
            }
            Some("intersecao") => {
                c.move_to(p(-r, -r)).line_to(p(r, r)).move_to(p(r, -r)).line_to(p(-r, r));
            }
            Some("perpendicular") => {
                c.move_to(p(-r, -r)).line_to(p(-r, r)).line_to(p(r, r)).move_to(p(-r, 0.0)).line_to(p(0.0, 0.0)).line_to(p(0.0, r));
            }
            Some("tangente") => {
                c.circle(p(0.0, 0.0), r - 1.0).move_to(p(-r - 2.0, -r - 1.0)).line_to(p(r + 2.0, -r - 1.0));
            }
            Some("no") => {
                c.circle(p(0.0, 0.0), r).move_to(p(-r, -r)).line_to(p(r, r)).move_to(p(r, -r)).line_to(p(-r, r));
            }
            Some("proximo") => {
                c.move_to(p(-r, -r)).line_to(p(r, -r)).line_to(p(-r, r)).line_to(p(r, r)).close();
            }
            _ => {
                c.rect(Rect { x: x - 5.0, y: y - 5.0, width: 10.0, height: 10.0 });
            }
        }
    })
}
