//! O desenho como imagem PNG: é como a IA **enxerga**. O traço vem do mesmo motor do aplicativo (blocos,
//! hachuras, textos, tipos de linha); por cima entram o que faz a imagem ser medida: grade com números
//! nas bordas, eixos X/Y, barra de escala e o `#id` de cada entidade (para poder falar dela depois).

use std::collections::{BTreeSet, HashMap};

use feathercad_core::documento::{Documento, Id};
use feathercad_core::geom::Ponto;
use feathercad_core::quadro::montar_com_selecao;
use feathercad_core::vista::Vista;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::fonte::{escrever, Ancora, Fonte, ESQUERDA_BASE};
use crate::json::Res;
use crate::medidas::ponto_de_rotulo;

pub struct Opcoes {
    /// A janela do desenho a mostrar (mínimo, máximo); `None` = tudo.
    pub janela: Option<(Ponto, Ponto)>,
    pub largura_px: u32,
    pub rotulos: bool,
    pub grade: bool,
    pub destacar: BTreeSet<Id>,
    pub rotacao_x: f64,
    pub rotacao_z: f64,
    pub isometria: bool,
}

impl Default for Opcoes {
    fn default() -> Self {
        Self {
            janela: None,
            largura_px: 1000,
            rotulos: true,
            grade: true,
            destacar: BTreeSet::new(),
            rotacao_x: 0.0,
            rotacao_z: 0.0,
            isometria: false,
        }
    }
}

pub struct Imagem {
    pub png: Vec<u8>,
    pub largura: u32,
    pub altura: u32,
    /// O que a imagem realmente mostra do desenho (mínimo, máximo).
    pub janela: (Ponto, Ponto),
    /// Unidades do desenho por pixel.
    pub unidades_por_pixel: f64,
}

/// O que se lê de um quadro (só a parte de desenhar objetos).
struct Lido {
    segmentos: Vec<([f32; 4], u32, f32)>,
    textos: Vec<Texto>,
    preenchimentos: Vec<Preenchimento>,
}

/// Uma área cheia: a cor e os laços (pontos em pixels, x e y alternados).
type Preenchimento = (u32, Vec<Vec<f32>>);

struct Texto {
    x: f32,
    y: f32,
    altura: f32,
    giro: f32,
    h: u8,
    v: u8,
    cor: u32,
    conteudo: String,
}

fn f32_em(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn u32_em(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Lê os bytes `FCQ1` que o motor devolve (o mesmo formato que o TypeScript lê).
fn ler_quadro(b: &[u8]) -> Res<Lido> {
    if b.len() < 16 || &b[..4] != b"FCQ1" {
        return Err("quadro desconhecido".into());
    }
    let n = u32_em(b, 12) as usize;
    let mut segmentos = Vec::with_capacity(n);
    for i in 0..n {
        let o = 16 + i * 24;
        segmentos.push(([f32_em(b, o), f32_em(b, o + 4), f32_em(b, o + 8), f32_em(b, o + 12)], u32_em(b, o + 16), f32_em(b, o + 20)));
    }
    let mut o = 16 + n * 24;
    let nt = u32_em(b, o) as usize;
    o += 4;
    let mut textos = Vec::with_capacity(nt);
    for _ in 0..nt {
        let tam = u16::from_le_bytes([b[o + 22], b[o + 23]]) as usize;
        textos.push(Texto { x: f32_em(b, o), y: f32_em(b, o + 4), altura: f32_em(b, o + 8), giro: f32_em(b, o + 12), h: b[o + 16], v: b[o + 17], cor: u32_em(b, o + 18), conteudo: String::from_utf8_lossy(&b[o + 24..o + 24 + tam]).into_owned() });
        o += 24 + tam;
    }
    let (preenchimentos, _) = ler_preenchimentos(b, o);
    Ok(Lido { segmentos, textos, preenchimentos })
}

fn ler_preenchimentos(b: &[u8], mut o: usize) -> (Vec<Preenchimento>, usize) {
    let n = u32_em(b, o) as usize;
    o += 4;
    let mut fora = Vec::with_capacity(n);
    for _ in 0..n {
        let (cor, nl) = (u32_em(b, o), u32_em(b, o + 4) as usize);
        o += 8;
        let mut lacos = Vec::with_capacity(nl);
        for _ in 0..nl {
            let nv = u32_em(b, o) as usize;
            o += 4;
            lacos.push((0..nv * 2).map(|k| f32_em(b, o + k * 4)).collect());
            o += nv * 8;
        }
        fora.push((cor, lacos));
    }
    (fora, o)
}

fn rgb(cor: u32) -> [u8; 3] {
    [(cor >> 16) as u8, (cor >> 8) as u8, cor as u8]
}

fn tinta(c: [u8; 3], alfa: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c[0], c[1], c[2], alfa));
    p.anti_alias = true;
    p
}

