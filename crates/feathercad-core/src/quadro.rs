//! O quadro: o que está na tela, já pronto para desenhar, em binário.
//!
//! ```text
//! "FCQ0"                                  nada mudou (a versão que a interface tem ainda vale)
//! "FCQ1" u64 versão u32 n  n × Segmento   desenho novo
//!        u32 nt  nt × Texto                       os textos
//!        u32 np  np × Preenchimento               áreas cheias (hachura sólida), desenhadas por baixo
//!        u32 nx u32 ny  nx × Marca  ny × Marca   (o fundo: linhas/pontos da grade)
//! Segmento = f32 x1, y1, x2, y2 (pixels) · u32 cor 0xRRGGBB · f32 espessura
//! Texto    = f32 x, y (pixels, no ponto de alinhamento) · f32 altura (pixels) ·
//!            f32 giro (radianos, já no sentido do canvas) · u8 alinha H (0 esq, 1 centro, 2 dir) ·
//!            u8 alinha V (0 base, 1 baixo, 2 meio, 3 topo) · u32 cor · u16 n · n bytes UTF-8
//! Preenchimento = u32 cor 0xRRGGBB · u32 nl · nl × (u32 nv · nv × (f32 x, y) pixels); par-ímpar
//! Marca    = f32 posição (pixels) · u8 principal
//!        u32 ni  ni × Imagem                      imagens raster, desenhadas por baixo de tudo
//! Imagem   = f32 x, y (pixels, canto de cima à esquerda) · f32 ux, uy (largura inteira, pixels) ·
//!            f32 vx, vy (altura inteira, de cima para baixo, pixels) · u16 n · n bytes UTF-8 (caminho do arquivo)
//!
//! Polilinha: cada segmento vira um `Segmento`; os arcos (bulge) já vêm em cordas que
//! não se afastam mais que meio pixel da curva.
//! ```
//! Tudo little-endian. Coordenadas já em pixels da tela; só entra o que cabe
//! na janela e cuja camada está ligada; a cor já é a final (`PorCamada` resolvido).

use std::collections::BTreeSet;

use crate::documento::{cantos_da_imagem, AlinhaH, AlinhaV, Documento, Geometria, Id, LinhaDePadrao, Propriedades, TipoLinha, PROFUNDIDADE_MAXIMA};
use crate::fundo::{marcas, Marca};
use crate::geom::{amostrar_arco, segmento_com_bulge, varredura_graus, Ponto, Transf};
use crate::vista::Vista;

pub const MAGIA_IGUAL: &[u8; 4] = b"FCQ0";
pub const MAGIA_NOVO: &[u8; 4] = b"FCQ1";
/// Bytes por segmento no quadro.
pub const BYTES_POR_SEGMENTO: usize = 24;
/// Bytes por marca do fundo.
pub const BYTES_POR_MARCA: usize = 5;
const ESPESSURA: f32 = 1.0;
/// Quanto a corda de um arco pode se afastar da curva, em pixels.
const TOLERANCIA_DO_ARCO_PX: f64 = 0.5;
/// Texto menor que isto (em pixels de altura) não vale desenhar: é só um borrão.
const ALTURA_MINIMA_DO_TEXTO_PX: f64 = 2.5;
/// Um texto maior que isto é cortado ao gravar no quadro.
const TEXTO_MAX_BYTES: usize = 4096;

/// Os dois segmentos tocam a janela? (caixa envolvente — recorte grosso, barato)
fn toca(area: (Ponto, Ponto), a: Ponto, b: Ponto) -> bool {
    let (min, max) = area;
    a.x.max(b.x) >= min.x && a.x.min(b.x) <= max.x && a.y.max(b.y) >= min.y && a.y.min(b.y) <= max.y
}

fn escrever_f32(saida: &mut Vec<u8>, v: f64) {
    saida.extend_from_slice(&(v as f32).to_le_bytes());
}

fn escrever_marcas(saida: &mut Vec<u8>, ms: &[Marca]) {
    for m in ms {
        saida.extend_from_slice(&m.pos.to_le_bytes());
        saida.push(u8::from(m.principal));
    }
}

/// A cor como aparece na tela: o "branco" do CAD (ACI 7) é a cor que **contrasta**
/// com o fundo, então vira preto no fundo claro, e o preto vira branco no escuro
/// (senão sumiria). As demais cores ficam como estão.
pub fn cor_na_tela(cor: u32, fundo_claro: bool) -> u32 {
    match (cor, fundo_claro) {
        (0xFF_FF_FF, true) => 0x00_00_00,
        (0x00_00_00, false) => 0xFF_FF_FF,
        _ => cor,
    }
}

/// Sem recorte: a área é o plano todo (para a prévia, que sempre se desenha inteira).
const PLANO_TODO: (Ponto, Ponto) = (Ponto { x: -1e300, y: -1e300, z: 0.0 }, Ponto { x: 1e300, y: 1e300, z: 0.0 });

/// Os traços de uma forma, em pixels da tela, sem recortar pela janela (a prévia do que está sendo desenhado).
pub fn tracos(geo: &Geometria, vista: &Vista) -> Vec<((f64, f64), (f64, f64))> {
    let mut pedacos = Vec::new();
    segmentos_da(geo, PLANO_TODO, TOLERANCIA_DO_ARCO_PX / vista.escala, &mut pedacos);
    pedacos.iter().map(|(a, b)| (vista.para_tela(*a), vista.para_tela(*b))).collect()
}

/// Uma linha quebrada em segmentos de reta: os pares de pontos consecutivos.
/// As cordas que tocam a janela: uma polilinha (ou um círculo) grande que atravessa a tela só entrega os trechos visíveis.
fn cordas(pontos: &[Ponto], area: (Ponto, Ponto), fora: &mut Vec<(Ponto, Ponto)>) {
    fora.extend(pontos.windows(2).filter(|w| toca(area, w[0], w[1])).map(|w| (w[0], w[1])));
}

