//! Exportar o desenho em **PDF vetorial**, uma página por folha.
//!
//! O que sai é o mesmo que o app desenha: o `quadro` do motor (blocos, hachuras, tipos de linha, espessuras,
//! textos) é montado com uma vista em que **1 pixel = 1 ponto do PDF**, e o resultado vira caminhos e textos do
//! PDF. Nada de imagem: as linhas continuam vetoriais, e o texto usa a fonte padrão Helvetica (letras gregas e setas saem
//! na fonte padrão Symbol, que o leitor de PDF já traz).
//!
//! Uma **folha** é um retângulo fechado na camada `FOLHA`; cada folha (que não está dentro de outra) vira uma
//! página, na ordem da esquerda para a direita. Sem folhas, sai uma página com o desenho inteiro.
//! No modo papel as cores claras (feitas para o fundo escuro da tela) viram preto, e os preenchimentos escuros
//! clareiam, para o texto continuar legível no papel.

use feathercad_core::documento::{Documento, Geometria};
use feathercad_core::geom::Ponto;
use feathercad_core::quadro;
use feathercad_core::vista::Vista;

/// Pontos do PDF por milímetro.
const PT_POR_MM: f64 = 72.0 / 25.4;
/// 0,7 pt (0,25 mm, a caneta fina padrão de plotagem) para cada pixel de espessura do quadro (1 px = linha fina padrão).
const PT_POR_PX_DE_TRACO: f64 = 0.7;
/// Erro máximo, em pontos, ao transformar arco e círculo em cordas (0,04 pt ≈ 0,014 mm: invisível no papel).
const TOLERANCIA_DO_ARCO_NO_PAPEL_PX: f64 = 0.04;
/// Altura das maiúsculas da Helvetica em relação ao corpo da fonte.
const ALTURA_DA_MAIUSCULA: f64 = 0.718;

/// O tamanho do papel em milímetros (em pé: largura menor que a altura).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Papel {
    pub largura_mm: f64,
    pub altura_mm: f64,
}

impl Papel {
    pub const A4: Papel = Papel { largura_mm: 210.0, altura_mm: 297.0 };
    pub const A3: Papel = Papel { largura_mm: 297.0, altura_mm: 420.0 };
    pub const A2: Papel = Papel { largura_mm: 420.0, altura_mm: 594.0 };
    pub const A1: Papel = Papel { largura_mm: 594.0, altura_mm: 841.0 };
    pub const A0: Papel = Papel { largura_mm: 841.0, altura_mm: 1189.0 };
    /// Acima do A0 (ISO 216: cada um tem o dobro da área do anterior). O PDF aceita até 5080 mm por lado.
    pub const A2A0: Papel = Papel { largura_mm: 1189.0, altura_mm: 1682.0 };
    pub const A4A0: Papel = Papel { largura_mm: 1682.0, altura_mm: 2378.0 };
    pub const A8A0: Papel = Papel { largura_mm: 2378.0, altura_mm: 3364.0 };
    pub const A16A0: Papel = Papel { largura_mm: 3364.0, altura_mm: 4756.0 };
    /// Sem papel fixo: cada página tem o tamanho da folha, 1 unidade do desenho = 1 mm (máximo do PDF: 5 m).
    pub const DA_FOLHA: Papel = Papel { largura_mm: 0.0, altura_mm: 0.0 };

    /// `A4`…`A0`, `2A0`, `4A0`, `8A0`, `16A0` ou `FOLHA` (sem diferenciar maiúscula).
    pub fn de_nome(nome: &str) -> Option<Self> {
        match nome.trim().to_ascii_uppercase().as_str() {
            "A4" => Some(Self::A4),
            "A3" => Some(Self::A3),
            "A2" => Some(Self::A2),
            "A1" => Some(Self::A1),
            "A0" => Some(Self::A0),
            "2A0" => Some(Self::A2A0),
            "4A0" => Some(Self::A4A0),
            "8A0" => Some(Self::A8A0),
            "16A0" => Some(Self::A16A0),
            "FOLHA" => Some(Self::DA_FOLHA),
            _ => None,
        }
    }
}

/// Como o papel fica na página: conforme a folha, ou forçado.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientacao {
    /// Deitado se a folha é mais larga que alta, em pé no resto.
    #[default]
    Automatica,
    Horizontal,
    Vertical,
}

