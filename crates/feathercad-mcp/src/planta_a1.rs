//! As plantas detalhadas do modelo `ferrovia`: folhas A1 em 1:2000 ao longo do traçado (como as de um projeto de via
//! permanente), cada uma com o mapa de um trecho (malha de coordenadas, eixo existente, eixo de projeto, trilhos, estacas,
//! quilômetros e pontos notáveis TS/SC/CS/ST) e, na coluna da direita, título, elementos das curvas do trecho, legenda e a
//! chave de articulação das folhas. A geometria é gerada aqui direto em Rust (sem passar por ferramentas), já nas
//! coordenadas da folha: em 1:2000 uma unidade (m) do desenho vale 0,5 mm de papel, então o mapa é só uma translação.

use feathercad_core::documento::{AlinhaH, AlinhaV, Geometria};
use feathercad_core::folha::InfoFolha;
use feathercad_core::geom::Ponto;
use feathercad_core::trajeto::{estaca_texto, km_texto, Trajeto};
use feathercad_core::viario::alinhamento::Alinhamento;
use feathercad_core::viario::superelevacao::ResultadoCurva;
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::fonte::Fonte;
use crate::grafico::recortar;
use crate::json::Res;
use crate::modelos::{inserir_folha, passo, passo_bonito, Contexto};
use crate::pecas::{aplicar, Peca, Pincel};
use crate::viario::legivel;

/// Denominador da escala das plantas.
pub const ESCALA: f64 = 2000.0;
/// Área do mapa na A1, em mm de papel: (x0, y0, x1, y1). A coluna da direita (x 640 a 821) leva o resto.
const MAPA: (f64, f64, f64, f64) = (35.0, 109.0, 625.0, 574.0);
/// Folga, em metros, entre o traçado e a borda do mapa.
const MARGEM_M: f64 = 40.0;
/// Unidades do desenho (m) por mm de papel.
const K: f64 = ESCALA / 1000.0;

const LARGURA_COLUNA: f64 = 181.0;
const X_COLUNA: f64 = 640.0;

/// Um trecho do traçado: o intervalo de progressivas que cabe numa folha e o centro da janela.
pub struct Trecho {
    pub de: f64,
    pub ate: f64,
    pub centro: Ponto,
}

/// Divide o traçado em trechos que cabem no mapa (a janela segue o traçado, qualquer que seja a direção).
pub fn dividir(a: &Alinhamento) -> Vec<Trecho> {
    let (w, h) = ((MAPA.2 - MAPA.0) * K - 2.0 * MARGEM_M, (MAPA.3 - MAPA.1) * K - 2.0 * MARGEM_M);
    let passo = 5.0;
    let n = (a.comprimento / passo).ceil() as usize;
    let prog = |i: usize| (a.progressiva_inicial + i as f64 * passo).min(a.progressiva_final());
    let mut trechos = Vec::new();
    let mut i0 = 0usize;
    while i0 < n {
        let p0 = a.em(prog(i0)).ponto;
        let (mut min, mut max) = (p0, p0);
        let mut i = i0;
        while i < n {
            let q = a.em(prog(i + 1)).ponto;
            let (nmin, nmax) = (Ponto::novo(min.x.min(q.x), min.y.min(q.y)), Ponto::novo(max.x.max(q.x), max.y.max(q.y)));
            if nmax.x - nmin.x > w || nmax.y - nmin.y > h {
                break;
            }
            (min, max) = (nmin, nmax);
            i += 1;
        }
        let i1 = i.max(i0 + 1);
        trechos.push(Trecho { de: prog(i0), ate: prog(i1), centro: Ponto::novo((min.x + max.x) / 2.0, (min.y + max.y) / 2.0) });
        i0 = i1;
    }
    trechos
}

/// O mapa de uma folha: a janela no mundo e onde ela fica na folha.
struct Janela {
    min: Ponto,
    max: Ponto,
    org: Ponto,
}

