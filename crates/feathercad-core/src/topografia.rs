//! Processamento de topografia e nuvens de pontos.
//!
//! Converte nuvens de pontos (CSV, XYZ, PTS, TXT) em levantamentos topográficos completos:
//! - Malha triangular irregular (TIN / Delaunay 2D)
//! - Curvas de nível (mestras e secundárias com interpolação linear)
//! - Rótulos de cota ao longo das curvas mestras
//! - Borda/perímetro do levantamento
//! - Pontos cotados com indicação de elevação
//! - Quadro de resumo estatístico do relevo

use std::collections::{HashMap, HashSet};

use crate::camadas::Propriedade;
use crate::documento::{AlinhaH, AlinhaV, Documento, Espaco, Geometria, Id, Mudanca, NovaEntidade, Propriedades, Unidade};
use crate::geom::{Ponto, Vertice};

/// Um triângulo da malha TIN indexando os pontos da nuvem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Triangulo {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}

impl Triangulo {
    pub fn novo(a: usize, b: usize, c: usize) -> Self {
        Self { a, b, c }
    }

    /// As 3 arestas do triângulo, ordenadas com o menor índice primeiro.
    pub fn arestas(self) -> [(usize, usize); 3] {
        [
            (self.a.min(self.b), self.a.max(self.b)),
            (self.b.min(self.c), self.b.max(self.c)),
            (self.c.min(self.a), self.c.max(self.a)),
        ]
    }
}

/// Uma curva de nível com sua cota, classificação e vértices.
#[derive(Clone, Debug, PartialEq)]
pub struct CurvaNivel {
    pub cota: f64,
    pub eh_mestra: bool,
    pub pontos: Vec<Ponto>,
    pub fechada: bool,
}

/// Opções para geração do levantamento topográfico.
#[derive(Clone, Debug)]
pub struct OpcoesTopografia {
    /// Equidistância vertical entre curvas de nível em metros (ex: 1.0 m).
    /// Se `None`, calculada automaticamente pela amplitude altimétrica.
    pub equidistancia: Option<f64>,
    /// A cada quantas curvas intermediárias ocorre uma curva mestra (padrão: 5).
    pub passo_mestra: usize,
    /// Comprimento máximo da aresta da malha TIN (em metros) para poda da borda côncava.
    /// Se `None`, estimado pela densidade dos pontos.
    pub max_comprimento_aresta: Option<f64>,
    /// Se deve incluir as linhas da malha triangular (TIN) na camada TOPOGRAFIA_TIN.
    pub incluir_tin: bool,
    /// Se deve incluir as faces 3D (3DFACE) da malha na camada TOPOGRAFIA_MALHA_3D.
    pub incluir_faces_3d: bool,
    /// Se deve incluir os pontos levantados na camada TOPOGRAFIA_PONTOS.
    pub incluir_pontos: bool,
    /// Se deve incluir textos de cota nos pontos levantados (recomendado para < 2000 pontos).
    pub rotular_pontos: bool,
    /// Se deve gerar o quadro de resumo estatístico.
    pub incluir_quadro: bool,
    /// Fator de exagero vertical para a geometria 3D (padrão 1.0 = escala real).
    pub exagero_vertical: Option<f64>,
}

impl Default for OpcoesTopografia {
    fn default() -> Self {
        Self {
            equidistancia: None,
            passo_mestra: 5,
            max_comprimento_aresta: None,
            incluir_tin: true,
            incluir_faces_3d: true,
            incluir_pontos: true,
            rotular_pontos: true,
            incluir_quadro: true,
            exagero_vertical: None,
        }
    }
}

/// Resumo estatístico do levantamento topográfico gerado.
#[derive(Clone, Debug, PartialEq)]
pub struct ResumoTopografico {
    pub total_pontos: usize,
    pub total_triangulos: usize,
    pub total_curvas: usize,
    pub cota_min: f64,
    pub cota_max: f64,
    pub cota_media: f64,
    pub desnivel: f64,
    pub equidistancia: f64,
    pub passo_mestra: usize,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub area_m2: f64,
    pub ids_criados: Vec<Id>,
}

// -----------------------------------------------------------------------------
// Leitura de Nuvem de Pontos
// -----------------------------------------------------------------------------