/// O passo "redondo" (1, 2 ou 5 × 10ⁿ) mais perto de `bruto`.
pub fn passo_redondo(bruto: f64) -> f64 {
    let mag = 10f64.powf(bruto.max(1e-12).log10().floor());
    let r = bruto / mag;
    mag * if r < 1.5 { 1.0 } else if r < 3.5 { 2.0 } else if r < 7.5 { 5.0 } else { 10.0 }
}

/// Número para legenda: sem zeros à direita, sem notação científica.
pub fn numero(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" { "0".into() } else { s }
}

/// A janela do desenho: a pedida ou a de todo o desenho, com folga; nunca degenerada.
fn escolher_janela(doc: &Documento, o: &Opcoes) -> (Ponto, Ponto) {
    let (mut min, mut max) = o.janela.or_else(|| doc.caixa()).unwrap_or((Ponto::novo3d(-10.0, -10.0, 0.0), Ponto::novo3d(10.0, 10.0, 0.0)));
    let (mut w, mut h) = (max.x - min.x, max.y - min.y);
    if w < 1e-9 && h < 1e-9 {
        (w, h) = (2.0, 2.0);
        (min, max) = (Ponto::novo3d(min.x - 1.0, min.y - 1.0, min.z), Ponto::novo3d(max.x + 1.0, max.y + 1.0, max.z));
    }
    if o.janela.is_none() {
        let folga = w.max(h) * 0.06;
        return (Ponto::novo3d(min.x - folga, min.y - folga, min.z), Ponto::novo3d(max.x + folga, max.y + folga, max.z));
    }
    (min, max)
}

/// A vista que enquadra a janela em `largura` pixels (a altura segue a proporção 2D ou projeção 3D).
fn montar_vista(janela: (Ponto, Ponto), largura: u32, rx: f64, rz: f64) -> Vista {
    let (w, h) = (janela.1.x - janela.0.x, janela.1.y - janela.0.y);
    let lw = f64::from(largura);
    if rx.abs() > 1e-4 || rz.abs() > 1e-4 {
        let temp_vista = Vista {
            centro: Ponto::novo(0.0, 0.0),
            escala: 1.0,
            largura: lw,
            altura: lw,
            rotacao_x: rx,
            rotacao_z: rz,
        };
        let cantos = [
            Ponto::novo3d(janela.0.x, janela.0.y, janela.0.z),
            Ponto::novo3d(janela.1.x, janela.0.y, janela.0.z),
            Ponto::novo3d(janela.0.x, janela.1.y, janela.0.z),
            Ponto::novo3d(janela.1.x, janela.1.y, janela.0.z),
            Ponto::novo3d(janela.0.x, janela.0.y, janela.1.z),
            Ponto::novo3d(janela.1.x, janela.0.y, janela.1.z),
            Ponto::novo3d(janela.0.x, janela.1.y, janela.1.z),
            Ponto::novo3d(janela.1.x, janela.1.y, janela.1.z),
        ];
        let mut p_min = (f64::INFINITY, f64::INFINITY);
        let mut p_max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for c in cantos {
            let (px, py) = temp_vista.projetar(c);
            p_min.0 = p_min.0.min(px);
            p_min.1 = p_min.1.min(py);
            p_max.0 = p_max.0.max(px);
            p_max.1 = p_max.1.max(py);
        }
        let proj_w = (p_max.0 - p_min.0).max(1e-6);
        let proj_h = (p_max.1 - p_min.1).max(1e-6);
        let lh = (lw * proj_h / proj_w).clamp(300.0, 2400.0);
        let mut v = Vista {
            centro: Ponto::novo((janela.0.x + janela.1.x) / 2.0, (janela.0.y + janela.1.y) / 2.0),
            escala: 1.0,
            largura: lw,
            altura: lh,
            rotacao_x: rx,
            rotacao_z: rz,
        };
        v = v.ajustar(Some(janela));
        v
    } else {
        let lh = (lw * h / w.max(1e-12)).clamp(200.0, 2400.0);
        let escala = (lw / w.max(1e-12)).min(lh / h.max(1e-12));
        Vista {
            centro: Ponto::novo((janela.0.x + janela.1.x) / 2.0, (janela.0.y + janela.1.y) / 2.0),
            escala,
            largura: lw,
            altura: lh,
            rotacao_x: 0.0,
            rotacao_z: 0.0,
        }
    }
}