/// Os segmentos (em unidades do desenho) que uma entidade desenha, se a caixa dela toca a janela.
fn segmentos_da(geo: &Geometria, area: (Ponto, Ponto), tolerancia: f64, fora: &mut Vec<(Ponto, Ponto)>) {
    match geo {
        Geometria::Circulo { centro, raio } | Geometria::Arco { centro, raio, .. } => {
            let (min, max) = (Ponto::novo(centro.x - raio, centro.y - raio), Ponto::novo(centro.x + raio, centro.y + raio));
            if !toca(area, min, max) {
                return;
            }
            let (inicio, varredura) = match geo {
                Geometria::Arco { inicio, fim, .. } => (inicio.to_radians(), varredura_graus(*inicio, *fim).to_radians()),
                _ => (0.0, std::f64::consts::TAU),
            };
            cordas(&amostrar_arco(*centro, *raio, inicio, varredura, tolerancia), area, fora);
        }
        Geometria::Linha { a, b } => {
            if toca(area, *a, *b) {
                fora.push((*a, *b));
            }
        }
        Geometria::Imagem { origem, u, v, .. } => {
            let c = cantos_da_imagem(*origem, *u, *v);
            fora.extend((0..4).map(|i| (c[i], c[(i + 1) % 4])).filter(|(a, b)| toca(area, *a, *b)));
        }
        Geometria::Face3D { p1, p2, p3, p4 } => {
            if toca(area, *p1, *p2) { fora.push((*p1, *p2)); }
            if toca(area, *p2, *p3) { fora.push((*p2, *p3)); }
            if toca(area, *p3, *p4) { fora.push((*p3, *p4)); }
            if toca(area, *p4, *p1) { fora.push((*p4, *p1)); }
        }
        Geometria::Polilinha { vertices, fechada } => {
            let (mut min, mut max) = (vertices[0].p, vertices[0].p);
            for v in vertices {
                min = Ponto::novo(min.x.min(v.p.x), min.y.min(v.p.y));
                max = Ponto::novo(max.x.max(v.p.x), max.y.max(v.p.y));
            }
            // A caixa dos vértices não cobre a barriga de um arco: folga de metade da maior corda.
            let folga = vertices.windows(2).map(|w| w[0].p.distancia(w[1].p)).fold(0.0, f64::max) / 2.0;
            if !toca(area, Ponto::novo(min.x - folga, min.y - folga), Ponto::novo(max.x + folga, max.y + folga)) {
                return;
            }
            let n = vertices.len();
            let trechos = if *fechada { n } else { n - 1 };
            for i in 0..trechos {
                let (de, ate) = (&vertices[i], &vertices[(i + 1) % n]);
                let pts = segmento_com_bulge(de.p, ate.p, de.bulge, tolerancia);
                cordas(&pts, area, fora);
            }
        }
        Geometria::Texto { .. } | Geometria::Insercao { .. } | Geometria::Hachura { .. } | Geometria::Ponto { .. } => {}
    }
}

fn codigo_h(h: AlinhaH) -> u8 {
    match h {
        AlinhaH::Esquerda => 0,
        AlinhaH::Centro => 1,
        AlinhaH::Direita => 2,
    }
}

fn codigo_v(v: AlinhaV) -> u8 {
    match v {
        AlinhaV::Base => 0,
        AlinhaV::Baixo => 1,
        AlinhaV::Meio => 2,
        AlinhaV::Topo => 3,
    }
}

/// Um texto no quadro, se cabe na janela e é grande o bastante para ler.
fn escrever_texto(saida: &mut Vec<u8>, geo: &Geometria, cor: u32, vista: &Vista, area: (Ponto, Ponto)) -> bool {
    let Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } = geo else { return false };
    let altura_px = altura * vista.escala;
    let alcance = altura * crate::documento::LARGURA_DO_CARACTERE * conteudo.chars().count() as f64;
    let (min, max) = (Ponto::novo(area.0.x - alcance, area.0.y - alcance), Ponto::novo(area.1.x + alcance, area.1.y + alcance));
    if altura_px < ALTURA_MINIMA_DO_TEXTO_PX || !toca((min, max), *ponto, *ponto) {
        return false;
    }
    let (x, y) = vista.para_tela(*ponto);
    for n in [x, y, altura_px, -rotacao.to_radians()] {
        escrever_f32(saida, n);
    }
    saida.push(codigo_h(*h));
    saida.push(codigo_v(*v));
    saida.extend_from_slice(&cor.to_le_bytes());
    let mut bytes = conteudo.as_bytes();
    if bytes.len() > TEXTO_MAX_BYTES {
        let mut fim = TEXTO_MAX_BYTES;
        while !conteudo.is_char_boundary(fim) {
            fim -= 1;
        }
        bytes = &conteudo.as_bytes()[..fim];
    }
    saida.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    saida.extend_from_slice(bytes);
    true
}

/// Um hash barato (multiplicar e girar) para a grade de pixels do LOD: a chave é só três inteiros e o `SipHash` do
/// `std` custava mais que o resto da conta em 300 mil traços.
#[derive(Default)]
struct HashRapido(u64);