impl Orientacao {
    /// `automatica`, `horizontal` ou `vertical` (sem diferenciar maiúscula).
    pub fn de_nome(nome: &str) -> Option<Self> {
        match nome.trim().to_ascii_lowercase().as_str() {
            "automatica" | "automática" | "auto" => Some(Self::Automatica),
            "horizontal" => Some(Self::Horizontal),
            "vertical" => Some(Self::Vertical),
            _ => None,
        }
    }
}

/// Como exportar.
#[derive(Clone, Copy, Debug)]
pub struct Opcoes {
    pub papel: Papel,
    pub orientacao: Orientacao,
    /// Cores para o papel (claras → preto; preenchimentos escuros → claros). `false` = as cores do desenho.
    pub modo_papel: bool,
}

impl Default for Opcoes {
    fn default() -> Self {
        Self { papel: Papel::A3, orientacao: Orientacao::Automatica, modo_papel: true }
    }
}

type Caixa = (Ponto, Ponto);

/// As folhas do desenho: as inseridas pelo botão (blocos `FOLHA-n`) e os retângulos fechados da camada `FOLHA` que não estão
/// dentro de outro, da esquerda para a direita.
/// Sem nenhuma, uma folha só, com o desenho inteiro.
pub fn folhas(doc: &Documento) -> Vec<Caixa> {
    let mut retangulos: Vec<Caixa> = doc
        .entidades()
        .iter()
        .filter(|e| doc.camadas().get(e.camada).is_some_and(|c| c.nome.eq_ignore_ascii_case("FOLHA")))
        .filter_map(|e| match &e.geometria {
            Geometria::Polilinha { vertices, fechada: true } if vertices.len() == 4 => Some(caixa_de(vertices.iter().map(|v| v.p))),
            _ => None,
        })
        .collect();
    retangulos.extend(doc.folhas_inseridas().iter().map(|f| (f.min, f.max)));
    let copia = retangulos.clone();
    retangulos.retain(|(a, b)| !copia.iter().any(|(c, d)| (c, d) != (a, b) && c.x <= a.x && c.y <= a.y && d.x >= b.x && d.y >= b.y && (d.x - c.x) * (d.y - c.y) > (b.x - a.x) * (b.y - a.y)));
    retangulos.sort_by(|(a, _), (b, _)| a.x.total_cmp(&b.x).then(b.y.total_cmp(&a.y)));
    if retangulos.is_empty() {
        retangulos.extend(caixa_do_desenho(doc));
    }
    retangulos
}

fn caixa_de(pontos: impl Iterator<Item = Ponto>) -> Caixa {
    let mut min = Ponto::novo(f64::INFINITY, f64::INFINITY);
    let mut max = Ponto::novo(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pontos {
        min = Ponto::novo(min.x.min(p.x), min.y.min(p.y));
        max = Ponto::novo(max.x.max(p.x), max.y.max(p.y));
    }
    (min, max)
}

/// A caixa do desenho todo, com 2 % de folga (`None` se está vazio).
fn caixa_do_desenho(doc: &Documento) -> Option<Caixa> {
    let mut pts: Vec<Ponto> = Vec::new();
    for e in doc.entidades() {
        match &e.geometria {
            Geometria::Linha { a, b } => pts.extend([*a, *b]),
            Geometria::Polilinha { vertices, .. } => pts.extend(vertices.iter().map(|v| v.p)),
            Geometria::Circulo { centro, raio } | Geometria::Arco { centro, raio, .. } => pts.extend([Ponto::novo(centro.x - raio, centro.y - raio), Ponto::novo(centro.x + raio, centro.y + raio)]),
            Geometria::Texto { ponto, altura, conteudo, .. } => pts.extend([*ponto, Ponto::novo(ponto.x + altura * 0.6 * conteudo.chars().count() as f64, ponto.y + altura)]),
            Geometria::Ponto { p } => pts.push(*p),
            Geometria::Insercao { ponto, .. } => pts.push(*ponto),
            Geometria::Hachura { contornos, .. } => pts.extend(contornos.iter().flatten().map(|v| v.p)),
        }
    }
    if pts.is_empty() {
        return None;
    }
    let (min, max) = caixa_de(pts.into_iter());
    let folga = ((max.x - min.x).max(max.y - min.y) * 0.02).max(1e-6);
    Some((Ponto::novo(min.x - folga, min.y - folga), Ponto::novo(max.x + folga, max.y + folga)))
}

// ---------------------------------------------------------------- ler o quadro

struct Segmento {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    cor: u32,
    espessura: f64,
}

struct Texto {
    x: f64,
    y: f64,
    altura: f64,
    giro: f64,
    h: u8,
    v: u8,
    cor: u32,
    conteudo: String,
}

struct Preenchimento {
    cor: u32,
    lacos: Vec<Vec<(f64, f64)>>,
}

#[derive(Default)]
struct Quadro {
    segmentos: Vec<Segmento>,
    textos: Vec<Texto>,
    preenchimentos: Vec<Preenchimento>,
}

struct Leitor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Leitor<'_> {
    fn pegar(&mut self, n: usize) -> Result<&[u8], String> {
        let fim = self.i.checked_add(n).filter(|f| *f <= self.b.len()).ok_or("quadro truncado")?;
        let r = &self.b[self.i..fim];
        self.i = fim;
        Ok(r)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.pegar(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.pegar(2)?.try_into().map_err(|_| "quadro truncado")?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.pegar(4)?.try_into().map_err(|_| "quadro truncado")?))
    }
    fn f32(&mut self) -> Result<f64, String> {
        Ok(f64::from(f32::from_le_bytes(self.pegar(4)?.try_into().map_err(|_| "quadro truncado")?)))
    }
}