fn linha(pix: &mut Pixmap, a: (f64, f64), b: (f64, f64), cor: [u8; 3], largura: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.0 as f32, a.1 as f32);
    pb.line_to(b.0 as f32, b.1 as f32);
    if let Some(p) = pb.finish() {
        pix.stroke_path(&p, &tinta(cor, 255), &Stroke { width: largura, ..Stroke::default() }, Transform::identity(), None);
    }
}

/// Grade com números nas bordas e os eixos X e Y (mais escuros) onde cruzam a janela.
fn desenhar_grade(pix: &mut Pixmap, v: &Vista, fonte: &Fonte) {
    let (min, max) = v.area_visivel();
    let passo = passo_redondo((max.x - min.x) / 10.0);
    let salto = (46.0 / (passo * v.escala)).ceil().max(1.0) as i64;
    let (w, h) = (v.largura as f32, v.altura as f32);
    let fim_x = (max.x / passo).floor() as i64;
    for k in (min.x / passo).ceil() as i64..=fim_x {
        let x = k as f64 * passo;
        let px = v.para_tela(Ponto::novo(x, 0.0)).0;
        linha(pix, (px, 0.0), (px, f64::from(h)), if k == 0 { [150, 150, 150] } else { [228, 228, 228] }, 1.0);
        if k % salto == 0 {
            escrever(pix, fonte, &numero(x), px as f32 + 3.0, h - 4.0, 12.0, 0.0, [90, 90, 90], ESQUERDA_BASE, true);
        }
    }
    let fim_y = (max.y / passo).floor() as i64;
    for k in (min.y / passo).ceil() as i64..=fim_y {
        let y = k as f64 * passo;
        let py = v.para_tela(Ponto::novo(0.0, y)).1;
        linha(pix, (0.0, py), (f64::from(w), py), if k == 0 { [150, 150, 150] } else { [228, 228, 228] }, 1.0);
        if k % salto == 0 {
            escrever(pix, fonte, &numero(y), 4.0, py as f32 - 3.0, 12.0, 0.0, [90, 90, 90], ESQUERDA_BASE, true);
        }
    }
    let origem = v.para_tela(Ponto::novo(0.0, 0.0));
    if (0.0..f64::from(w)).contains(&origem.0) && (0.0..f64::from(h)).contains(&origem.1) {
        escrever(pix, fonte, "x", w - 14.0, origem.1 as f32 - 4.0, 12.0, 0.0, [120, 120, 120], ESQUERDA_BASE, true);
        escrever(pix, fonte, "y", origem.0 as f32 + 4.0, 14.0, 12.0, 0.0, [120, 120, 120], ESQUERDA_BASE, true);
    }
}