impl std::hash::Hasher for HashRapido {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(*b)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    fn write_i32(&mut self, i: i32) {
        self.0 = (self.0.rotate_left(5) ^ u64::from(i as u32)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = (self.0.rotate_left(5) ^ u64::from(i)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

/// Os pixels que já têm um traço miúdo de certa cor (LOD).
type PixelsOcupados = std::collections::HashSet<(i32, i32, u32), std::hash::BuildHasherDefault<HashRapido>>;

/// O que se acumula ao montar o quadro.
#[derive(Default)]
struct Saida {
    corpo: Vec<u8>,
    n: u32,
    textos: Vec<u8>,
    nt: u32,
    preenchimentos: Vec<u8>,
    np: u32,
    imagens: Vec<u8>,
    ni: u32,
    /// LOD da tela: um traço que cabe num pixel só é escrito uma vez por pixel e cor (senão um mapa denso, com o
    /// desenho todo na tela, manda centenas de milhares de riscos que se sobrepõem no mesmo pixel).
    lod: bool,
    ocupados: PixelsOcupados,
}

/// O que não muda ao descer pelas entidades e pelos blocos.
struct Contexto<'a> {
    doc: &'a Documento,
    vista: &'a Vista,
    area: (Ponto, Ponto),
    tolerancia: f64,
    fundo_claro: bool,
    /// Tela: descarta o que não se enxerga (`sem_traco_invisivel`). Impressão (PDF) não: o leitor dá zoom.
    simplificar: bool,
}

/// A cor de um objeto selecionado.
const COR_DA_SELECAO: u32 = 0x3B_82_F6;

fn escrever_segmento(saida: &mut Saida, vista: &Vista, a: Ponto, b: Ponto, cor: u32) {
    escrever_px(saida, (vista.para_tela(a), vista.para_tela(b)), cor, ESPESSURA);
}

/// A caixa dos quatro cantos de `(min, max)` depois de transformada.
fn caixa_transformada(caixa: (Ponto, Ponto), t: &Transf) -> (Ponto, Ponto) {
    let (min, max) = caixa;
    let cantos = [min, Ponto::novo(max.x, min.y), max, Ponto::novo(min.x, max.y)].map(|c| t.aplicar(c));
    let menor = cantos.iter().fold(cantos[0], |m, c| Ponto::novo(m.x.min(c.x), m.y.min(c.y)));
    let maior = cantos.iter().fold(cantos[0], |m, c| Ponto::novo(m.x.max(c.x), m.y.max(c.y)));
    (menor, maior)
}

/// A entidade a desenhar: camada, cor própria (`None` = da camada) e forma.
struct Peca<'a> {
    camada: usize,
    cor: Option<u32>,
    geo: &'a Geometria,
    /// Está selecionada (ou dentro de um bloco selecionado): desenha na cor da seleção.
    destaque: bool,
    /// Espessura e tipo de linha da entidade.
    props: &'a Propriedades,
}

/// Pixels de espessura para cada 0,25 mm de espessura de linha (0,25 mm = 1 px), entre 1 e 8 px.
fn espessura_em_px(espessura_mm: Option<f32>) -> f32 {
    espessura_mm.map_or(ESPESSURA, |mm| (mm / 0.25).clamp(1.0, 8.0))
}

/// A cor `cor` misturada com o fundo em `percentual` % (transparência da camada: 0 = a cor, 90 = quase o fundo).
fn misturar_com_o_fundo(cor: u32, fundo_claro: bool, percentual: u8) -> u32 {
    if percentual == 0 {
        return cor;
    }
    let fundo: u32 = if fundo_claro { 0xFF_FF_FF } else { 0x21_28_30 };
    let t = f64::from(percentual.min(90)) / 100.0;
    let canal = |c: u32, f: u32, deslocamento: u32| {
        let (a, b) = (f64::from((c >> deslocamento) & 0xFF), f64::from((f >> deslocamento) & 0xFF));
        (a * (1.0 - t) + b * t).round() as u32
    };
    (canal(cor, fundo, 16) << 16) | (canal(cor, fundo, 8) << 8) | canal(cor, fundo, 0)
}

/// Escreve traços já em pixels, com o tipo de linha: o traço e o vão correm ao longo da sequência (contínua) de
/// segmentos, sem reiniciar em cada junta. Traço longo demais para o padrão vira contínuo.
fn escrever_tracos(saida: &mut Saida, tracos: &[TracoPx], cor: u32, espessura: f32, tipo: TipoLinha) {
    let padrao = tipo.padrao_px();
    let total: f64 = tracos.iter().map(|((x1, y1), (x2, y2))| (x2 - x1).hypot(y2 - y1)).sum();
    let periodo: f64 = padrao.iter().sum();
    if padrao.is_empty() || total / periodo.max(1.0) > MAXIMO_DE_TRACOS_POR_FORMA {
        for t in tracos {
            escrever_px(saida, *t, cor, espessura);
        }
        return;
    }
    let (mut idx, mut resto) = (0usize, padrao[0]);
    for ((x1, y1), (x2, y2)) in tracos {
        let comprimento = (x2 - x1).hypot(y2 - y1);
        let mut pos = 0.0;
        while pos < comprimento - 1e-9 {
            let pedaco = resto.min(comprimento - pos);
            if idx % 2 == 0 {
                let (a, b) = (pos / comprimento, (pos + pedaco) / comprimento);
                escrever_px(saida, ((x1 + (x2 - x1) * a, y1 + (y2 - y1) * a), (x1 + (x2 - x1) * b, y1 + (y2 - y1) * b)), cor, espessura);
            }
            pos += pedaco;
            resto -= pedaco;
            if resto <= 1e-9 {
                idx = (idx + 1) % padrao.len();
                resto = padrao[idx];
            }
        }
    }
}

/// Um segmento em pixels da tela: `((x1, y1), (x2, y2))`.
type TracoPx = ((f64, f64), (f64, f64));

/// Uma forma com mais traços que isto fica contínua (proteção de tempo e de tamanho do quadro).
const MAXIMO_DE_TRACOS_POR_FORMA: f64 = 4000.0;

fn escrever_px(saida: &mut Saida, ((x1, y1), (x2, y2)): TracoPx, cor: u32, espessura: f32) {
    // LOD: traço que cabe num pixel já desenhado (mesma cor) não muda nada na tela.
    if saida.lod && (x2 - x1).abs() < 1.0 && (y2 - y1).abs() < 1.0 && !saida.ocupados.insert((x1.round() as i32, y1.round() as i32, cor)) {
        return;
    }
    for v in [x1, y1, x2, y2] {
        escrever_f32(&mut saida.corpo, v);
    }
    saida.corpo.extend_from_slice(&cor.to_le_bytes());
    saida.corpo.extend_from_slice(&espessura.to_le_bytes());
    saida.n += 1;
}

/// Onde estamos ao descer nos blocos: a transformação composta, a cor do `INSERT` que contém e a profundidade.
struct Nivel {
    t: Transf,
    herdada: Option<u32>,
    profundidade: u8,
    /// Dentro de um bloco selecionado.
    destaque: bool,
}

/// LOD: bloco menor que isto (em pixels na tela) é desenhado como um ponto.
const BLOCO_MINIMO_PX: f64 = 2.0;

/// Desenha um bloco inserido: recorta pela caixa dele e desce em cada entidade, sob a transformação composta.
fn desenhar_insercao(ctx: &Contexto<'_>, geo: &Geometria, nivel: &Nivel, cor: u32, destaque: bool, saida: &mut Saida) {
    let Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao } = geo else { return };
    let (Some(def), true) = (ctx.doc.blocos().get(*bloco), nivel.profundidade < PROFUNDIDADE_MAXIMA) else { return };
    let filha = nivel.t.depois_de(&Transf::de_insercao(def.base, *ponto, *escala_x, *escala_y, *rotacao));
    if let Some(caixa) = ctx.doc.caixa_guardada_do_bloco(*bloco) {
        let (min, max) = caixa_transformada(caixa, &filha);
        if !toca(ctx.area, min, max) {
            return;
        }
        // LOD: um bloco que na tela mede menos que `BLOCO_MINIMO_PX` vira um ponto (as árvores de um mapa inteiro).
        if ctx.simplificar && (max.x - min.x).max(max.y - min.y) * ctx.vista.escala < BLOCO_MINIMO_PX {
            let cor_do_ponto = if destaque { COR_DA_SELECAO } else { cor_na_tela(cor, ctx.fundo_claro) };
            let (px, py) = ctx.vista.para_tela(Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0));
            escrever_px(saida, ((px, py), (px + 0.5, py)), cor_do_ponto, ESPESSURA);
            return;
        }
    }
    let dentro = Nivel { t: filha, herdada: Some(cor), profundidade: nivel.profundidade + 1, destaque };
    for n in &def.entidades {
        desenhar(ctx, &Peca { camada: n.camada, cor: n.cor, geo: &n.geometria, destaque: false, props: &n.props }, &dentro, saida);
    }
}