impl Janela {
    fn nova(t: &Trecho, f: &InfoFolha) -> Self {
        let (w, h) = ((MAPA.2 - MAPA.0) * K, (MAPA.3 - MAPA.1) * K);
        let min = Ponto::novo(t.centro.x - w / 2.0, t.centro.y - h / 2.0);
        Self { min, max: Ponto::novo(min.x + w, min.y + h), org: Ponto::novo(f.min.x + MAPA.0 * K, f.min.y + MAPA.1 * K) }
    }

    /// Do mundo para a folha.
    fn p(&self, q: Ponto) -> Ponto {
        Ponto::novo(self.org.x + q.x - self.min.x, self.org.y + q.y - self.min.y)
    }

    fn dentro(&self, q: Ponto) -> bool {
        q.x >= self.min.x && q.x <= self.max.x && q.y >= self.min.y && q.y <= self.max.y
    }

    /// Partes da polilinha que caem na janela, já em coordenadas da folha.
    fn recortar(&self, pts: &[Ponto]) -> Vec<Vec<Ponto>> {
        let (w, h) = (self.max.x - self.min.x, self.max.y - self.min.y);
        let mut saida: Vec<Vec<Ponto>> = Vec::new();
        let mut atual: Vec<Ponto> = Vec::new();
        for j in pts.windows(2) {
            let rel = |q: Ponto| (q.x - self.min.x, q.y - self.min.y);
            match recortar(rel(j[0]), rel(j[1]), w, h) {
                Some((p, q)) => {
                    let (p, q) = (self.p(Ponto::novo(p.0 + self.min.x, p.1 + self.min.y)), self.p(Ponto::novo(q.0 + self.min.x, q.1 + self.min.y)));
                    if atual.last().is_some_and(|u| u.distancia(p) > 1e-6) {
                        saida.push(std::mem::take(&mut atual));
                    }
                    if atual.is_empty() {
                        atual.push(p);
                    }
                    atual.push(q);
                }
                None => {
                    if !atual.is_empty() {
                        saida.push(std::mem::take(&mut atual));
                    }
                }
            }
        }
        if !atual.is_empty() {
            saida.push(atual);
        }
        saida
    }
}

fn polilinhas(p: &mut Pincel, j: &Janela, pts: &[Ponto]) {
    for parte in j.recortar(pts) {
        p.polilinha(&parte);
    }
}

fn numero_inteiro(v: f64) -> String {
    format!("{}", v.round() as i64)
}

/// Cruzetas e a graduação com as coordenadas (E embaixo, N à esquerda).
fn malha(p: &mut Pincel, j: &Janela) {
    let passo = passo_bonito(j.max.x - j.min.x, 6.0);
    let (ht, braco) = (2.2 * K, 2.5 * K);
    p.em("PLANTA_MALHA", Some(0x80_80_80));
    let (x0, y0) = ((j.min.x / passo).ceil() * passo, (j.min.y / passo).ceil() * passo);
    let mut x = x0;
    while x <= j.max.x {
        let mut y = y0;
        while y <= j.max.y {
            let c = j.p(Ponto::novo(x, y));
            p.linha(Ponto::novo(c.x - braco, c.y), Ponto::novo(c.x + braco, c.y));
            p.linha(Ponto::novo(c.x, c.y - braco), Ponto::novo(c.x, c.y + braco));
            y += passo;
        }
        p.texto(Ponto::novo(j.p(Ponto::novo(x, j.min.y)).x, j.org.y - 1.2 * K), ht, 0.0, &format!("E {}", numero_inteiro(x)), AlinhaH::Centro, AlinhaV::Topo);
        x += passo;
    }
    let mut y = y0;
    while y <= j.max.y {
        p.texto(Ponto::novo(j.org.x - 1.2 * K, j.p(Ponto::novo(j.min.x, y)).y), ht, 90.0, &format!("N {}", numero_inteiro(y)), AlinhaH::Centro, AlinhaV::Base);
        y += passo;
    }
}

