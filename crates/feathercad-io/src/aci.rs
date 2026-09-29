//! O AutoCAD Color Index (ACI, 0–255) ↔ RGB. As cores 1–9 e 250–255 são fixas;
//! de 10 a 249 são 24 matizes (de 15° em 15°) em 10 variações cada, na ordem
//! documentada do DXF (saturação cheia/metade × brilho 100/80/60/50/30%).

const FIXAS: [u32; 10] = [
    0x00_00_00, 0xFF_00_00, 0xFF_FF_00, 0x00_FF_00, 0x00_FF_FF, 0x00_00_FF, 0xFF_00_FF, 0xFF_FF_FF, 0x80_80_80, 0xC0_C0_C0,
];
const CINZAS: [u32; 6] = [0x33_33_33, 0x5B_5B_5B, 0x84_84_84, 0xAD_AD_AD, 0xD6_D6_D6, 0xFF_FF_FF];
const BRILHOS: [f64; 5] = [1.0, 0.8, 0.6, 0.5, 0.3];

/// HSV (h em graus, s e v de 0 a 1) → `0xRRGGBB`.
fn hsv(h: f64, s: f64, v: f64) -> u32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    // Trunca (não arredonda): a tabela oficial tem 0x7F onde o arredondamento daria 0x80.
    let byte = |v: f64| ((v + m) * 255.0 + 1e-9).floor() as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

/// A cor RGB de um índice ACI. O 0 (`PorBloco`) e o 256 (`PorCamada`) não são
/// cores: quem chama trata antes; aqui o 0 dá preto.
pub fn aci_para_rgb(aci: u16) -> u32 {
    match aci {
        0..=9 => FIXAS[aci as usize],
        10..=249 => {
            let i = (aci - 10) as usize;
            let (grupo, k) = (i / 10, i % 10);
            hsv(grupo as f64 * 15.0, if k % 2 == 0 { 1.0 } else { 0.5 }, BRILHOS[k / 2])
        }
        250..=255 => CINZAS[(aci - 250) as usize],
        _ => 0xFF_FF_FF,
    }
}

fn distancia(a: u32, b: u32) -> u32 {
    let canal = |c: u32, s: u32| ((c >> s) & 0xFF) as i32;
    [16, 8, 0].iter().map(|s| (canal(a, *s) - canal(b, *s)).pow(2) as u32).sum()
}

/// O índice ACI mais próximo de uma cor RGB (1–255; o 7 é o branco/preto do fundo).
pub fn rgb_para_aci(rgb: u32) -> u16 {
    (1..=255u16).min_by_key(|i| distancia(rgb, aci_para_rgb(*i))).unwrap_or(7)
}