/// Extrai coordenadas (X, Y, Z) de uma linha de texto nos formatos comuns.
fn extrair_coordenadas(linha: &str) -> Option<(f64, f64, f64)> {
    let limpa = linha.trim();
    if limpa.is_empty() || limpa.starts_with('#') || limpa.starts_with("//") {
        return None;
    }

    let separador = if limpa.contains(',') {
        ','
    } else if limpa.contains(';') {
        ';'
    } else if limpa.contains('\t') {
        '\t'
    } else {
        ' '
    };

    let pedacos: Vec<&str> = if separador == ' ' {
        limpa.split_whitespace().collect()
    } else {
        limpa.split(separador).map(str::trim).filter(|s| !s.is_empty()).collect()
    };

    let mut nums: Vec<f64> = Vec::with_capacity(pedacos.len());
    for p in pedacos {
        if let Ok(v) = p.parse::<f64>() {
            nums.push(v);
        }
    }

    if nums.len() < 3 {
        return None;
    }

    if nums.len() >= 4 && nums[0].fract().abs() < 1e-6 && nums[0] >= 0.0 && nums[0] < 500_000.0 && (nums[1].abs() > 1_000.0 || nums[2].abs() > 1_000.0) {
        Some((nums[1], nums[2], nums[3]))
    } else {
        Some((nums[0], nums[1], nums[2]))
    }
}

/// Deduplica pontos que estejam a menos de `tolerancia` de distância horizontal.
pub fn deduplicar_pontos(pontos: &[Ponto], tolerancia: f64) -> Vec<Ponto> {
    if pontos.is_empty() {
        return Vec::new();
    }
    let mut ordenados: Vec<Ponto> = pontos.to_vec();
    ordenados.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });

    let tol2 = tolerancia * tolerancia;
    let mut unicos: Vec<Ponto> = Vec::with_capacity(ordenados.len());

    for p in ordenados {
        let mut duplicado = false;
        for u in unicos.iter().rev() {
            if (p.x - u.x).abs() > tolerancia {
                break;
            }
            if (p.x - u.x).powi(2) + (p.y - u.y).powi(2) < tol2 {
                duplicado = true;
                break;
            }
        }
        if !duplicado {
            unicos.push(p);
        }
    }
    unicos
}

/// Lê uma nuvem de pontos em texto (CSV, XYZ, PTS, TXT) e devolve a lista de pontos 3D únicos.
pub fn ler_nuvem_pontos(conteudo: &str) -> Result<Vec<Ponto>, String> {
    let mut brutos: Vec<Ponto> = Vec::new();

    for linha in conteudo.lines() {
        if let Some((x, y, z)) = extrair_coordenadas(linha) {
            brutos.push(Ponto::novo3d(x, y, z));
        }
    }

    if brutos.is_empty() {
        return Err("Nenhum ponto válido encontrado no arquivo.".into());
    }

    let unicos = deduplicar_pontos(&brutos, 0.005);
    if unicos.len() < 3 {
        return Err("A nuvem precisa conter pelo menos 3 pontos distintos para gerar levantamento topográfico.".into());
    }

    Ok(unicos)
}

// -----------------------------------------------------------------------------
// Triangulação de Delaunay 2D (TIN)
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Circuncentro {
    cx: f64,
    cy: f64,
    r2: f64,
}

fn circuncentro(a: Ponto, b: Ponto, c: Ponto) -> Option<Circuncentro> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-11 {
        return None;
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;

    let cx = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let cy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    let r2 = (a.x - cx).powi(2) + (a.y - cy).powi(2);

    Some(Circuncentro { cx, cy, r2 })
}

#[derive(Clone, Copy, Debug)]
struct TriAtivo {
    a: usize,
    b: usize,
    c: usize,
    circ: Circuncentro,
}