/// Marcas de estaca a cada 20 m, rótulo a cada 100 m e o quilômetro destacado.
fn estacas(p: &mut Pincel, a: &Alinhamento, t: &Trecho, j: &Janela) {
    p.em("PLANTA_ESTACAS", Some(0x80_80_80));
    let mut s = (t.de / 20.0).ceil() * 20.0;
    while s <= t.ate + 1e-9 {
        let am = a.em(s);
        if j.dentro(am.ponto) {
            let cem = (s / 100.0 - (s / 100.0).round()).abs() < 1e-6;
            let km = (s / 1000.0 - (s / 1000.0).round()).abs() < 1e-6;
            let (sn, cs) = am.direcao.to_radians().sin_cos();
            let normal = (-sn, cs);
            let c = j.p(am.ponto);
            let meio = if km { 3.0 } else if cem { 1.8 } else { 1.0 } * K;
            p.linha(Ponto::novo(c.x - normal.0 * meio, c.y - normal.1 * meio), Ponto::novo(c.x + normal.0 * meio, c.y + normal.1 * meio));
            if cem {
                let (giro, h) = legivel(am.direcao + 90.0);
                let d = 2.6 * K;
                p.texto(Ponto::novo(c.x + normal.0 * d, c.y + normal.1 * d), 2.2 * K, giro, &estaca_texto(s, 20.0), h, AlinhaV::Meio);
            }
            if km {
                p.em("PLANTA_KM", Some(0xFF_7F_DF));
                let (giro, h) = legivel(am.direcao - 90.0);
                let d = 4.2 * K;
                p.texto(Ponto::novo(c.x - normal.0 * d, c.y - normal.1 * d), 3.0 * K, giro, &format!("KM {}", km_texto(s)), h, AlinhaV::Meio);
                p.em("PLANTA_ESTACAS", Some(0x80_80_80));
            }
        }
        s += 20.0;
    }
}

/// TS, SC, CS e ST das curvas do trecho: círculo e rótulo com a estaca.
fn notaveis(p: &mut Pincel, a: &Alinhamento, j: &Janela) {
    p.em("PLANTA_NOTAVEIS", Some(0xFF_7F_9F));
    for (n, c) in a.curvas.iter().enumerate() {
        for (nome, m) in [("TS", &c.ts_marco), ("SC", &c.sc), ("CS", &c.cs), ("ST", &c.st)] {
            if !j.dentro(m.ponto) {
                continue;
            }
            let am = a.em(m.progressiva);
            let (sn, cs) = am.direcao.to_radians().sin_cos();
            let centro = j.p(m.ponto);
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro, raio: 0.9 * K } });
            let (giro, h) = legivel(am.direcao - 90.0);
            let d = 3.4 * K;
            p.texto(Ponto::novo(centro.x + sn * d, centro.y - cs * d), 2.2 * K, giro, &format!("{nome}{} {}", n + 1, estaca_texto(m.progressiva, 20.0)), h, AlinhaV::Meio);
        }
    }
}

/// Ripagem ampliada 20 vezes: do eixo de projeto até onde o existente estaria com o desvio multiplicado (o desvio real, de
/// decímetros, seria menor que a espessura do traço em 1:2000).
fn ripagem(p: &mut Pincel, a: &Alinhamento, tr: &Trajeto, existente: &[Vec<Ponto>], j: &Janela) {
    p.em("PLANTA_RIPAGEM", Some(0x7F_FF_00));
    for q in existente.iter().flatten().filter(|q| j.dentro(**q)) {
        let (d, s) = tr.projetar(*q);
        let proj = a.em(a.progressiva_inicial + s).ponto;
        if d.abs() < 0.05 || !j.dentro(proj) {
            continue;
        }
        p.linha(j.p(proj), j.p(Ponto::novo(proj.x + (q.x - proj.x) * 20.0, proj.y + (q.y - proj.y) * 20.0)));
    }
}

/// Cores das faixas de cota da nuvem (as mesmas da folha da nuvem de pontos).
const PALETA_DA_NUVEM: [u32; 6] = [0x30_60_D0, 0x20_A0_B0, 0x30_A0_40, 0xB0_90_00, 0xE0_70_00, 0xD0_30_30];