/// Lê um quadro `FCQ1` (o fundo, se houver, é ignorado).
fn ler_quadro(bytes: &[u8]) -> Result<Quadro, String> {
    let mut l = Leitor { b: bytes, i: 0 };
    if l.pegar(4)? != quadro::MAGIA_NOVO {
        return Err("quadro inesperado".into());
    }
    l.pegar(8)?; // versão
    let mut q = Quadro::default();
    for _ in 0..l.u32()? {
        let (x1, y1, x2, y2) = (l.f32()?, l.f32()?, l.f32()?, l.f32()?);
        let cor = l.u32()?;
        let espessura = l.f32()?;
        q.segmentos.push(Segmento { x1, y1, x2, y2, cor, espessura });
    }
    for _ in 0..l.u32()? {
        let (x, y, altura, giro) = (l.f32()?, l.f32()?, l.f32()?, l.f32()?);
        let (h, v) = (l.u8()?, l.u8()?);
        let cor = l.u32()?;
        let n = usize::from(l.u16()?);
        let conteudo = String::from_utf8_lossy(l.pegar(n)?).into_owned();
        q.textos.push(Texto { x, y, altura, giro, h, v, cor, conteudo });
    }
    for _ in 0..l.u32()? {
        let cor = l.u32()?;
        let mut lacos = Vec::new();
        for _ in 0..l.u32()? {
            let nv = l.u32()?;
            let mut laco = Vec::with_capacity(nv as usize);
            for _ in 0..nv {
                laco.push((l.f32()?, l.f32()?));
            }
            lacos.push(laco);
        }
        q.preenchimentos.push(Preenchimento { cor, lacos });
    }
    Ok(q)
}

// ---------------------------------------------------------------- cores e texto

fn canais(cor: u32) -> (f64, f64, f64) {
    (f64::from((cor >> 16) & 0xFF) / 255.0, f64::from((cor >> 8) & 0xFF) / 255.0, f64::from(cor & 0xFF) / 255.0)
}

fn luminancia((r, g, b): (f64, f64, f64)) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// A cor de um traço ou texto. No modo papel, o que foi feito para o fundo escuro da tela precisa ficar legível no branco:
/// cinza claro (texto secundário, guias) vira preto; cor viva e clara (amarelo, ciano) escurece mantendo o tom.
fn cor_do_traco(cor: u32, papel: bool) -> (f64, f64, f64) {
    let c = canais(cor);
    if !papel {
        return c;
    }
    let l = luminancia(c);
    let saturacao = c.0.max(c.1).max(c.2) - c.0.min(c.1).min(c.2);
    if saturacao < 0.15 {
        return if l > 0.7 { (0.0, 0.0, 0.0) } else { c };
    }
    if l > 0.55 {
        let f = 0.45 / l;
        (c.0 * f, c.1 * f, c.2 * f)
    } else {
        c
    }
}