/// Executa a triangulação de Delaunay 2D pelo algoritmo incremental de Bowyer-Watson.
pub fn triangular_delaunay(pontos: &[Ponto], max_aresta: Option<f64>) -> Vec<Triangulo> {
    let n = pontos.len();
    if n < 3 {
        return Vec::new();
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in pontos {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let dmax = dx.max(dy).max(1.0);
    let x_mid = (min_x + max_x) / 2.0;
    let y_mid = (min_y + max_y) / 2.0;

    let s0 = Ponto::novo3d(x_mid - 20.0 * dmax, y_mid - dmax, 0.0);
    let s1 = Ponto::novo3d(x_mid, y_mid + 20.0 * dmax, 0.0);
    let s2 = Ponto::novo3d(x_mid + 20.0 * dmax, y_mid - dmax, 0.0);

    let mut todos_pontos: Vec<Ponto> = pontos.to_vec();
    todos_pontos.push(s0);
    todos_pontos.push(s1);
    todos_pontos.push(s2);

    let circ_sup = circuncentro(s0, s1, s2).unwrap_or(Circuncentro { cx: x_mid, cy: y_mid, r2: 1e12 });
    let mut ativos = vec![TriAtivo { a: n, b: n + 1, c: n + 2, circ: circ_sup }];
    let mut finalizados: Vec<Triangulo> = Vec::new();

    for (i, p) in pontos.iter().enumerate() {
        let mut arestas_cavidade: HashMap<(usize, usize), usize> = HashMap::new();
        let mut novos_ativos: Vec<TriAtivo> = Vec::with_capacity(ativos.len());

        for tri in ativos {
            let dist2 = (p.x - tri.circ.cx).powi(2) + (p.y - tri.circ.cy).powi(2);
            if dist2 <= tri.circ.r2 + 1e-9 {
                for edge in [
                    (tri.a.min(tri.b), tri.a.max(tri.b)),
                    (tri.b.min(tri.c), tri.b.max(tri.c)),
                    (tri.c.min(tri.a), tri.c.max(tri.a)),
                ] {
                    *arestas_cavidade.entry(edge).or_insert(0) += 1;
                }
            } else {
                novos_ativos.push(tri);
            }
        }

        ativos = novos_ativos;

        for ((u, v), contagem) in arestas_cavidade {
            if contagem == 1 {
                if let Some(circ) = circuncentro(todos_pontos[u], todos_pontos[v], todos_pontos[i]) {
                    ativos.push(TriAtivo { a: u, b: v, c: i, circ });
                }
            }
        }
    }

    for tri in ativos {
        finalizados.push(Triangulo::novo(tri.a, tri.b, tri.c));
    }

    let mut limpos: Vec<Triangulo> = finalizados
        .into_iter()
        .filter(|t| t.a < n && t.b < n && t.c < n)
        .collect();

    let limite_aresta = max_aresta.unwrap_or_else(|| estimar_max_aresta(&limpos, pontos));
    podar_arestas_de_borda(&mut limpos, pontos, limite_aresta);

    limpos
}

fn estimar_max_aresta(triangulos: &[Triangulo], pontos: &[Ponto]) -> f64 {
    if triangulos.is_empty() {
        return 100.0;
    }
    let mut comprimentos: Vec<f64> = Vec::with_capacity(triangulos.len() * 3);
    for t in triangulos {
        for (u, v) in t.arestas() {
            comprimentos.push(pontos[u].distancia(pontos[v]));
        }
    }
    comprimentos.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mediana = comprimentos[comprimentos.len() / 2];
    (mediana * 4.0).clamp(35.0, 250.0)
}

fn podar_arestas_de_borda(triangulos: &mut Vec<Triangulo>, pontos: &[Ponto], limite_aresta: f64) {
    let lim2 = limite_aresta * limite_aresta;
    let mut removeu_algum = true;

    while removeu_algum {
        removeu_algum = false;
        let mut contagem: HashMap<(usize, usize), usize> = HashMap::new();
        for t in triangulos.iter() {
            for edge in t.arestas() {
                *contagem.entry(edge).or_insert(0) += 1;
            }
        }

        let mut mantidos: Vec<Triangulo> = Vec::with_capacity(triangulos.len());
        for t in triangulos.drain(..) {
            let mut tem_aresta_borda_longa = false;
            for (u, v) in t.arestas() {
                if contagem.get(&(u, v)).copied().unwrap_or(0) == 1 {
                    let d2 = (pontos[u].x - pontos[v].x).powi(2) + (pontos[u].y - pontos[v].y).powi(2);
                    if d2 > lim2 {
                        tem_aresta_borda_longa = true;
                        break;
                    }
                }
            }

            if tem_aresta_borda_longa {
                removeu_algum = true;
            } else {
                mantidos.push(t);
            }
        }
        *triangulos = mantidos;
    }
}

// -----------------------------------------------------------------------------
// Curvas de Nível (Isoípsas)
// -----------------------------------------------------------------------------

fn interpolar_corte(p1: Ponto, p2: Ponto, z: f64) -> Option<Ponto> {
    let z_min = p1.z.min(p2.z);
    let z_max = p1.z.max(p2.z);

    if z <= z_min || z > z_max {
        return None;
    }

    let dz = p2.z - p1.z;
    if dz.abs() < 1e-9 {
        return None;
    }

    let t = (z - p1.z) / dz;
    let x = p1.x + t * (p2.x - p1.x);
    let y = p1.y + t * (p2.y - p1.y);
    Some(Ponto::novo3d(x, y, z))
}

fn cortar_triangulo(t: Triangulo, pontos: &[Ponto], z: f64) -> Option<(Ponto, Ponto)> {
    let p0 = pontos[t.a];
    let p1 = pontos[t.b];
    let p2 = pontos[t.c];

    let mut cortes: Vec<Ponto> = Vec::with_capacity(2);
    for (a, b) in [(p0, p1), (p1, p2), (p2, p0)] {
        if let Some(corte) = interpolar_corte(a, b, z) {
            cortes.push(corte);
        }
    }

    if cortes.len() == 2 {
        Some((cortes[0], cortes[1]))
    } else {
        None
    }
}

type ArestaVizinha = (usize, (i64, i64), Ponto);
type GrafoAdjacencia = HashMap<(i64, i64), Vec<ArestaVizinha>>;

fn encadear_segmentos(segmentos: &[(Ponto, Ponto)]) -> Vec<(Vec<Ponto>, bool)> {
    if segmentos.is_empty() {
        return Vec::new();
    }

    fn chave(p: Ponto) -> (i64, i64) {
        ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64)
    }

    let mut adj: GrafoAdjacencia = HashMap::new();
    for (seg_idx, (a, b)) in segmentos.iter().enumerate() {
        let ka = chave(*a);
        let kb = chave(*b);
        adj.entry(ka).or_default().push((seg_idx, kb, *b));
        adj.entry(kb).or_default().push((seg_idx, ka, *a));
    }

    let mut usado = vec![false; segmentos.len()];
    let mut polilinhas: Vec<(Vec<Ponto>, bool)> = Vec::new();

    let pontas: Vec<(i64, i64)> = adj
        .iter()
        .filter(|(_, viz)| viz.len() == 1)
        .map(|(k, _)| *k)
        .collect();

    for mut atual_k in pontas {
        let mut caminho: Vec<Ponto> = Vec::new();
        while let Some(&(seg_id, prox_k, prox_pt)) = adj.get(&atual_k).and_then(|v| v.iter().find(|(id, _, _)| !usado[*id])) {
            usado[seg_id] = true;
            if caminho.is_empty() {
                let pt_ini = if chave(segmentos[seg_id].0) == atual_k { segmentos[seg_id].0 } else { segmentos[seg_id].1 };
                caminho.push(pt_ini);
            }
            caminho.push(prox_pt);
            atual_k = prox_k;
        }

        if caminho.len() >= 2 {
            polilinhas.push((caminho, false));
        }
    }

    for (seg_idx, (a, b)) in segmentos.iter().enumerate() {
        if usado[seg_idx] {
            continue;
        }
        usado[seg_idx] = true;
        let mut caminho = vec![*a, *b];
        let mut atual_k = chave(*b);
        let inicio_k = chave(*a);

        while let Some(&(prox_seg, prox_k, prox_pt)) = adj.get(&atual_k).and_then(|v| v.iter().find(|(id, _, _)| !usado[*id])) {
            usado[prox_seg] = true;
            caminho.push(prox_pt);
            atual_k = prox_k;
            if atual_k == inicio_k {
                break;
            }
        }

        if caminho.len() >= 3 {
            let eh_fechada = atual_k == inicio_k;
            if eh_fechada {
                caminho.pop();
            }
            polilinhas.push((caminho, eh_fechada));
        }
    }

    polilinhas
}

/// Gera as curvas de nível mestras e secundárias a partir da malha triangular.
pub fn gerar_curvas_nivel(
    pontos: &[Ponto],
    triangulos: &[Triangulo],
    equidistancia: f64,
    passo_mestra: usize,
) -> Vec<CurvaNivel> {
    if triangulos.is_empty() || pontos.is_empty() || equidistancia <= 0.0 {
        return Vec::new();
    }

    let z_min = pontos.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
    let z_max = pontos.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);

    let k_min = (z_min / equidistancia).ceil() as i64;
    let k_max = (z_max / equidistancia).floor() as i64;

    let mut curvas: Vec<CurvaNivel> = Vec::new();
    let passo_mestra = passo_mestra.max(1);

    for k in k_min..=k_max {
        let cota = k as f64 * equidistancia;
        let z_corte = cota + 1e-6 * equidistancia;

        let mut segmentos: Vec<(Ponto, Ponto)> = Vec::new();
        for t in triangulos {
            if let Some(seg) = cortar_triangulo(*t, pontos, z_corte) {
                segmentos.push((Ponto::novo3d(seg.0.x, seg.0.y, cota), Ponto::novo3d(seg.1.x, seg.1.y, cota)));
            }
        }

        let eh_mestra = k.rem_euclid(passo_mestra as i64) == 0;
        let linhas = encadear_segmentos(&segmentos);

        for (pts, fechada) in linhas {
            if pts.len() >= 2 {
                curvas.push(CurvaNivel {
                    cota,
                    eh_mestra,
                    pontos: pts,
                    fechada,
                });
            }
        }
    }

    curvas
}