/// Um texto sob a transformação do bloco onde está (posição, altura e giro seguem o bloco).
fn desenhar_texto(ctx: &Contexto<'_>, geo: &Geometria, t: &Transf, cor: u32, saida: &mut Saida) {
    let Geometria::Texto { ponto, altura, rotacao, conteudo, h, v } = geo else { return };
    let movido = Geometria::Texto { ponto: t.aplicar(*ponto), altura: altura * t.escala_media(), rotacao: rotacao + t.rotacao_graus(), conteudo: conteudo.clone(), h: *h, v: *v };
    saida.nt += u32::from(escrever_texto(&mut saida.textos, &movido, cor, ctx.vista, ctx.area));
}

/// Meia-largura da cruz que marca um `POINT`, em pixels.
const MEIA_MARCA_DO_PONTO_PX: f64 = 3.0;
/// Menos que isto de espaço entre linhas do padrão (em pixels) e a hachura vira preenchimento sólido.
const ESPACO_MINIMO_DO_PADRAO_PX: f64 = 3.0;
/// Mais linhas que isto numa família e a hachura vira preenchimento sólido (proteção de tempo).
const LINHAS_MAXIMAS_DO_PADRAO: f64 = 4000.0;

/// Os laços da hachura como pontos (arcos por bulge viram cordas), no espaço do desenho.
pub fn lacos_da_hachura(contornos: &[Vec<crate::geom::Vertice>], tolerancia: f64) -> Vec<Vec<Ponto>> {
    contornos
        .iter()
        .map(|c| {
            let n = c.len();
            let mut pts = Vec::new();
            for i in 0..n {
                let (de, ate) = (&c[i], &c[(i + 1) % n]);
                let trecho = segmento_com_bulge(de.p, ate.p, de.bulge, tolerancia);
                pts.extend(trecho.iter().take(trecho.len().saturating_sub(1)));
            }
            pts
        })
        .filter(|l| l.len() >= 3)
        .collect()
}

/// Um preenchimento sólido (par-ímpar) dos `lacos`, já em pixels.
fn preencher(saida: &mut Saida, ctx: &Contexto<'_>, lacos: &[Vec<Ponto>], t: &Transf, cor: u32) {
    saida.preenchimentos.extend_from_slice(&cor.to_le_bytes());
    saida.preenchimentos.extend_from_slice(&(lacos.len() as u32).to_le_bytes());
    for laco in lacos {
        saida.preenchimentos.extend_from_slice(&(laco.len() as u32).to_le_bytes());
        for p in laco {
            let (x, y) = ctx.vista.para_tela(t.aplicar(*p));
            escrever_f32(&mut saida.preenchimentos, x);
            escrever_f32(&mut saida.preenchimentos, y);
        }
    }
    saida.np += 1;
}

/// Uma família com mais traços que isto (por causa de pontos e vãos) não se desenha: vira preenchimento.
const TRACOS_MAXIMOS_DE_UMA_FAMILIA: f64 = 60000.0;

/// Corta o trecho `x0..x1` (ao longo da linha) pelo padrão de traço e vão, com o padrão começando em `fase`.
/// Ponto (`0`) vira um risco curto (12 % do espaço entre as linhas). `false` se o padrão é degenerado ou gera demais.
fn cortar_em_tracos(x0: f64, x1: f64, tracos: &[f64], fase: f64, passo: f64, saida: &mut impl FnMut(f64, f64)) -> bool {
    let periodo: f64 = tracos.iter().map(|t| t.abs()).sum();
    if periodo.is_nan() || periodo <= 1e-9 || (x1 - x0) / periodo > TRACOS_MAXIMOS_DE_UMA_FAMILIA {
        return false;
    }
    let ponto = 0.12 * passo;
    let mut s = fase + ((x0 - fase) / periodo).floor() * periodo;
    while s < x1 {
        for t in tracos {
            let (ini, fim) = (s, s + if *t == 0.0 { ponto } else { t.abs() });
            if *t >= 0.0 && fim > x0 && ini < x1 {
                saida(ini.max(x0), fim.min(x1));
            }
            s += t.abs();
            if s >= x1 {
                break;
            }
        }
    }
    true
}

/// Espaço perpendicular entre as linhas de uma família.
fn passo_do_padrao(l: &LinhaDePadrao) -> f64 {
    let (seno, cosseno) = l.angulo.to_radians().sin_cos();
    (-l.dx * seno + l.dy * cosseno).abs()
}

struct Plano3D {
    p0: Ponto,
    e1: (f64, f64, f64),
    e2: (f64, f64, f64),
}

impl Plano3D {
    fn de_lacos(lacos: &[Vec<Ponto>]) -> Self {
        let Some(p0) = lacos.first().and_then(|l| l.first()).copied() else {
            return Self { p0: Ponto::novo(0.0, 0.0), e1: (1.0, 0.0, 0.0), e2: (0.0, 1.0, 0.0) };
        };
        let p1 = lacos.iter().flatten().find(|p| p.distancia(p0) > 1e-6).copied().unwrap_or(Ponto::novo3d(p0.x + 1.0, p0.y, p0.z));
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let dz = p1.z - p0.z;

        let mut nx = 0.0;
        let mut ny = 0.0;
        let mut nz = 0.0;
        for laco in lacos {
            let n = laco.len();
            for i in 0..n {
                let (a, b) = (laco[i], laco[(i + 1) % n]);
                nx += (a.y - b.y) * (a.z + b.z);
                ny += (a.z - b.z) * (a.x + b.x);
                nz += (a.x - b.x) * (a.y + b.y);
            }
        }
        let len_n = (nx * nx + ny * ny + nz * nz).sqrt();
        let n_unit = if len_n > 1e-9 {
            (nx / len_n, ny / len_n, nz / len_n)
        } else {
            (0.0, 0.0, 1.0)
        };

        // Projetar o vetor p1 - p0 sobre o plano perpendicular a n_unit para formar e1
        let dot = dx * n_unit.0 + dy * n_unit.1 + dz * n_unit.2;
        let v1_x = dx - dot * n_unit.0;
        let v1_y = dy - dot * n_unit.1;
        let v1_z = dz - dot * n_unit.2;
        let len_v1 = (v1_x * v1_x + v1_y * v1_y + v1_z * v1_z).sqrt();

        let e1 = if len_v1 > 1e-9 {
            (v1_x / len_v1, v1_y / len_v1, v1_z / len_v1)
        } else {
            let ax = n_unit.0.abs();
            let ay = n_unit.1.abs();
            let az = n_unit.2.abs();
            let arb = if ax <= ay && ax <= az {
                (1.0, 0.0, 0.0)
            } else if ay <= az {
                (0.0, 1.0, 0.0)
            } else {
                (0.0, 0.0, 1.0)
            };
            let cx = arb.1 * n_unit.2 - arb.2 * n_unit.1;
            let cy = arb.2 * n_unit.0 - arb.0 * n_unit.2;
            let cz = arb.0 * n_unit.1 - arb.1 * n_unit.0;
            let len_c = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-12);
            (cx / len_c, cy / len_c, cz / len_c)
        };