/// Preenchimentos, segmentos (um caminho por cor e espessura) e textos do quadro.
fn desenhar_objetos(pix: &mut Pixmap, l: &Lido, fonte: &Fonte) {
    for (cor, lacos) in &l.preenchimentos {
        let mut pb = PathBuilder::new();
        for laco in lacos.iter().filter(|l| l.len() >= 6) {
            pb.move_to(laco[0], laco[1]);
            for k in (2..laco.len()).step_by(2) {
                pb.line_to(laco[k], laco[k + 1]);
            }
            pb.close();
        }
        if let Some(p) = pb.finish() {
            pix.fill_path(&p, &tinta(rgb(*cor), 90), FillRule::EvenOdd, Transform::identity(), None);
        }
    }
    let mut grupos: HashMap<(u32, u32), PathBuilder> = HashMap::new();
    for (s, cor, esp) in &l.segmentos {
        let pb = grupos.entry((*cor, esp.to_bits())).or_default();
        pb.move_to(s[0], s[1]);
        pb.line_to(s[2], s[3]);
    }
    for ((cor, esp), pb) in grupos {
        if let Some(p) = pb.finish() {
            pix.stroke_path(&p, &tinta(rgb(cor), 255), &Stroke { width: f32::from_bits(esp).max(1.2), ..Stroke::default() }, Transform::identity(), None);
        }
    }
    for t in &l.textos {
        escrever(pix, fonte, &t.conteudo, t.x, t.y, t.altura, t.giro, rgb(t.cor), Ancora { h: t.h, v: t.v }, false);
    }
}

/// O `#id` de cada entidade visível, ao lado da forma.
fn desenhar_rotulos(pix: &mut Pixmap, doc: &Documento, v: &Vista, fonte: &Fonte) {
    let (min, max) = v.area_visivel();
    for e in doc.entidades().iter().filter(|e| doc.visivel(e)).take(400) {
        let Some(p) = ponto_de_rotulo(&e.geometria) else { continue };
        if p.x < min.x || p.x > max.x || p.y < min.y || p.y > max.y {
            continue;
        }
        let (x, y) = v.para_tela(p);
        escrever(pix, fonte, &format!("#{}", e.id), x as f32 + 4.0, y as f32 - 4.0, 11.0, 0.0, [190, 40, 40], ESQUERDA_BASE, true);
    }
}

/// A barra de escala (canto inferior direito): quanto vale um trecho da imagem.
fn desenhar_escala(pix: &mut Pixmap, v: &Vista, fonte: &Fonte) {
    let (min, max) = v.area_visivel();
    let comprimento = passo_redondo((max.x - min.x) / 5.0);
    let px = comprimento * v.escala;
    let (x1, y) = (v.largura - 16.0, v.altura - 28.0);
    let x0 = x1 - px;
    linha(pix, (x0, y), (x1, y), [20, 20, 20], 2.0);
    linha(pix, (x0, y - 5.0), (x0, y + 5.0), [20, 20, 20], 2.0);
    linha(pix, (x1, y - 5.0), (x1, y + 5.0), [20, 20, 20], 2.0);
    escrever(pix, fonte, &numero(comprimento), ((x0 + x1) / 2.0) as f32, y as f32 - 8.0, 12.0, 0.0, [20, 20, 20], Ancora { h: 1, v: 0 }, true);
}

fn desenhar_indicador_3d(pix: &mut Pixmap, v: &Vista, fonte: &Fonte) {
    let h = v.altura as f32;
    escrever(
        pix,
        fonte,
        &format!("3D [rx={:.0}° rz={:.0}°]", v.rotacao_x, v.rotacao_z),
        14.0,
        22.0,
        13.0,
        0.0,
        [80, 80, 80],
        ESQUERDA_BASE,
        true,
    );

    let ox = 50.0;
    let oy = f64::from(h) - 45.0;
    let comps = [
        (Ponto::novo3d(1.0, 0.0, 0.0), [220, 50, 50], "X"),
        (Ponto::novo3d(0.0, 1.0, 0.0), [40, 160, 40], "Y"),
        (Ponto::novo3d(0.0, 0.0, 1.0), [50, 100, 220], "Z"),
    ];

    for (dir, cor, nome) in comps {
        let (dx, dy) = v.projetar(dir);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let ex = (dx / len) * 28.0;
        let ey = (dy / len) * 28.0;
        let tx = ox + ex;
        let ty = oy - ey;
        linha(pix, (ox, oy), (tx, ty), cor, 2.0);
        escrever(
            pix,
            fonte,
            nome,
            (tx + (dx / len) * 6.0) as f32,
            (ty - (dy / len) * 4.0) as f32 + 4.0,
            11.0,
            0.0,
            cor,
            Ancora { h: 1, v: 2 },
            true,
        );
    }
}