/// Extrai os segmentos que formam a borda externa da malha triangular.
pub fn extrair_borda_externa(pontos: &[Ponto], triangulos: &[Triangulo]) -> Vec<(Vec<Ponto>, bool)> {
    let mut contagem: HashMap<(usize, usize), usize> = HashMap::new();
    for t in triangulos {
        for aresta in t.arestas() {
            *contagem.entry(aresta).or_insert(0) += 1;
        }
    }

    let segmentos: Vec<(Ponto, Ponto)> = contagem
        .into_iter()
        .filter(|(_, c)| *c == 1)
        .map(|((u, v), _)| (pontos[u], pontos[v]))
        .collect();

    encadear_segmentos(&segmentos)
}

// -----------------------------------------------------------------------------
// Criação do Documento Topográfico Completo
// -----------------------------------------------------------------------------

pub const CAMADA_MESTRAS: &str = "TOPOGRAFIA_CURVAS_MESTRAS";
pub const CAMADA_SECUNDARIAS: &str = "TOPOGRAFIA_CURVAS_SECUNDARIAS";
pub const CAMADA_ROTULOS: &str = "TOPOGRAFIA_ROTULOS";
pub const CAMADA_TIN: &str = "TOPOGRAFIA_TIN";
pub const CAMADA_MALHA_3D: &str = "TOPOGRAFIA_MALHA_3D";
pub const CAMADA_PONTOS: &str = "TOPOGRAFIA_PONTOS";
pub const CAMADA_BORDA: &str = "TOPOGRAFIA_BORDA";
pub const CAMADA_QUADRO: &str = "TOPOGRAFIA_QUADRO";