/// Os pontos do levantamento e os limites (quantis) das 6 faixas de cota.
pub(crate) struct Nuvem<'a> {
    pub pontos: &'a [Ponto],
    pub limites: [f64; 5],
    /// A malha triangular (a mesma do leitor de nuvem) e as curvas de nível.
    pub triangulos: Vec<feathercad_core::topografia::Triangulo>,
    pub curvas: Vec<feathercad_core::topografia::CurvaNivel>,
}

impl<'a> Nuvem<'a> {
    pub fn nova(pontos: &'a [Ponto]) -> Self {
        let mut z: Vec<f64> = pontos.iter().map(|p| p.z).collect();
        z.sort_by(f64::total_cmp);
        let q = |k: usize| z.get(z.len() * k / 6).copied().unwrap_or(0.0);
        let desnivel = z.last().copied().unwrap_or(0.0) - z.first().copied().unwrap_or(0.0);
        let triangulos = feathercad_core::topografia::triangular_delaunay(pontos, None);
        let curvas = feathercad_core::topografia::gerar_curvas_nivel(pontos, &triangulos, feathercad_core::topografia::equidistancia_padrao(desnivel), 5);
        Self { pontos, limites: [q(1), q(2), q(3), q(4), q(5)], triangulos, curvas }
    }

    fn faixa(&self, z: f64) -> usize {
        self.limites.iter().position(|l| z < *l).unwrap_or(5)
    }
}

/// A superfície triangulada (arestas do TIN) e as curvas de nível, cortadas pela janela; as mestras levam a cota.
fn superficie(p: &mut Pincel, n: &Nuvem, j: &Janela) {
    p.em("PLANTA_TIN", Some(0x90_90_90));
    let mut vistas = std::collections::HashSet::new();
    for t in &n.triangulos {
        for (u, v) in t.arestas() {
            if vistas.insert((u, v)) {
                polilinhas(p, j, &[n.pontos[u], n.pontos[v]]);
            }
        }
    }
    for c in &n.curvas {
        p.em(if c.eh_mestra { "PLANTA_CURVA_MESTRA" } else { "PLANTA_CURVA_NIVEL" }, Some(if c.eh_mestra { 0xA0_50_20 } else { 0xC0_80_50 }));
        let mut pts: Vec<Ponto> = c.pontos.clone();
        if c.fechada {
            pts.push(pts[0]);
        }
        polilinhas(p, j, &pts);
        if let Some(m) = pts.iter().step_by(1).find(|q| j.dentro(**q)).filter(|_| c.eh_mestra) {
            p.texto(Ponto::novo(j.p(*m).x + 1.0 * K, j.p(*m).y + 1.0 * K), 2.0 * K, 0.0, &format!("{:.0}", c.cota), AlinhaH::Esquerda, AlinhaV::Base);
        }
    }
}

/// Os pontos da nuvem na janela, por faixa de cota, e a cota (1 casa) a cada ~80 m de distância entre rótulos.
fn pontos_da_nuvem(p: &mut Pincel, n: &Nuvem, j: &Janela) {
    for (faixa, cor) in PALETA_DA_NUVEM.iter().enumerate() {
        p.em(&format!("PLANTA_NUVEM_{}", faixa + 1), Some(*cor));
        for q in n.pontos.iter().filter(|q| j.dentro(**q) && n.faixa(q.z) == faixa) {
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro: j.p(*q), raio: 0.55 * K } });
        }
    }
    p.em("PLANTA_NUVEM_COTAS", Some(0x80_80_80));
    let mut ultimo: Option<Ponto> = None;
    for q in n.pontos.iter().filter(|q| j.dentro(**q)) {
        if ultimo.map_or(true, |u| u.distancia(*q) > 80.0) {
            p.texto(Ponto::novo(j.p(*q).x + 1.2 * K, j.p(*q).y - 0.8 * K), 1.8 * K, 0.0, &format!("{:.1}", q.z).replace('.', ","), AlinhaH::Esquerda, AlinhaV::Topo);
            ultimo = Some(*q);
        }
    }
}