/// A cor de um preenchimento: no modo papel, o claro vira preto (símbolo cheio) e o escuro clareia (fundo de peça).
fn cor_do_preenchimento(cor: u32, papel: bool) -> (f64, f64, f64) {
    let c = canais(cor);
    if !papel {
        return c;
    }
    let l = luminancia(c);
    if l > 0.8 {
        (0.0, 0.0, 0.0)
    } else if l < 0.5 {
        let t = ((0.85 - l) / (1.0 - l)).clamp(0.0, 1.0);
        (c.0 + (1.0 - c.0) * t, c.1 + (1.0 - c.1) * t, c.2 + (1.0 - c.2) * t)
    } else {
        c
    }
}

/// Larguras da Helvetica (por mil do corpo) para ASCII de 32 a 126.
const LARGURAS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667,
    556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722,
    500, 500, 500, 334, 260, 334, 584,
];

/// O byte do caractere na codificação WinAnsi (`?` se não existe nela).
fn winansi(c: char) -> u8 {
    match c {
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        '\u{2022}' => 0x95,
        '\u{2026}' => 0x85,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '\u{20AC}' => 0x80,
        c if (c as u32) < 256 => c as u32 as u8,
        _ => b'?',
    }
}

/// Largura do texto em relação ao corpo da fonte (1.0 = o corpo).
fn largura_do_texto(bytes: &[u8]) -> f64 {
    bytes.iter().map(|&b| if (32..=126).contains(&b) { f64::from(LARGURAS[usize::from(b) - 32]) } else if b == 0xD8 { 778.0 } else { 556.0 }).sum::<f64>() / 1000.0
}

/// O byte da letra grega ou seta na fonte Symbol (`None` = não é dela).
fn simbolo(c: char) -> Option<u8> {
    Some(match c {
        '\u{0394}' => b'D',
        '\u{0398}' => b'Q',
        '\u{039B}' => b'L',
        '\u{03A0}' => b'P',
        '\u{03A3}' => b'S',
        '\u{03A6}' => b'F',
        '\u{03A9}' => b'W',
        '\u{03B1}' => b'a',
        '\u{03B2}' => b'b',
        '\u{03B3}' => b'g',
        '\u{03B4}' => b'd',
        '\u{03B5}' => b'e',
        '\u{03B8}' => b'q',
        '\u{03BB}' => b'l',
        '\u{03BC}' => b'm',
        '\u{03C0}' => b'p',
        '\u{03C1}' => b'r',
        '\u{03C3}' => b's',
        '\u{03C4}' => b't',
        '\u{03C6}' => b'f',
        '\u{03C9}' => b'w',
        '\u{2190}' => 0xAC,
        '\u{2191}' => 0xAD,
        '\u{2192}' => 0xAE,
        '\u{2193}' => 0xAF,
        '\u{2264}' => 0xA3,
        '\u{2265}' => 0xB3,
        '\u{2248}' => 0xBB,
        '\u{2260}' => 0xB9,
        '\u{221E}' => 0xA5,
        _ => return None,
    })
}

/// Um pedaço de texto todo na mesma fonte: Helvetica (WinAnsi) ou Symbol.
struct Trecho {
    simbolo: bool,
    bytes: Vec<u8>,
}

/// Divide o texto em pedaços de Helvetica e de Symbol.
fn trechos(texto: &str) -> Vec<Trecho> {
    let mut v: Vec<Trecho> = Vec::new();
    for c in texto.chars() {
        let (e_simbolo, b) = simbolo(c).map_or_else(|| (false, winansi(c)), |b| (true, b));
        match v.last_mut() {
            Some(t) if t.simbolo == e_simbolo => t.bytes.push(b),
            _ => v.push(Trecho { simbolo: e_simbolo, bytes: vec![b] }),
        }
    }
    v
}

/// Largura de um pedaço em relação ao corpo (Symbol: 0,6 por letra; setas 1,0).
fn largura_do_trecho(t: &Trecho) -> f64 {
    if t.simbolo {
        t.bytes.iter().map(|&b| if (0xAC..=0xAF).contains(&b) { 1.0 } else { 0.6 }).sum()
    } else {
        largura_do_texto(&t.bytes)
    }
}