/// Obtém o índice de uma camada pelo nome ou cria uma nova com os parâmetros dados.
pub fn obter_ou_criar_camada(doc: &mut Documento, nome: &str, cor: u32, espessura: Option<f32>) -> usize {
    if let Some(idx) = doc.camadas().iter().position(|c| c.nome.eq_ignore_ascii_case(nome)) {
        return idx;
    }
    let idx = doc.nova_camada(nome, None).unwrap_or(0);
    let _ = doc.definir_propriedade_de_camadas(&[idx], Propriedade::Cor(cor));
    if let Some(esp) = espessura {
        let _ = doc.definir_propriedade_de_camadas(&[idx], Propriedade::Espessura(Some(esp)));
    }
    idx
}

/// Assegura que as camadas topográficas padrões existam no documento e devolve seus índices.
pub fn configurar_camadas_topografia(doc: &mut Documento) -> CamadasIndices {
    CamadasIndices {
        mestras: obter_ou_criar_camada(doc, CAMADA_MESTRAS, 0x8B_45_13, Some(0.50)),
        secundarias: obter_ou_criar_camada(doc, CAMADA_SECUNDARIAS, 0xD2_B4_8C, Some(0.20)),
        rotulos: obter_ou_criar_camada(doc, CAMADA_ROTULOS, 0x8B_45_13, None),
        tin: obter_ou_criar_camada(doc, CAMADA_TIN, 0x70_80_90, Some(0.13)),
        malha_3d: obter_ou_criar_camada(doc, CAMADA_MALHA_3D, 0x46_82_B4, Some(0.15)),
        pontos: obter_ou_criar_camada(doc, CAMADA_PONTOS, 0x29_80_B9, Some(0.18)),
        borda: obter_ou_criar_camada(doc, CAMADA_BORDA, 0x27_AE_60, Some(0.35)),
        quadro: obter_ou_criar_camada(doc, CAMADA_QUADRO, 0x2C_3E_50, Some(0.25)),
    }
}

pub struct CamadasIndices {
    pub mestras: usize,
    pub secundarias: usize,
    pub rotulos: usize,
    pub tin: usize,
    pub malha_3d: usize,
    pub pontos: usize,
    pub borda: usize,
    pub quadro: usize,
}

fn adicionar_mudanca(mudancas: &mut Vec<Mudanca>, camada: usize, geo: Geometria, espaco: &Espaco) {
    mudancas.push(Mudanca::Novo(NovaEntidade {
        camada,
        cor: None,
        geometria: geo,
        props: Propriedades::default(),
        espaco: espaco.clone(),
    }));
}

fn equidistancia_padrao(desnivel: f64) -> f64 {
    if desnivel <= 5.0 {
        0.50
    } else if desnivel <= 20.0 {
        1.00
    } else if desnivel <= 60.0 {
        2.00
    } else {
        5.00
    }
}

/// Os pontos da nuvem (e, se pedido e não forem muitos, a cota escrita ao lado de cada um).
fn adicionar_pontos_amostrados(mudancas: &mut Vec<Mudanca>, camada: usize, pontos: &[Ponto], rotular: bool, ajustar_z: &dyn Fn(f64) -> f64, espaco: &Espaco) {
    let altura_txt = 1.6;
    for p in pontos {
        let p_3d = Ponto::novo3d(p.x, p.y, ajustar_z(p.z));
        adicionar_mudanca(mudancas, camada, Geometria::Ponto { p: p_3d }, espaco);
        if rotular && pontos.len() <= 2000 {
            let rotulo = format!("{:.2}", p.z);
            adicionar_mudanca(
                mudancas,
                camada,
                Geometria::Texto {
                    ponto: Ponto::novo3d(p.x + 0.8, p.y + 0.8, p_3d.z),
                    altura: altura_txt,
                    rotacao: 0.0,
                    conteudo: rotulo,
                    h: AlinhaH::Esquerda,
                    v: AlinhaV::Base,
                },
                espaco,
            );
        }
    }
}