/// Raios das curvas: do centro até o SC, o meio do arco e o CS, com "R = …" perto do arco (a parte fora da janela é cortada).
fn raios(p: &mut Pincel, a: &Alinhamento, j: &Janela) {
    p.em("PLANTA_RAIOS", Some(0x00_A0_C0));
    for c in &a.curvas {
        let sc = a.em(c.sc.progressiva);
        let (sn, cs) = sc.direcao.to_radians().sin_cos();
        let lado = f64::from(c.lado);
        let centro = Ponto::novo(c.sc.ponto.x - sn * c.raio * lado, c.sc.ponto.y + cs * c.raio * lado);
        let meio = a.em((c.sc.progressiva + c.cs.progressiva) / 2.0).ponto;
        for alvo in [c.sc.ponto, meio, c.cs.ponto] {
            for parte in j.recortar(&[centro, alvo]) {
                p.polilinha(&parte);
            }
        }
        if j.dentro(centro) {
            let m = j.p(centro);
            p.linha(Ponto::novo(m.x - 1.5 * K, m.y), Ponto::novo(m.x + 1.5 * K, m.y));
            p.linha(Ponto::novo(m.x, m.y - 1.5 * K), Ponto::novo(m.x, m.y + 1.5 * K));
        }
        if j.dentro(meio) {
            let (dx, dy) = (centro.x - meio.x, centro.y - meio.y);
            let d = dx.hypot(dy).max(1e-9);
            let pos = j.p(Ponto::novo(meio.x + dx / d * 9.0 * K, meio.y + dy / d * 9.0 * K));
            let (giro, h) = legivel(dy.atan2(dx).to_degrees());
            p.texto(pos, 2.4 * K, giro, &format!("R = {:.0} m", c.raio), h, AlinhaV::Meio);
        }
    }
}

/// O mapa da folha, tudo numa gravação só.
fn mapa(ag: &mut Agente, a: &Alinhamento, existente: &[Vec<Ponto>], nuvem: &Nuvem, t: &Trecho, j: &Janela) -> Res<()> {
    let mut p = Pincel::novo("PLANTA_MOLDURA");
    p.retangulo(j.org, j.p(j.max));
    malha(&mut p, j);
    superficie(&mut p, nuvem, j);
    pontos_da_nuvem(&mut p, nuvem, j);
    p.em("PLANTA_EXISTENTE", Some(0xFF_7F_00));
    for seg in existente {
        polilinhas(&mut p, j, seg);
    }
    let eixo = a.amostrar(1.0);
    if let Some(tr) = Trajeto::novo(&eixo) {
        ripagem(&mut p, a, &tr, existente, j);
        p.em("PLANTA_TRILHOS", Some(0x7F_9F_FF));
        for d in [0.5375, -0.5375] {
            polilinhas(&mut p, j, &tr.paralela(d));
        }
    }
    p.em("PLANTA_PROJETO", Some(0xFF_00_FF));
    polilinhas(&mut p, j, &eixo);
    raios(&mut p, a, j);
    estacas(&mut p, a, t, j);
    notaveis(&mut p, a, j);
    aplicar(&mut ag.doc, p.pecas).map(|_| ())
}

fn pf(f: &InfoFolha, x_mm: f64, y_mm: f64) -> Ponto {
    Ponto::novo(f.min.x + x_mm * K, f.min.y + y_mm * K)
}

fn par_de(p: Ponto) -> Value {
    json!([p.x, p.y])
}

/// Título, tabelas das curvas do trecho, legenda e chave de articulação.
struct Coluna<'a> {
    a: &'a Alinhamento,
    tabela: &'a [ResultadoCurva],
    trechos: &'a [Trecho],
    idx: usize,
    f: &'a InfoFolha,
    nuvem: &'a Nuvem<'a>,
}