fn escapar(bytes: &[u8], saida: &mut Vec<u8>) {
    for &b in bytes {
        if matches!(b, b'(' | b')' | b'\\') {
            saida.push(b'\\');
        }
        saida.push(b);
    }
}

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

// ---------------------------------------------------------------- uma página

/// A espessura do traço em pontos. A linha fina (1 px no quadro) sai como fio de cabelo (`0 w`): o leitor a desenha com 1 pixel
/// da tela em qualquer zoom, como o próprio FeatherCAD, e os detalhes não se fundem quando se aproxima. Só a linha grossa tem largura fixa.
fn largura_do_traco(espessura_px: f64) -> f64 {
    if espessura_px <= 1.0 {
        0.0
    } else {
        espessura_px * PT_POR_PX_DE_TRACO
    }
}

/// O conteúdo de uma página (`altura_pt`: para virar o eixo Y).
fn conteudo(q: &Quadro, altura_pt: f64, papel: bool) -> Vec<u8> {
    let mut o: Vec<u8> = Vec::new();
    let y = |v: f64| altura_pt - v;
    for p in &q.preenchimentos {
        let (r, g, b) = cor_do_preenchimento(p.cor, papel);
        o.extend(format!("{} {} {} rg\n", n(r), n(g), n(b)).bytes());
        for laco in p.lacos.iter().filter(|l| l.len() >= 3) {
            for (i, (px, py)) in laco.iter().enumerate() {
                o.extend(format!("{} {} {}\n", n(*px), n(y(*py)), if i == 0 { "m" } else { "l" }).bytes());
            }
            o.extend(b"h\n");
        }
        o.extend(b"f*\n");
    }
    o.extend(b"1 J 1 j\n");
    let mut atual: Option<(u32, i64)> = None;
    let mut aberto = false;
    for s in &q.segmentos {
        let chave = (s.cor, (s.espessura * 100.0).round() as i64);
        if atual != Some(chave) {
            if aberto {
                o.extend(b"S\n");
            }
            let (r, g, b) = cor_do_traco(s.cor, papel);
            o.extend(format!("{} {} {} RG {} w\n", n(r), n(g), n(b), n(largura_do_traco(s.espessura))).bytes());
            atual = Some(chave);
        }
        o.extend(format!("{} {} m {} {} l\n", n(s.x1), n(y(s.y1)), n(s.x2), n(y(s.y2))).bytes());
        aberto = true;
    }
    if aberto {
        o.extend(b"S\n");
    }
    for t in &q.textos {
        escrever_texto(t, altura_pt, papel, &mut o);
    }
    o
}

fn escrever_texto(t: &Texto, altura_pt: f64, papel: bool, o: &mut Vec<u8>) {
    let partes = trechos(&t.conteudo);
    let corpo = t.altura / ALTURA_DA_MAIUSCULA;
    let largura = partes.iter().map(largura_do_trecho).sum::<f64>() * corpo;
    let angulo = -t.giro; // o quadro guarda o giro no sentido do canvas (Y para baixo)
    let (seno, cosseno) = angulo.sin_cos();
    let deslocamento_h = match t.h {
        1 => -largura / 2.0,
        2 => -largura,
        _ => 0.0,
    };
    let deslocamento_v = match t.v {
        1 => t.altura * 0.2,
        2 => -t.altura / 2.0,
        3 => -t.altura,
        _ => 0.0,
    };
    let (x, y) = (t.x, altura_pt - t.y);
    let px = x + cosseno * deslocamento_h - seno * deslocamento_v;
    let py = y + seno * deslocamento_h + cosseno * deslocamento_v;
    let (r, g, b) = cor_do_traco(t.cor, papel);
    o.extend(format!("BT {} {} {} rg {} {} {} {} {} {} Tm\n", n(r), n(g), n(b), n(cosseno), n(seno), n(-seno), n(cosseno), n(px), n(py)).bytes());
    if partes.is_empty() {
        o.extend(format!("/F1 {} Tf () Tj\n", n(corpo)).bytes());
    }
    for p in &partes {
        o.extend(format!("/{} {} Tf (", if p.simbolo { "F2" } else { "F1" }, n(corpo)).bytes());
        escapar(&p.bytes, o);
        o.extend(b") Tj\n");
    }
    o.extend(b"ET\n");
}