/// Desenha o documento e devolve o PNG e o que ele mostra.
pub fn desenhar(doc: &Documento, o: &Opcoes, fonte: &Fonte) -> Res<Imagem> {
    let janela = escolher_janela(doc, o);
    let (rx, rz) = if o.isometria && o.rotacao_x.abs() < 1e-4 && o.rotacao_z.abs() < 1e-4 {
        (55.0, 45.0)
    } else {
        (o.rotacao_x, o.rotacao_z)
    };
    let eh_3d = rx.abs() > 1e-4 || rz.abs() > 1e-4;
    let vista = montar_vista(janela, o.largura_px.clamp(300, 2400), rx, rz);
    let mut pix = Pixmap::new(vista.largura as u32, vista.altura as u32).ok_or("imagem grande demais")?;
    pix.fill(Color::WHITE);
    if o.grade && !eh_3d {
        desenhar_grade(&mut pix, &vista, fonte);
    }
    let bytes = montar_com_selecao(doc, &vista, None, None, true, &o.destacar, 0);
    desenhar_objetos(&mut pix, &ler_quadro(&bytes)?, fonte);
    if o.rotulos && !eh_3d {
        desenhar_rotulos(&mut pix, doc, &vista, fonte);
    }
    if !eh_3d {
        desenhar_escala(&mut pix, &vista, fonte);
    } else {
        desenhar_indicador_3d(&mut pix, &vista, fonte);
    }
    let png = pix.encode_png().map_err(|e| format!("não consegui gerar o PNG: {e}"))?;
    let (min, max) = vista.area_visivel();
    Ok(Imagem { png, largura: pix.width(), altura: pix.height(), janela: (Ponto::novo(min.x, min.y), Ponto::novo(max.x, max.y)), unidades_por_pixel: 1.0 / vista.escala })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn passo_redondo_e_numero_de_legenda() {
        assert_eq!(passo_redondo(1.2), 1.0);
        assert_eq!(passo_redondo(2.4), 2.0);
        assert_eq!(passo_redondo(4.9), 5.0);
        assert_eq!(passo_redondo(8.0), 10.0);
        assert_eq!(passo_redondo(0.037), 0.05);
        assert_eq!(numero(2.5000), "2.5");
        assert_eq!(numero(-0.00001), "0");
        assert_eq!(numero(100.0), "100");
    }

    #[test]
    fn desenha_um_png_valido_com_a_janela_do_desenho() {
        let mut d = Documento::default();
        d.adicionar_linha(Ponto::novo(0.0, 0.0), Ponto::novo(100.0, 50.0));
        d.adicionar_circulo(Ponto::novo(50.0, 25.0), 10.0);
        let o = Opcoes {
            janela: None,
            largura_px: 600,
            rotulos: true,
            grade: true,
            destacar: BTreeSet::new(),
            rotacao_x: 0.0,
            rotacao_z: 0.0,
            isometria: false,
        };
        let img = desenhar(&d, &o, &crate::fonte::Fonte::Basica).unwrap();
        assert_eq!(&img.png[1..4], b"PNG");
        assert!(img.janela.0.x < 0.0 && img.janela.1.x > 100.0, "a janela tem folga em volta do desenho");
        assert!((img.largura as f64 * img.unidades_por_pixel) >= 100.0);
    }

    #[test]
    fn desenha_png_isometria_3d() {
        let mut d = Documento::default();
        d.adicionar_linha(Ponto::novo3d(0.0, 0.0, 0.0), Ponto::novo3d(100.0, 50.0, 25.0));
        let o = Opcoes {
            janela: None,
            largura_px: 600,
            rotulos: false,
            grade: false,
            destacar: BTreeSet::new(),
            rotacao_x: 55.0,
            rotacao_z: 45.0,
            isometria: true,
        };
        let img = desenhar(&d, &o, &crate::fonte::Fonte::Basica).unwrap();
        assert_eq!(&img.png[1..4], b"PNG");
    }
}