fn titulo_da_folha(ag: &mut Agente, fonte: &Fonte, c: &Coluna) -> Res<()> {
    let t = &c.trechos[c.idx];
    let subtitulo = format!("EST {} a {}  —  KM {} a {}  —  ESC. 1:{}", estaca_texto(t.de, 20.0), estaca_texto(t.ate, 20.0), km_texto(t.de), km_texto(t.ate), ESCALA as i64);
    passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": par_de(pf(c.f, X_COLUNA, 572.0)), "escala": ESCALA, "largura": LARGURA_COLUNA, "altura_texto": 3.4,
        "itens": [{ "tipo": "titulo", "texto": format!("PLANTA E CURVAS — FOLHA {:02} ({:02}/{:02})", c.idx + 1, c.idx + 1, c.trechos.len()) }, { "tipo": "texto", "texto": subtitulo }] }))
        .map(|_| ())
}

fn tabelas_das_curvas(ag: &mut Agente, fonte: &Fonte, c: &Coluna) -> Res<()> {
    let t = &c.trechos[c.idx];
    let nelas: Vec<usize> = (0..c.a.curvas.len()).filter(|&i| c.a.curvas[i].ts_marco.progressiva <= t.ate && c.a.curvas[i].st.progressiva >= t.de).take(6).collect();
    if nelas.is_empty() {
        return passo(ag, fonte, "cad_texto_bloco", &json!({ "origem": par_de(pf(c.f, X_COLUNA, 538.0)), "escala": ESCALA, "largura": LARGURA_COLUNA, "altura_texto": 3.0, "itens": [{ "tipo": "nota", "texto": "Trecho em tangente: sem curvas horizontais." }] })).map(|_| ());
    }
    let col = |titulo: &str, l: f64, d: usize| json!({ "titulo": titulo, "largura": l, "alinhamento": "centro", "decimais": d });
    let linhas: Vec<Value> = nelas.iter().map(|&i| { let cv = &c.a.curvas[i]; json!([i + 1, if cv.lado > 0 { "E" } else { "D" }, cv.delta_graus, cv.raio, cv.ls, cv.lc, c.tabela[i].h_mm]) }).collect();
    passo(ag, fonte, "cad_tabela", &json!({ "origem": par_de(pf(c.f, X_COLUNA, 540.0)), "escala": ESCALA, "titulo": "ELEMENTOS DAS CURVAS", "altura_linha": 5.2, "altura_texto": 2.8,
        "colunas": [col("Curva", 20.0, 0), col("Lado", 16.0, 0), col("Δ (°)", 28.0, 2), col("R (m)", 26.0, 0), col("Ls (m)", 24.0, 0), col("Lc (m)", 28.0, 1), col("h (mm)", 22.0, 0)], "linhas": linhas }))?;
    let pontos: Vec<Value> = nelas.iter().map(|&i| { let cv = &c.a.curvas[i]; let e = |m: f64| json!(estaca_texto(m, 20.0)); json!([i + 1, e(cv.ts_marco.progressiva), e(cv.sc.progressiva), e(cv.cs.progressiva), e(cv.st.progressiva)]) }).collect();
    passo(ag, fonte, "cad_tabela", &json!({ "origem": par_de(pf(c.f, X_COLUNA, 490.0)), "escala": ESCALA, "titulo": "PONTOS NOTÁVEIS (ESTACA)", "altura_linha": 5.2, "altura_texto": 2.8,
        "colunas": [col("Curva", 20.0, 0), col("TS", 40.0, 0), col("SC", 40.0, 0), col("CS", 40.0, 0), col("ST", 41.0, 0)], "linhas": pontos })).map(|_| ())
}