/// Uma página: o quadro da `janela` cabendo no papel (em pé ou deitado, conforme a janela).
fn pagina(doc: &Documento, janela: Caixa, op: &Opcoes) -> Result<(f64, f64, Vec<u8>), String> {
    let (largura_u, altura_u) = (janela.1.x - janela.0.x, janela.1.y - janela.0.y);
    if largura_u <= 0.0 || altura_u <= 0.0 {
        return Err("folha sem tamanho".into());
    }
    if op.papel == Papel::DA_FOLHA {
        let f = (14400.0 / PT_POR_MM / largura_u.max(altura_u)).min(1.0);
        let (w, h) = (largura_u * f * PT_POR_MM, altura_u * f * PT_POR_MM);
        let vista = Vista { centro: Ponto::novo((janela.0.x + janela.1.x) / 2.0, (janela.0.y + janela.1.y) / 2.0), escala: w / largura_u, largura: w, altura: h };
        let bytes = quadro::montar_fino(doc, &vista, true, TOLERANCIA_DO_ARCO_NO_PAPEL_PX);
        return Ok((w, h, conteudo(&ler_quadro(&bytes)?, h, op.modo_papel)));
    }
    let (menor, maior) = (op.papel.largura_mm.min(op.papel.altura_mm) * PT_POR_MM, op.papel.largura_mm.max(op.papel.altura_mm) * PT_POR_MM);
    let deitado = match op.orientacao {
        Orientacao::Automatica => largura_u >= altura_u,
        Orientacao::Horizontal => true,
        Orientacao::Vertical => false,
    };
    let (w, h) = if deitado { (maior, menor) } else { (menor, maior) };
    let escala = (w / largura_u).min(h / altura_u);
    let vista = Vista { centro: Ponto::novo((janela.0.x + janela.1.x) / 2.0, (janela.0.y + janela.1.y) / 2.0), escala, largura: w, altura: h };
    let bytes = quadro::montar_fino(doc, &vista, true, TOLERANCIA_DO_ARCO_NO_PAPEL_PX);
    Ok((w, h, conteudo(&ler_quadro(&bytes)?, h, op.modo_papel)))
}