/// Cria o levantamento topográfico completo no documento a partir da nuvem de pontos.
pub fn criar_levantamento_topografico(
    doc: &mut Documento,
    pontos: &[Ponto],
    opcoes: &OpcoesTopografia,
) -> Result<ResumoTopografico, String> {
    if pontos.len() < 3 {
        return Err("Levantamento requer pelo menos 3 pontos".into());
    }

    doc.definir_unidade(Some(Unidade::Metro));
    let camadas = configurar_camadas_topografia(doc);
    let espaco = doc.espaco_ativo().clone();

    let z_min = pontos.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
    let z_max = pontos.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
    let desnivel = z_max - z_min;
    let equidistancia = opcoes.equidistancia.unwrap_or_else(|| equidistancia_padrao(desnivel));

    let triangulos = triangular_delaunay(pontos, opcoes.max_comprimento_aresta);
    let curvas = gerar_curvas_nivel(pontos, &triangulos, equidistancia, opcoes.passo_mestra);

    let z_mult = opcoes.exagero_vertical.unwrap_or(1.0);
    let ajustar_z = |z: f64| -> f64 {
        if (z_mult - 1.0).abs() > 1e-6 {
            (z - z_min) * z_mult + z_min
        } else {
            z
        }
    };

    let mut mudancas = Vec::new();

    // 1. Curvas de Nível e rótulos (com cotas 3D)
    for curva in &curvas {
        let camada = if curva.eh_mestra { camadas.mestras } else { camadas.secundarias };
        let z_3d = ajustar_z(curva.cota);
        let vertices: Vec<Vertice> = curva.pontos.iter().map(|p| Vertice {
            p: Ponto::novo3d(p.x, p.y, z_3d),
            bulge: 0.0,
        }).collect();
        adicionar_mudanca(&mut mudancas, camada, Geometria::Polilinha { vertices, fechada: curva.fechada }, &espaco);

        if curva.eh_mestra && curva.pontos.len() >= 2 {
            adicionar_rotulos_curva(&mut mudancas, camadas.rotulos, curva, z_3d, &espaco);
        }
    }

    // 2. Malha TIN (wireframe 3D)
    if opcoes.incluir_tin {
        let mut arestas_vistas = HashSet::new();
        for t in &triangulos {
            for (u, v) in t.arestas() {
                if arestas_vistas.insert((u, v)) {
                    let mut a = pontos[u];
                    let mut b = pontos[v];
                    a.z = ajustar_z(a.z);
                    b.z = ajustar_z(b.z);
                    adicionar_mudanca(&mut mudancas, camadas.tin, Geometria::Linha { a, b }, &espaco);
                }
            }
        }
    }

    // 3. Malha 3D (Faces 3DFACE / Superfície contínua de terreno)
    if opcoes.incluir_faces_3d {
        for t in &triangulos {
            let mut p1 = pontos[t.a];
            let mut p2 = pontos[t.b];
            let mut p3 = pontos[t.c];
            p1.z = ajustar_z(p1.z);
            p2.z = ajustar_z(p2.z);
            p3.z = ajustar_z(p3.z);
            adicionar_mudanca(&mut mudancas, camadas.malha_3d, Geometria::Face3D { p1, p2, p3, p4: p3 }, &espaco);
        }
    }

    // 4. Borda do Terreno
    let bordas = extrair_borda_externa(pontos, &triangulos);
    for (borda_pts, fechada) in bordas {
        let vertices: Vec<Vertice> = borda_pts.iter().map(|p| Vertice {
            p: Ponto::novo3d(p.x, p.y, ajustar_z(p.z)),
            bulge: 0.0,
        }).collect();
        adicionar_mudanca(&mut mudancas, camadas.borda, Geometria::Polilinha { vertices, fechada }, &espaco);
    }

    // 5. Pontos amostrados
    if opcoes.incluir_pontos {
        adicionar_pontos_amostrados(&mut mudancas, camadas.pontos, pontos, opcoes.rotular_pontos, &ajustar_z, &espaco);
    }

    let min_x = pontos.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = pontos.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = pontos.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = pontos.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    let cota_media = pontos.iter().map(|p| p.z).sum::<f64>() / pontos.len() as f64;
    let area_m2 = calcular_area_tin(pontos, &triangulos);

    let mut resumo = ResumoTopografico {
        total_pontos: pontos.len(),
        total_triangulos: triangulos.len(),
        total_curvas: curvas.len(),
        cota_min: z_min,
        cota_max: z_max,
        cota_media,
        desnivel,
        equidistancia,
        passo_mestra: opcoes.passo_mestra,
        min_x,
        max_x,
        min_y,
        max_y,
        area_m2,
        ids_criados: Vec::new(),
    };

    // 6. Quadro Estatístico
    if opcoes.incluir_quadro {
        adicionar_quadro_estatistico(&mut mudancas, camadas.quadro, &resumo, opcoes.exagero_vertical, &espaco);
    }

    resumo.ids_criados = doc.aplicar_mudancas(mudancas);
    Ok(resumo)
}