/// Cor e rótulo de cada coisa desenhada no mapa.
fn legenda(ag: &mut Agente, c: &Coluna) -> Res<()> {
    let mut p = Pincel::novo("PLANTA_LEGENDA");
    p.texto(pf(c.f, X_COLUNA, 440.0), 3.0 * K, 0.0, "LEGENDA", AlinhaH::Esquerda, AlinhaV::Base);
    let itens: [(u32, &str, bool); 9] = [(0x90_90_90, "Malha triangular (TIN) da nuvem de pontos", false), (0xA0_50_20, "Curva de nível mestra (cota em m) / secundária mais clara", false), (0x00_A0_C0, "Raio da curva (do centro ao SC, ao meio e ao CS)", false), (0xFF_7F_00, "Eixo existente (ponto médio de cada par de trilhos)", false), (0x7F_FF_00, "Ripagem ampliada 20 vezes (projeto → existente)", false), (0xFF_00_FF, "Eixo de projeto (tangentes, espirais e circulares)", false), (0x7F_9F_FF, "Trilhos de projeto (bitola métrica)", false), (0x80_80_80, "Estaca de 20 m (rótulo a cada 100 m); KM em destaque", false), (0xFF_7F_9F, "Ponto notável: TS, SC, CS, ST", true)];
    for (n, (cor, texto, circulo)) in itens.iter().enumerate() {
        let y = 431.0 - n as f64 * 7.0;
        p.em("PLANTA_LEGENDA", Some(*cor));
        if *circulo {
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro: pf(c.f, X_COLUNA + 6.0, y), raio: 0.9 * K } });
        } else {
            p.linha(pf(c.f, X_COLUNA, y), pf(c.f, X_COLUNA + 12.0, y));
        }
        p.em("PLANTA_LEGENDA", None);
        p.texto(pf(c.f, X_COLUNA + 16.0, y), 2.4 * K, 0.0, texto, AlinhaH::Esquerda, AlinhaV::Meio);
    }
    faixas_da_nuvem(&mut p, c);
    aplicar(&mut ag.doc, p.pecas).map(|_| ())
}

/// A legenda das faixas de cota dos pontos da nuvem (cor = faixa).
fn faixas_da_nuvem(p: &mut Pincel, c: &Coluna) {
    let y = 431.0 - 9.0 * 7.0;
    p.em("PLANTA_LEGENDA", None);
    p.texto(pf(c.f, X_COLUNA, y), 2.4 * K, 0.0, "Pontos da nuvem do levantamento, por faixa de cota (m):", AlinhaH::Esquerda, AlinhaV::Meio);
    let l = c.nuvem.limites;
    for (faixa, cor) in PALETA_DA_NUVEM.iter().enumerate() {
        let (x, yy) = (X_COLUNA + faixa as f64 * 30.0, y - 5.5);
        p.em("PLANTA_LEGENDA", Some(*cor));
        p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro: pf(c.f, x + 2.0, yy), raio: 0.9 * K } });
        let rotulo = match faixa {
            0 => format!("< {:.1}", l[0]),
            5 => format!("> {:.1}", l[4]),
            k => format!("{:.1}–{:.1}", l[k - 1], l[k]),
        }
        .replace('.', ",");
        p.em("PLANTA_LEGENDA", None);
        p.texto(pf(c.f, x + 4.5, yy), 2.0 * K, 0.0, &rotulo, AlinhaH::Esquerda, AlinhaV::Meio);
    }
}