/// O PDF com uma página por janela de `paginas` (em unidades do desenho).
pub fn exportar(doc: &Documento, paginas: &[Caixa], op: &Opcoes) -> Result<Vec<u8>, String> {
    if paginas.is_empty() {
        return Err("não há o que exportar: o desenho está vazio".into());
    }
    let mut corpos = Vec::with_capacity(paginas.len());
    for p in paginas {
        corpos.push(pagina(doc, *p, op)?);
    }
    let mut pdf: Vec<u8> = Vec::new();
    let mut posicoes: Vec<usize> = Vec::new();
    pdf.extend(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut objeto = |pdf: &mut Vec<u8>, id: usize, corpo: &[u8]| {
        if posicoes.len() < id {
            posicoes.resize(id, 0);
        }
        posicoes[id - 1] = pdf.len();
        pdf.extend(format!("{id} 0 obj\n").bytes());
        pdf.extend(corpo);
        pdf.extend(b"\nendobj\n");
    };
    objeto(&mut pdf, 1, b"<< /Type /Catalog /Pages 2 0 R >>");
    let filhas: Vec<String> = (0..corpos.len()).map(|i| format!("{} 0 R", 5 + 2 * i)).collect();
    objeto(&mut pdf, 2, format!("<< /Type /Pages /Kids [{}] /Count {} >>", filhas.join(" "), corpos.len()).as_bytes());
    objeto(&mut pdf, 3, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    objeto(&mut pdf, 4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>");
    for (i, (w, h, c)) in corpos.iter().enumerate() {
        let (pag, cont) = (5 + 2 * i, 6 + 2 * i);
        objeto(&mut pdf, pag, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {cont} 0 R >>", n(*w), n(*h)).as_bytes());
        let mut fluxo = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        fluxo.extend(c);
        fluxo.extend(b"\nendstream");
        objeto(&mut pdf, cont, &fluxo);
    }
    let inicio_xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", posicoes.len() + 1).bytes());
    for p in &posicoes {
        pdf.extend(format!("{p:010} 00000 n \n").bytes());
    }
    pdf.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{inicio_xref}\n%%EOF\n", posicoes.len() + 1).bytes());
    Ok(pdf)
}

/// O PDF com uma página por folha do desenho (ver [`folhas`]).
pub fn exportar_folhas(doc: &Documento, op: &Opcoes) -> Result<Vec<u8>, String> {
    exportar(doc, &folhas(doc), op)
}

/// O PDF só com as folhas de `indices` (posições na lista de [`folhas`], na ordem dada); `None` = todas.
pub fn exportar_folhas_escolhidas(doc: &Documento, indices: Option<&[usize]>, op: &Opcoes) -> Result<Vec<u8>, String> {
    let todas = folhas(doc);
    let Some(indices) = indices else { return exportar(doc, &todas, op) };
    let escolhidas: Vec<Caixa> = indices.iter().map(|&i| todas.get(i).copied().ok_or_else(|| format!("a folha {} não existe", i + 1))).collect::<Result<_, _>>()?;
    if escolhidas.is_empty() {
        return Err("nenhuma folha escolhida".into());
    }
    exportar(doc, &escolhidas, op)
}

/// As folhas em texto para a interface: largura e altura em unidades do desenho.
pub fn descrever_folhas(doc: &Documento) -> Vec<(f64, f64)> {
    folhas(doc).iter().map(|(a, b)| (b.x - a.x, b.y - a.y)).collect()
}

/// Traços a mais que isto numa miniatura são pulados (1 a cada tantos): o desenho fica reconhecível e o SVG leve.
const MAX_TRACOS_NA_MINIATURA: usize = 20_000;

/// Uma miniatura em SVG (`largura` × `altura` px, fundo escuro como a tela) da primeira folha do desenho, ou do desenho todo
/// se não há folha. Só traços e preenchimentos; texto não entra (ilegível nesse tamanho). `None` se o desenho está vazio.
pub fn miniatura_svg(doc: &Documento, largura: f64, altura: f64) -> Option<String> {
    let janela = *folhas(doc).first()?;
    let (largura_u, altura_u) = (janela.1.x - janela.0.x, janela.1.y - janela.0.y);
    if largura_u <= 0.0 || altura_u <= 0.0 {
        return None;
    }
    let margem = 0.94;
    let escala = (largura / largura_u).min(altura / altura_u) * margem;
    let vista = Vista { centro: Ponto::novo((janela.0.x + janela.1.x) / 2.0, (janela.0.y + janela.1.y) / 2.0), escala, largura, altura };
    let q = ler_quadro(&quadro::montar_fino(doc, &vista, false, 0.3)).ok()?;
    let cor = |c: u32| format!("#{:06X}", c & 0xFF_FFFF);
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {largura} {altura}\" preserveAspectRatio=\"xMidYMid meet\"><rect width=\"100%\" height=\"100%\" fill=\"#1F252C\"/>");
    for p in &q.preenchimentos {
        let mut d = String::new();
        for laco in p.lacos.iter().filter(|l| l.len() >= 3) {
            for (i, (x, y)) in laco.iter().enumerate() {
                d.push_str(&format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x, y));
            }
            d.push('Z');
        }
        svg.push_str(&format!("<path d=\"{d}\" fill=\"{}\" fill-opacity=\"0.5\" fill-rule=\"evenodd\"/>", cor(p.cor)));
    }
    let passo = q.segmentos.len().div_ceil(MAX_TRACOS_NA_MINIATURA).max(1);
    let mut por_cor: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
    for s in q.segmentos.iter().step_by(passo) {
        por_cor.entry(s.cor).or_default().push_str(&format!("M{:.1} {:.1}L{:.1} {:.1}", s.x1, s.y1, s.x2, s.y2));
    }
    for (c, d) in por_cor {
        svg.push_str(&format!("<path d=\"{d}\" stroke=\"{}\" stroke-width=\"0.8\" fill=\"none\" stroke-linecap=\"round\"/>", cor(c)));
    }
    svg.push_str("</svg>");
    Some(svg)
}