fn adicionar_rotulos_curva(mudancas: &mut Vec<Mudanca>, camada: usize, curva: &CurvaNivel, z_3d: f64, espaco: &Espaco) {
    let pts = &curva.pontos;
    if pts.len() < 2 {
        return;
    }

    let mut dist_acumulada = 0.0;
    let mut distancias = vec![0.0];
    for w in pts.windows(2) {
        dist_acumulada += w[0].distancia(w[1]);
        distancias.push(dist_acumulada);
    }

    let txt = format!("{:.1} m", curva.cota);
    let intervalo = 80.0;

    let mut proximo_alvo = (dist_acumulada / 2.0).min(intervalo);
    while proximo_alvo < dist_acumulada {
        for i in 0..pts.len() - 1 {
            if distancias[i] <= proximo_alvo && proximo_alvo <= distancias[i + 1] {
                let p1 = pts[i];
                let p2 = pts[i + 1];
                let seg_len = distancias[i + 1] - distancias[i];
                let t = if seg_len > 1e-4 { (proximo_alvo - distancias[i]) / seg_len } else { 0.5 };
                let pos = Ponto::novo3d(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y), z_3d);

                let mut ang = (p2.y - p1.y).atan2(p2.x - p1.x).to_degrees();
                if ang > 90.0 || ang <= -90.0 {
                    ang = (ang + 180.0).rem_euclid(360.0);
                }

                adicionar_mudanca(
                    mudancas,
                    camada,
                    Geometria::Texto {
                        ponto: pos,
                        altura: 2.5,
                        rotacao: ang,
                        conteudo: txt.clone(),
                        h: AlinhaH::Centro,
                        v: AlinhaV::Meio,
                    },
                    espaco,
                );
                break;
            }
        }
        proximo_alvo += intervalo;
    }
}

fn calcular_area_tin(pontos: &[Ponto], triangulos: &[Triangulo]) -> f64 {
    let mut area_total = 0.0;
    for t in triangulos {
        let a = pontos[t.a];
        let b = pontos[t.b];
        let c = pontos[t.c];
        let area = 0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs();
        area_total += area;
    }
    area_total
}