/// A miniatura do traçado todo com a janela de cada folha; a desta fica em destaque.
fn chave(ag: &mut Agente, c: &Coluna) -> Res<()> {
    let eixo = c.a.amostrar(10.0);
    let (mut min, mut max) = (eixo[0], eixo[0]);
    for q in &eixo {
        (min, max) = (Ponto::novo(min.x.min(q.x), min.y.min(q.y)), Ponto::novo(max.x.max(q.x), max.y.max(q.y)));
    }
    let (w_mapa, h_mapa) = ((MAPA.2 - MAPA.0) * K, (MAPA.3 - MAPA.1) * K);
    for t in c.trechos {
        min = Ponto::novo(min.x.min(t.centro.x - w_mapa / 2.0), min.y.min(t.centro.y - h_mapa / 2.0));
        max = Ponto::novo(max.x.max(t.centro.x + w_mapa / 2.0), max.y.max(t.centro.y + h_mapa / 2.0));
    }
    let (caixa_w, caixa_h) = ((LARGURA_COLUNA - 8.0) * K, 205.0 * K);
    let escala = (caixa_w / (max.x - min.x).max(1.0)).min(caixa_h / (max.y - min.y).max(1.0));
    let origem = pf(c.f, X_COLUNA + 4.0, 112.0);
    let q = |p: Ponto| Ponto::novo(origem.x + (p.x - min.x) * escala, origem.y + (p.y - min.y) * escala);
    let mut p = Pincel::novo("PLANTA_LEGENDA");
    p.texto(pf(c.f, X_COLUNA, 326.0), 3.0 * K, 0.0, "CHAVE DE ARTICULAÇÃO", AlinhaH::Esquerda, AlinhaV::Base);
    p.em("PLANTA_LEGENDA", Some(0xFF_00_FF));
    p.polilinha(&eixo.iter().map(|e| q(*e)).collect::<Vec<_>>());
    for (i, t) in c.trechos.iter().enumerate() {
        let (a, b) = (q(Ponto::novo(t.centro.x - w_mapa / 2.0, t.centro.y - h_mapa / 2.0)), q(Ponto::novo(t.centro.x + w_mapa / 2.0, t.centro.y + h_mapa / 2.0)));
        p.em("PLANTA_LEGENDA", if i == c.idx { Some(0xFF_30_30) } else { Some(0x80_80_80) });
        p.retangulo(a, b);
        p.texto(Ponto::novo(a.x + 0.8 * K, b.y - 0.8 * K), 2.4 * K, 0.0, &format!("{}", i + 1), AlinhaH::Esquerda, AlinhaV::Topo);
    }
    aplicar(&mut ag.doc, p.pecas).map(|_| ())
}

/// Todas as folhas de planta (`primeira` = o número da primeira folha no carimbo).
pub(crate) fn plantas(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, dados: (&Alinhamento, &[ResultadoCurva], &[Vec<Ponto>], &Nuvem), primeira: usize, trechos: &[Trecho]) -> Res<()> {
    let (a, tabela, existente, nuvem) = dados;
    for (i, t) in trechos.iter().enumerate() {
        let f = inserir_folha(ag, fonte, ctx, "A1", Some(ESCALA), "Planta e curvas", primeira + i)?;
        mapa(ag, a, existente, nuvem, t, &Janela::nova(t, &f))?;
        let c = Coluna { a, tabela, trechos, idx: i, f: &f, nuvem };
        titulo_da_folha(ag, fonte, &c)?;
        tabelas_das_curvas(ag, fonte, &c)?;
        legenda(ag, &c)?;
        chave(ag, &c)?;
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::viario::alinhamento::EntradaCurva;

    #[test]
    fn divide_o_tracado_em_trechos_que_cabem_no_mapa() {
        let d = 70f64.to_radians();
        let a = Alinhamento::novo(&[Ponto::novo(0.0, 0.0), Ponto::novo(3000.0, 0.0), Ponto::novo(3000.0 + 2000.0 * d.cos(), 2000.0 * d.sin())], &[Some(EntradaCurva { raio: 400.0, ls: 70.0 })], 0.0).unwrap();
        let ts = dividir(&a);
        assert!(ts.len() >= 4, "{} trechos", ts.len());
        assert!((ts[0].de - a.progressiva_inicial).abs() < 1e-9 && (ts.last().unwrap().ate - a.progressiva_final()).abs() < 1e-6);
        assert!(ts.windows(2).all(|w| (w[0].ate - w[1].de).abs() < 1e-9), "os trechos se encostam");
        let (w, h) = ((MAPA.2 - MAPA.0) * K, (MAPA.3 - MAPA.1) * K);
        for t in &ts {
            let (a0, a1) = (a.em(t.de).ponto, a.em(t.ate).ponto);
            assert!((a0.x - t.centro.x).abs() <= w / 2.0 && (a1.y - t.centro.y).abs() <= h / 2.0);
        }
    }
}