        let e2_x = n_unit.1 * e1.2 - n_unit.2 * e1.1;
        let e2_y = n_unit.2 * e1.0 - n_unit.0 * e1.2;
        let e2_z = n_unit.0 * e1.1 - n_unit.1 * e1.0;
        let len2 = (e2_x * e2_x + e2_y * e2_y + e2_z * e2_z).sqrt().max(1e-12);
        let e2 = (e2_x / len2, e2_y / len2, e2_z / len2);

        Self { p0, e1, e2 }
    }

    fn para_2d(&self, p: Ponto) -> (f64, f64) {
        let (dx, dy, dz) = (p.x - self.p0.x, p.y - self.p0.y, p.z - self.p0.z);
        (dx * self.e1.0 + dy * self.e1.1 + dz * self.e1.2, dx * self.e2.0 + dy * self.e2.1 + dz * self.e2.2)
    }

    fn para_3d(&self, u: f64, v: f64) -> Ponto {
        Ponto::novo3d(
            self.p0.x + u * self.e1.0 + v * self.e2.0,
            self.p0.y + u * self.e1.1 + v * self.e2.1,
            self.p0.z + u * self.e1.2 + v * self.e2.2,
        )
    }
}

/// Os trechos (em unidades do desenho) em que uma família de linhas paralelas cruza a região dos `lacos`.
/// `None` se a família tem linhas demais para o `espaco_max` dado.
fn linhas_do_padrao(lacos: &[Vec<Ponto>], l: &LinhaDePadrao, maximo: f64) -> Option<Vec<(Ponto, Ponto)>> {
    let plano3d = Plano3D::de_lacos(lacos);
    let (seno, cosseno) = l.angulo.to_radians().sin_cos();
    let (u, v) = ((cosseno, seno), (-seno, cosseno));
    let de_padrao = |p: Ponto| {
        let (px, py) = plano3d.para_2d(p);
        let (bx, by) = plano3d.para_2d(l.base);
        ((px - bx) * u.0 + (py - by) * u.1, (px - bx) * v.0 + (py - by) * v.1)
    };
    // Só a parte do deslocamento perpendicular à linha separa uma linha da outra.
    let passo = (-l.dx * seno + l.dy * cosseno).abs();
    if passo < 1e-12 {
        return Some(Vec::new());
    }
    let pts: Vec<Vec<(f64, f64)>> = lacos.iter().map(|laco| laco.iter().map(|p| de_padrao(*p)).collect()).collect();
    let (menor, maior) = pts.iter().flatten().fold((f64::MAX, f64::MIN), |(a, b), (_, y)| (a.min(*y), b.max(*y)));
    let (k0, k1) = ((menor / passo).ceil(), (maior / passo).floor());
    if k1 - k0 > maximo {
        return None;
    }
    let mut fora = Vec::new();
    let mut k = k0;
    while k <= k1 {
        let c = k * passo;
        let mut xs = Vec::new();
        for laco in &pts {
            for i in 0..laco.len() {
                let ((x0, y0), (x1, y1)) = (laco[i], laco[(i + 1) % laco.len()]);
                // Regra meio-aberta: um vértice exatamente na linha conta uma vez só.
                if (y0 <= c && c < y1) || (y1 <= c && c < y0) {
                    xs.push(x0 + (c - y0) / (y1 - y0) * (x1 - x0));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        let (bx, by) = plano3d.para_2d(l.base);
        for par in xs.chunks_exact(2) {
            let ponto = |x: f64| {
                let u_2d = bx + x * u.0 + c * v.0;
                let v_2d = by + x * u.1 + c * v.1;
                plano3d.para_3d(u_2d, v_2d)
            };
            if l.tracos.is_empty() {
                fora.push((ponto(par[0]), ponto(par[1])));
            } else {
                // A fase da linha `k` anda `k · (deslocamento ao longo da linha)`, como no `.pat` do AutoCAD.
                let fase = k * (l.dx * cosseno + l.dy * seno);
                if !cortar_em_tracos(par[0], par[1], &l.tracos, fase, passo, &mut |a, b| fora.push((ponto(a), ponto(b)))) || fora.len() as f64 > TRACOS_MAXIMOS_DE_UMA_FAMILIA {
                    return None;
                }
            }
        }
        k += 1.0;
    }
    Some(fora)
}

/// Mais linhas que isto numa hachura e ela não é decomposta em traços (para gravar em formato sem hachura).
const TRACOS_MAXIMOS_AO_DECOMPOR: f64 = 20000.0;

/// A hachura de padrão como traços soltos (em unidades do desenho); `None` se são traços demais.
pub fn tracos_do_padrao(lacos: &[Vec<Ponto>], padrao: &[LinhaDePadrao]) -> Option<Vec<(Ponto, Ponto)>> {
    let mut todos = Vec::new();
    for l in padrao {
        todos.append(&mut linhas_do_padrao(lacos, l, TRACOS_MAXIMOS_AO_DECOMPOR)?);
    }
    (todos.len() as f64 <= TRACOS_MAXIMOS_AO_DECOMPOR).then_some(todos)
}

/// Hachura: sólida vira preenchimento; de padrão vira linhas paralelas recortadas pelo contorno
/// (e, se ficarem coladas demais na tela, vira preenchimento — é o que se enxergaria).
fn desenhar_hachura(ctx: &Contexto<'_>, contornos: &[Vec<crate::geom::Vertice>], padrao: &[LinhaDePadrao], t: &Transf, cor: u32, saida: &mut Saida) {
    let lacos = lacos_da_hachura(contornos, ctx.tolerancia / t.escala_maxima().max(1e-12));
    let Some(primeiro) = lacos.first().and_then(|l| l.first()) else { return };
    let (mut min, mut max) = (t.aplicar(*primeiro), t.aplicar(*primeiro));
    for p in lacos.iter().flatten() {
        let q = t.aplicar(*p);
        min = Ponto::novo(min.x.min(q.x), min.y.min(q.y));
        max = Ponto::novo(max.x.max(q.x), max.y.max(q.y));
    }
    if !toca(ctx.area, min, max) {
        return;
    }
    let escala_tela = ctx.vista.escala * t.escala_media();
    let mut tracos = Vec::new();
    let denso = padrao.iter().any(|l| passo_do_padrao(l) * escala_tela < ESPACO_MINIMO_DO_PADRAO_PX);
    if !padrao.is_empty() && !denso {
        for l in padrao {
            match linhas_do_padrao(&lacos, l, LINHAS_MAXIMAS_DO_PADRAO) {
                Some(mut v) => tracos.append(&mut v),
                None => {
                    tracos.clear();
                    break;
                }
            }
        }
    }
    if padrao.is_empty() || denso {
        preencher(saida, ctx, &lacos, t, cor);
    } else {
        for (a, b) in tracos {
            escrever_segmento(saida, ctx.vista, t.aplicar(a), t.aplicar(b), cor);
        }
    }
}

/// Trecho menor que isto (em pixels ao quadrado) não se enxerga: com o desenho todo na tela, o lado curto de cada dormente
/// (0,24 m) são milhares de riscos de meio pixel. Se a forma inteira é miúda, fica como está (não some).
const TRACO_MINIMO_PX2: f64 = 0.12;

fn sem_traco_invisivel(tracos: &mut Vec<TracoPx>) {
    let comprido = |((x1, y1), (x2, y2)): &TracoPx| {
        let (dx, dy) = (*x2 - *x1, *y2 - *y1);
        dx * dx + dy * dy >= TRACO_MINIMO_PX2
    };
    if tracos.len() > 1 && tracos.iter().any(comprido) {
        tracos.retain(comprido);
    }
}

/// LOD: o passo mínimo entre pontos de uma linha quebrada na tela (em pixels ao quadrado): pontos mais juntos que
/// isto (0,8 px) são absorvidos no traço seguinte. Uma curva de mil cordas com o desenho todo na tela vira dezenas.
const PASSO_MINIMO_PX2: f64 = 0.64;

/// Decima uma sequência de segmentos: a cadeia que se emenda (o fim de um é o começo do próximo) só emite um ponto
/// novo quando ele está a mais de `PASSO_MINIMO_PX2` do último emitido. Se a forma toda é miúda, sobra um traço
/// (ela não some). Sequências que não se emendam (recortadas pela janela) recomeçam a cadeia.
fn decimar(tracos: &mut Vec<TracoPx>) {
    if tracos.len() < 2 {
        return;
    }
    let junto = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6;
    let longe = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2) >= PASSO_MINIMO_PX2;
    let mut fora: Vec<TracoPx> = Vec::with_capacity(tracos.len() / 2 + 1);
    let mut inicio = tracos[0].0;
    let mut ultimo = inicio;
    for (de, ate) in tracos.iter().copied() {
        if !junto(de, ultimo) {
            if !junto(ultimo, inicio) {
                fora.push((inicio, ultimo));
            }
            inicio = de;
        }
        ultimo = ate;
        if longe(inicio, ultimo) {
            fora.push((inicio, ultimo));
            inicio = ultimo;
        }
    }
    if !junto(ultimo, inicio) {
        fora.push((inicio, ultimo));
    }
    if !fora.is_empty() {
        *tracos = fora;
    }
}

/// Desenha uma entidade (do desenho ou de dentro de um bloco).
fn desenhar(ctx: &Contexto<'_>, peca: &Peca<'_>, nivel: &Nivel, saida: &mut Saida) {
    let Some(c) = ctx.doc.camadas().get(peca.camada) else { return };
    if !c.visivel() {
        return;
    }
    // A camada `0` é especial: dentro de um bloco ela adota a cor do `INSERT` (regra do DXF).
    let cor = peca.cor.or_else(|| if c.nome == "0" { nivel.herdada } else { None }).unwrap_or(c.cor);
    let destaque = peca.destaque || nivel.destaque;
    let cor_na_tela = if destaque { COR_DA_SELECAO } else { misturar_com_o_fundo(cor_na_tela(cor, ctx.fundo_claro), ctx.fundo_claro, c.transparencia) };
    let t = &nivel.t;
    match peca.geo {
        Geometria::Insercao { .. } => desenhar_insercao(ctx, peca.geo, nivel, cor, destaque, saida),
        Geometria::Texto { .. } => desenhar_texto(ctx, peca.geo, t, cor_na_tela, saida),
        Geometria::Ponto { p } => {
            // Cruz de 3 px de meia-largura, qualquer que seja o zoom.
            let (c, d) = (t.aplicar(*p), ctx.vista.pixels_em_unidades(MEIA_MARCA_DO_PONTO_PX));
            if toca(ctx.area, c, c) || !t.eh_identidade() {
                escrever_segmento(saida, ctx.vista, Ponto::novo(c.x - d, c.y), Ponto::novo(c.x + d, c.y), cor_na_tela);
                escrever_segmento(saida, ctx.vista, Ponto::novo(c.x, c.y - d), Ponto::novo(c.x, c.y + d), cor_na_tela);
            }
        }
        Geometria::Hachura { contornos, padrao, .. } => desenhar_hachura(ctx, contornos, padrao, t, cor_na_tela, saida),
        Geometria::Imagem { origem, u, v, arquivo } => {
            let c = cantos_da_imagem(*origem, *u, *v).map(|p| t.aplicar(p));
            let (min, max) = c.iter().fold((c[0], c[0]), |(a, b), p| (Ponto::novo(a.x.min(p.x), a.y.min(p.y)), Ponto::novo(b.x.max(p.x), b.y.max(p.y))));
            if !toca(ctx.area, min, max) {
                return;
            }
            // c[3] = canto de cima à esquerda; largura até c[2], altura (para baixo) até c[0]
            let [base, _, cima_dir, cima_esq] = c.map(|p| ctx.vista.para_tela(p));
            for v in [cima_esq.0, cima_esq.1, cima_dir.0 - cima_esq.0, cima_dir.1 - cima_esq.1, base.0 - cima_esq.0, base.1 - cima_esq.1] {
                escrever_f32(&mut saida.imagens, v);
            }
            let bytes = arquivo.as_bytes();
            let n = bytes.len().min(usize::from(u16::MAX));
            saida.imagens.extend_from_slice(&(n as u16).to_le_bytes());
            saida.imagens.extend_from_slice(&bytes[..n]);
            saida.ni += 1;
            if destaque {
                for i in 0..4 {
                    escrever_segmento(saida, ctx.vista, c[i], c[(i + 1) % 4], cor_na_tela);
                }
            }
        }
        Geometria::Face3D { p1, p2, p3, p4 } => {
            escrever_segmento(saida, ctx.vista, t.aplicar(*p1), t.aplicar(*p2), cor_na_tela);
            escrever_segmento(saida, ctx.vista, t.aplicar(*p2), t.aplicar(*p3), cor_na_tela);
            escrever_segmento(saida, ctx.vista, t.aplicar(*p3), t.aplicar(*p4), cor_na_tela);
            escrever_segmento(saida, ctx.vista, t.aplicar(*p4), t.aplicar(*p1), cor_na_tela);
        }
        geo => {
            let (area, tolerancia) = if t.eh_identidade() { (ctx.area, ctx.tolerancia) } else { (PLANO_TODO, ctx.tolerancia / t.escala_maxima().max(1e-12)) };
            let mut pedacos = Vec::new();
            segmentos_da(geo, area, tolerancia, &mut pedacos);
            let mut em_pixels: Vec<_> = pedacos.iter().map(|(a, b)| (ctx.vista.para_tela(t.aplicar(*a)), ctx.vista.para_tela(t.aplicar(*b)))).collect();
            if ctx.simplificar {
                sem_traco_invisivel(&mut em_pixels);
                decimar(&mut em_pixels);
            }
            escrever_tracos(saida, &em_pixels, cor_na_tela, espessura_em_px(peca.props.espessura.or(c.espessura)), peca.props.tipo_linha.unwrap_or(c.tipo_linha));
        }
    }
}

/// Monta o quadro da `vista`. Se `versao_conhecida` é a versão atual, devolve só
/// `FCQ0`. `fundo` é o passo base da grade (`None` = sem fundo); `fundo_claro`
/// diz se o tema é claro (ver `cor_na_tela`).
pub fn montar(doc: &Documento, vista: &Vista, versao_conhecida: Option<u64>, fundo: Option<f64>, fundo_claro: bool) -> Vec<u8> {
    montar_com_selecao(doc, vista, versao_conhecida, fundo, fundo_claro, &BTreeSet::new(), 0)
}

/// Como `montar`, para impressão: arcos e círculos com `tolerancia_px` de erro (a tela usa 0,5 px; o papel pede bem menos,
fn area_de_corte(vista: &Vista) -> (Ponto, Ponto) {
    if vista.rotacao_x.abs() > 1e-4 || vista.rotacao_z.abs() > 1e-4 {
        PLANO_TODO
    } else {
        vista.area_visivel()
    }
}

/// Como `montar`, para impressão: arcos e círculos com `tolerancia_px` de erro (a tela usa 0,5 px; o papel pede bem menos,
/// senão um círculo pequeno sai como um octógono). Sempre é um quadro novo, sem fundo.
pub fn montar_fino(doc: &Documento, vista: &Vista, fundo_claro: bool, tolerancia_px: f64) -> Vec<u8> {
    montar_fino_filtrado(doc, vista, fundo_claro, tolerancia_px, None)
}

/// Como `montar_fino`, só com as entidades da `camada` (um bloco vai inteiro com a camada da sua inserção):
/// o PDF desenha camada por camada para o leitor poder ligar e desligar cada uma.
pub fn montar_fino_da_camada(doc: &Documento, vista: &Vista, fundo_claro: bool, tolerancia_px: f64, camada: usize) -> Vec<u8> {
    montar_fino_filtrado(doc, vista, fundo_claro, tolerancia_px, Some(camada))
}

fn montar_fino_filtrado(doc: &Documento, vista: &Vista, fundo_claro: bool, tolerancia_px: f64, camada: Option<usize>) -> Vec<u8> {
    let ctx = Contexto { doc, vista, area: area_de_corte(vista), tolerancia: tolerancia_px / vista.escala, fundo_claro, simplificar: false };
    let mut dentro = Saida::default();
    let raiz = Nivel { t: Transf::IDENTIDADE, herdada: None, profundidade: 0, destaque: false };
    for e in doc.entidades().iter().filter(|e| camada.is_none() || camada == Some(e.camada)) {
        desenhar(&ctx, &Peca { camada: e.camada, cor: e.cor, geo: &e.geometria, destaque: false, props: &e.props }, &raiz, &mut dentro);
    }
    escrever_quadro(0, &dentro, vista, None)
}

/// Como `montar`, com os objetos `selecao` na cor de seleção. `revisao` é o contador da seleção: entra
/// na versão do quadro (`versão do desenho + revisão × 2³²`), porque mudar só a seleção também refaz o quadro.
pub fn montar_com_selecao(doc: &Documento, vista: &Vista, versao_conhecida: Option<u64>, fundo: Option<f64>, fundo_claro: bool, selecao: &BTreeSet<Id>, revisao: u64) -> Vec<u8> {
    montar_arrastando(doc, vista, versao_conhecida, fundo, fundo_claro, selecao, revisao, false)
}

/// Como `montar_com_selecao`; com `ocultar_selecao` os objetos selecionados **não** entram (durante o arrasto
/// eles são desenhados por `montar_deslocado`, na posição nova). A versão muda ao ligar/desligar isso.
#[allow(clippy::too_many_arguments)]
pub fn montar_arrastando(doc: &Documento, vista: &Vista, versao_conhecida: Option<u64>, fundo: Option<f64>, fundo_claro: bool, selecao: &BTreeSet<Id>, revisao: u64, ocultar_selecao: bool) -> Vec<u8> {
    let versao = doc.versao() + (revisao << 32) + if ocultar_selecao { 1 << 62 } else { 0 };
    if versao_conhecida == Some(versao) {
        return MAGIA_IGUAL.to_vec();
    }
    let ctx = Contexto { doc, vista, area: area_de_corte(vista), tolerancia: TOLERANCIA_DO_ARCO_PX / vista.escala, fundo_claro, simplificar: true };
    let mut dentro = Saida { lod: true, ..Saida::default() };
    let raiz = Nivel { t: Transf::IDENTIDADE, herdada: None, profundidade: 0, destaque: false };
    for e in doc.entidades() {
        let selecionada = selecao.contains(&e.id);
        if selecionada && ocultar_selecao {
            continue;
        }
        desenhar(&ctx, &Peca { camada: e.camada, cor: e.cor, geo: &e.geometria, destaque: selecionada, props: &e.props }, &raiz, &mut dentro);
    }
    escrever_quadro(versao, &dentro, vista, fundo)
}

/// Só os objetos `selecao`, movidos de `(dx, dy)`, na cor de seleção e sem fundo: a camada do arrasto ao vivo.
/// Sempre é um quadro novo (a versão não importa: muda a cada movimento do mouse).
pub fn montar_deslocado(doc: &Documento, vista: &Vista, fundo_claro: bool, selecao: &BTreeSet<Id>, dx: f64, dy: f64) -> Vec<u8> {
    let ctx = Contexto { doc, vista, area: area_de_corte(vista), tolerancia: TOLERANCIA_DO_ARCO_PX / vista.escala, fundo_claro, simplificar: true };
    let mut dentro = Saida::default();
    let raiz = Nivel { t: Transf::mover(dx, dy), herdada: None, profundidade: 0, destaque: false };
    for e in doc.entidades().iter().filter(|e| selecao.contains(&e.id)) {
        desenhar(&ctx, &Peca { camada: e.camada, cor: e.cor, geo: &e.geometria, destaque: true, props: &e.props }, &raiz, &mut dentro);
    }
    escrever_quadro(0, &dentro, vista, None)
}

fn escrever_quadro(versao: u64, dentro: &Saida, vista: &Vista, fundo: Option<f64>) -> Vec<u8> {
    let mut saida = Vec::with_capacity(24 + dentro.corpo.len() + dentro.textos.len());
    saida.extend_from_slice(MAGIA_NOVO);
    saida.extend_from_slice(&versao.to_le_bytes());
    saida.extend_from_slice(&dentro.n.to_le_bytes());
    saida.extend_from_slice(&dentro.corpo);
    saida.extend_from_slice(&dentro.nt.to_le_bytes());
    saida.extend_from_slice(&dentro.textos);
    saida.extend_from_slice(&dentro.np.to_le_bytes());
    saida.extend_from_slice(&dentro.preenchimentos);
    let (xs, ys) = fundo.map(|base| marcas(vista, base)).unwrap_or_default();
    saida.extend_from_slice(&(xs.len() as u32).to_le_bytes());
    saida.extend_from_slice(&(ys.len() as u32).to_le_bytes());
    escrever_marcas(&mut saida, &xs);
    escrever_marcas(&mut saida, &ys);
    saida.extend_from_slice(&dentro.ni.to_le_bytes());
    saida.extend_from_slice(&dentro.imagens);
    saida
}

#[cfg(test)]
mod lod {
    use super::*;
    use crate::documento::{Bloco, Documento, NovaEntidade};
    use crate::geom::Vertice;

    fn segmentos_do_quadro(bytes: &[u8]) -> u32 {
        u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]])
    }

    fn vista(escala: f64) -> Vista {
        Vista { centro: Ponto::novo(0.0, 0.0), escala, largura: 1000.0, altura: 800.0, rotacao_x: 0.0, rotacao_z: 0.0 }
    }

    #[test]
    fn curva_de_mil_cordas_vira_dezenas_com_o_desenho_todo_na_tela_e_fica_inteira_de_perto() {
        let mut d = Documento::default();
        let vertices: Vec<Vertice> = (0..=1000).map(|i| Vertice { p: Ponto::novo(f64::from(i) * 0.01, (f64::from(i) * 0.05).sin() * 0.3), bulge: 0.0 }).collect();
        d.adicionar_polilinha(vertices, false).unwrap();
        // longe: 10 unidades em 5 px (0,5 px/unidade) → as 1000 cordas colapsam
        let longe = montar(&d, &vista(0.5), None, None, false);
        assert!(segmentos_do_quadro(&longe) < 20, "{}", segmentos_do_quadro(&longe));
        // perto: 10 unidades em 1000 px → cada corda tem 10 px, nenhuma some
        let perto = montar(&d, &Vista { centro: Ponto::novo(5.0, 0.0), escala: 100.0, largura: 1000.0, altura: 800.0, rotacao_x: 0.0, rotacao_z: 0.0 }, None, None, false);
        assert!(segmentos_do_quadro(&perto) >= 1000, "{}", segmentos_do_quadro(&perto));
    }

    #[test]
    fn dez_mil_linhas_miudas_no_mesmo_pixel_saem_uma_vez_so() {
        let mut d = Documento::default();
        for i in 0..10_000 {
            let y = f64::from(i % 3) * 0.001;
            d.adicionar_linha(Ponto::novo(0.0, y), Ponto::novo(0.3, y));
        }
        // escala 1 px/unidade: cada linha tem 0,3 px e todas caem no pixel do centro
        let q = montar(&d, &vista(1.0), None, None, false);
        assert!(segmentos_do_quadro(&q) <= 3, "{}", segmentos_do_quadro(&q));
        // a impressão (fino) não simplifica: continua com tudo
        let fino = montar_fino(&d, &vista(1.0), false, 0.05);
        assert_eq!(segmentos_do_quadro(&fino), 10_000);
    }

    #[test]
    fn bloco_minusculo_na_tela_vira_um_ponto_e_de_perto_desce_no_bloco() {
        let arvore = Bloco {
            nome: "ARVORE".into(),
            base: Ponto::novo(0.0, 0.0),
            entidades: vec![
                NovaEntidade { camada: 0, cor: None, geometria: crate::documento::Geometria::Circulo { centro: Ponto::novo(0.0, 0.0), raio: 1.0 }, props: Default::default(), espaco: crate::documento::Espaco::Modelo },
                NovaEntidade { camada: 0, cor: None, geometria: crate::documento::Geometria::Linha { a: Ponto::novo(-1.0, 0.0), b: Ponto::novo(1.0, 0.0) }, props: Default::default(), espaco: crate::documento::Espaco::Modelo },
            ],
        };
        let insercoes = (0..500).map(|i| NovaEntidade { camada: 0, cor: None, geometria: crate::documento::Geometria::Insercao { bloco: 0, ponto: Ponto::novo(f64::from(i) * 3.0, 0.0), escala_x: 1.0, escala_y: 1.0, rotacao: 0.0 }, props: Default::default(), espaco: crate::documento::Espaco::Modelo }).collect();
        let d = Documento::de_partes_com_blocos(Vec::new(), 0, vec![arvore], insercoes);
        // escala 0,1: cada árvore (2 unidades) mede 0,2 px → um ponto por árvore
        let longe = montar(&d, &Vista { centro: Ponto::novo(750.0, 0.0), escala: 0.1, largura: 1000.0, altura: 800.0, rotacao_x: 0.0, rotacao_z: 0.0 }, None, None, false);
        assert!(segmentos_do_quadro(&longe) <= 500, "{}", segmentos_do_quadro(&longe));
        // escala 50: a árvore mede 100 px → círculo (muitas cordas) e a linha
        let perto = montar(&d, &Vista { centro: Ponto::novo(0.0, 0.0), escala: 50.0, largura: 1000.0, altura: 800.0, rotacao_x: 0.0, rotacao_z: 0.0 }, None, None, false);
        assert!(segmentos_do_quadro(&perto) > 10, "{}", segmentos_do_quadro(&perto));
    }
}