fn adicionar_quadro_estatistico(
    mudancas: &mut Vec<Mudanca>,
    camada: usize,
    resumo: &ResumoTopografico,
    exagero_vertical: Option<f64>,
    espaco: &Espaco,
) {
    let largura_caixa = 260.0;
    let altura_linha = 7.0;

    let mut linhas = vec![
        format!("Pontos amostrados: {}", resumo.total_pontos),
        format!("Triângulos na malha (TIN): {}", resumo.total_triangulos),
        format!("Curvas de nível geradas: {}", resumo.total_curvas),
        format!("Cota Mínima: {:.3} m", resumo.cota_min),
        format!("Cota Máxima: {:.3} m", resumo.cota_max),
        format!("Desnível Total (ΔZ): {:.3} m", resumo.desnivel),
        format!("Cota Média: {:.3} m", resumo.cota_media),
        format!("Equidistância: {:.2} m (Mestras a cada {:.2} m)", resumo.equidistancia, resumo.equidistancia * resumo.passo_mestra as f64),
        format!("Área em Projeção: {:.2} m² ({:.4} ha)", resumo.area_m2, resumo.area_m2 / 10_000.0),
        format!("Extensão Norte: {:.2} m | Leste: {:.2} m", resumo.max_y - resumo.min_y, resumo.max_x - resumo.min_x),
    ];
    if let Some(exagero) = exagero_vertical {
        if (exagero - 1.0).abs() > 1e-4 {
            linhas.push(format!("Exagero Vertical (Geometria 3D): {:.1}x", exagero));
        }
    }

    let num_linhas = linhas.len();
    let altura_total = (num_linhas as f64 + 1.0) * altura_linha;

    let x0 = resumo.max_x + 25.0;
    let y0 = resumo.max_y;

    for (a, b) in [
        (Ponto::novo(x0, y0), Ponto::novo(x0 + largura_caixa, y0)),
        (Ponto::novo(x0 + largura_caixa, y0), Ponto::novo(x0 + largura_caixa, y0 - altura_total)),
        (Ponto::novo(x0 + largura_caixa, y0 - altura_total), Ponto::novo(x0, y0 - altura_total)),
        (Ponto::novo(x0, y0 - altura_total), Ponto::novo(x0, y0)),
        (Ponto::novo(x0, y0 - 10.0), Ponto::novo(x0 + largura_caixa, y0 - 10.0)),
    ] {
        adicionar_mudanca(mudancas, camada, Geometria::Linha { a, b }, espaco);
    }

    adicionar_mudanca(
        mudancas,
        camada,
        Geometria::Texto {
            ponto: Ponto::novo(x0 + largura_caixa / 2.0, y0 - 6.5),
            altura: 3.5,
            rotacao: 0.0,
            conteudo: "LEVANTAMENTO TOPOGRÁFICO - RESUMO".into(),
            h: AlinhaH::Centro,
            v: AlinhaV::Meio,
        },
        espaco,
    );

    let mut y_texto = y0 - 15.0;
    for l in linhas {
        adicionar_mudanca(
            mudancas,
            camada,
            Geometria::Texto {
                ponto: Ponto::novo(x0 + 8.0, y_texto),
                altura: 2.2,
                rotacao: 0.0,
                conteudo: l,
                h: AlinhaH::Esquerda,
                v: AlinhaV::Meio,
            },
            espaco,
        );
        y_texto -= altura_linha;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ler_nuvem_csv_e_xyz() {
        let csv = "E,N,Z\n100.0,200.0,15.5\n110.0,200.0,16.0\n105.0,210.0,17.2\n";
        let pts = ler_nuvem_pontos(csv).unwrap();
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Ponto::novo3d(100.0, 200.0, 15.5));
    }

    #[test]
    fn triangular_e_gerar_curvas() {
        let pts = vec![
            Ponto::novo3d(0.0, 0.0, 10.0),
            Ponto::novo3d(100.0, 0.0, 10.0),
            Ponto::novo3d(50.0, 100.0, 20.0),
            Ponto::novo3d(50.0, 30.0, 15.0),
        ];
        let tris = triangular_delaunay(&pts, None);
        assert!(!tris.is_empty());

        let curvas = gerar_curvas_nivel(&pts, &tris, 1.0, 5);
        assert!(!curvas.is_empty());
        for c in &curvas {
            assert!(c.cota >= 10.0 && c.cota <= 20.0);
        }
    }

    #[test]
    fn levantamento_completo_no_documento() {
        let csv = "X,Y,Z\n0,0,10\n10,0,12\n10,10,14\n0,10,11\n5,5,15\n";
        let pts = ler_nuvem_pontos(csv).unwrap();
        let mut doc = Documento::default();
        let resumo = criar_levantamento_topografico(&mut doc, &pts, &OpcoesTopografia::default()).unwrap();

        assert_eq!(resumo.total_pontos, 5);
        assert!(doc.camadas().iter().any(|c| c.nome == CAMADA_MESTRAS));
        assert!(doc.camadas().iter().any(|c| c.nome == CAMADA_TIN));
        assert!(doc.camadas().iter().any(|c| c.nome == CAMADA_BORDA));
        assert!(!doc.entidades().is_empty());
    }

    #[test]
    fn levantamento_nuvem_real_do_usuario() {
        let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../exemplos/nuvem_pontos_teste.csv");
        let conteudo = std::fs::read_to_string(caminho).expect("arquivo de teste existe");
        let pts = ler_nuvem_pontos(&conteudo).unwrap();
        assert!(pts.len() > 1000);

        let mut doc = Documento::default();
        let resumo = criar_levantamento_topografico(&mut doc, &pts, &OpcoesTopografia::default()).unwrap();

        assert_eq!(resumo.total_pontos, pts.len());
        assert!(resumo.total_triangulos > 500);
        assert!(resumo.total_curvas > 10);
        assert!(resumo.desnivel > 20.0);
        assert!(!resumo.ids_criados.is_empty());
    }

    #[test]
    fn levantamento_com_faces_3d_e_exagero() {
        let csv = "X,Y,Z\n0,0,10\n10,0,12\n10,10,14\n0,10,11\n5,5,15\n";
        let pts = ler_nuvem_pontos(csv).unwrap();
        let mut doc = Documento::default();
        let opcoes = OpcoesTopografia { incluir_faces_3d: true, exagero_vertical: Some(5.0), ..Default::default() };
        let resumo = criar_levantamento_topografico(&mut doc, &pts, &opcoes).unwrap();

        assert!(doc.camadas().iter().any(|c| c.nome == CAMADA_MALHA_3D));
        assert!(doc.entidades().iter().any(|e| matches!(e.geometria, Geometria::Face3D { .. })));
        assert_eq!(resumo.cota_min, 10.0);
        assert_eq!(resumo.cota_max, 15.0);
    }
}
