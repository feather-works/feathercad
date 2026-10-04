//! O perfil longitudinal do projeto ferroviário, em folhas A1 (escala horizontal 1:2000; a vertical é escolhida para o perfil
//! caber, 1:200 no projeto de referência). Cada folha tem o gráfico (terreno existente × greide, com **aterro** e **escavação**
//! hachurados, PIVs e curvas verticais), a tabela clássica logo abaixo (estaca, cotas, aterro/escavação, rampa, alinhamento
//! horizontal, superelevação e criticidade), o diagrama de levante e a lista dos pontos de atenção do trecho.

use feathercad_core::documento::{AlinhaH, AlinhaV};
use feathercad_core::trajeto::{estaca_texto, km_texto};
use feathercad_core::viario::criticidade::{Elemento, Nivel, Tipo};
use feathercad_core::viario::superelevacao::h_em;

use crate::agente::Agente;
use crate::desenho::{cor_do_nivel, Caneta, Mm, COR_ATERRO, COR_ESCAVACAO, COR_EXISTENTE, COR_GRADE, COR_PROJETO};
use crate::fonte::Fonte;
use crate::json::Res;
use crate::modelos::{inserir_folha, Contexto};
use crate::projeto::Projeto;
use crate::texto_bloco::quebrar;

const ESCALA_H: f64 = 2000.0;
/// Início do gráfico na folha (mm): à esquerda ficam os títulos das linhas da tabela.
const X0: f64 = 82.0;
/// Comprimento máximo de traçado por folha, m (782 mm úteis − o recuo de 82 mm).
const COMPRIMENTO_POR_FOLHA: f64 = 1440.0;
const Y_BASE: f64 = 405.0;
const LINHA: f64 = 8.0;

/// Quantas folhas de perfil o traçado pede.
pub(crate) fn numero_de_folhas(comprimento: f64) -> usize {
    ((comprimento / COMPRIMENTO_POR_FOLHA).ceil() as usize).max(1)
}

/// A escala vertical (denominador) mais detalhada em que o perfil, com folga de 1 m, cabe em 125 mm: cada folha tem a sua, dita no título.
fn escala_vertical(zmin: f64, zmax: f64) -> f64 {
    [50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0].into_iter().find(|v| (zmax - zmin + 1.0) * 1000.0 / v <= 125.0).unwrap_or(5000.0)
}

struct Quadro {
    c0: f64,
    c1: f64,
    z0: f64,
    esc_v: f64,
    altura: f64,
}

impl Quadro {
    fn x(&self, s: f64) -> f64 {
        X0 + (s - self.c0) * 1000.0 / ESCALA_H
    }
    fn y(&self, z: f64) -> f64 {
        Y_BASE + (z - self.z0) * 1000.0 / self.esc_v
    }
    fn x_fim(&self) -> f64 {
        self.x(self.c1)
    }
}

fn virgula(v: f64, casas: usize) -> String {
    format!("{v:.casas$}").replace('.', ",")
}

type Amostra3 = (f64, f64, f64);
type Faixa = (bool, Vec<Amostra3>);

/// Uma faixa contínua entre duas curvas onde `a` está de um lado de `b`: pontos `(s, a, b)` de uma mesma classe.
fn faixas(pontos: &[Amostra3], minimo: f64, passo: f64) -> Vec<Faixa> {
    let mut saida: Vec<Faixa> = Vec::new();
    for p in pontos {
        let d = p.1 - p.2;
        if d.abs() < minimo {
            continue;
        }
        let acima = d > 0.0;
        match saida.last_mut() {
            Some((c, v)) if *c == acima && p.0 - v.last().map_or(0.0, |u| u.0) <= passo * 1.05 => v.push(*p),
            _ => saida.push((acima, vec![*p])),
        }
    }
    saida.retain(|(_, v)| v.len() >= 2);
    saida
}

fn grade(c: &mut Caneta, q: &Quadro, z1: f64) {
    c.em("PERFIL_GRADE", Some(COR_GRADE));
    let mut z = q.z0.ceil();
    while z <= z1 + 1e-9 {
        c.linha((X0, q.y(z)), (q.x_fim(), q.y(z)));
        c.texto((X0 - 1.5, q.y(z)), 2.2, 0.0, &virgula(z, 0), AlinhaH::Direita, AlinhaV::Meio);
        c.texto((q.x_fim() + 1.5, q.y(z)), 2.2, 0.0, &virgula(z, 0), AlinhaH::Esquerda, AlinhaV::Meio);
        z += 1.0;
    }
    let mut s = (q.c0 / 100.0).ceil() * 100.0;
    while s <= q.c1 + 1e-9 {
        c.linha((q.x(s), Y_BASE), (q.x(s), Y_BASE + q.altura));
        s += 100.0;
    }
    c.em("PERFIL_MOLDURA", None);
    c.ret((X0, Y_BASE), (q.x_fim(), Y_BASE + q.altura));
}

/// Aterro (greide acima do existente) e escavação (abaixo), hachurados entre as duas linhas.
fn preencher(c: &mut Caneta, q: &Quadro, p: &Projeto) {
    let passo = 2.0;
    let n = ((q.c1 - q.c0) / passo).ceil() as usize;
    let pts: Vec<(f64, f64, f64)> = (0..=n).filter_map(|i| { let s = (q.c0 + i as f64 * passo).min(q.c1); p.ze(s).map(|ze| (s, p.zg(s), ze)) }).collect();
    for (acima, v) in faixas(&pts, 0.01, passo) {
        let mut contorno: Vec<Mm> = v.iter().map(|(s, zg, _)| (q.x(*s), q.y(*zg))).collect();
        contorno.extend(v.iter().rev().map(|(s, _, ze)| (q.x(*s), q.y(*ze))));
        c.em(if acima { "PERFIL_ATERRO" } else { "PERFIL_ESCAVACAO" }, Some(if acima { COR_ATERRO } else { COR_ESCAVACAO }));
        c.hachura(&contorno, if acima { "ANSI31" } else { "ANSI37" }, 1.1);
    }
}

fn linhas_do_perfil(c: &mut Caneta, q: &Quadro, p: &Projeto) {
    let n = ((q.c1 - q.c0) / 5.0).ceil() as usize;
    let existente: Vec<Mm> = (0..=n).filter_map(|i| { let s = (q.c0 + i as f64 * 5.0).min(q.c1); p.ze(s).map(|z| (q.x(s), q.y(z))) }).collect();
    c.em("PERFIL_EXISTENTE", Some(COR_EXISTENTE));
    c.poli(&existente);
    let m = ((q.c1 - q.c0) / 2.0).ceil() as usize;
    let greide: Vec<Mm> = (0..=m).map(|i| { let s = (q.c0 + i as f64 * 2.0).min(q.c1); (q.x(s), q.y(p.zg(s))) }).collect();
    c.em("PERFIL_GREIDE", Some(COR_PROJETO));
    c.poli(&greide);
}

/// PIVs e curvas verticais no gráfico: traço vertical e o rótulo girado acima do quadro.
fn pivs(c: &mut Caneta, q: &Quadro, p: &Projeto) {
    let p0 = p.a.progressiva_inicial;
    for (i, piv) in p.greide.pivs.iter().enumerate() {
        let s = p0 + piv.s;
        if s < q.c0 || s > q.c1 {
            continue;
        }
        let x = q.x(s);
        c.em("PERFIL_PIV", Some(COR_PROJETO));
        c.linha((x, q.y(piv.z)), (x, Y_BASE + q.altura + 4.0));
        let estaca = format!("{}+{:02.0}", (s / 20.0).floor() as i64, s - (s / 20.0).floor() * 20.0);
        let rotulo = format!("PIV {} · {} · {} m", i + 1, estaca, virgula(piv.z, 2));
        c.texto((x - 0.8, Y_BASE + q.altura + 5.0), 2.1, 90.0, &rotulo, AlinhaH::Esquerda, AlinhaV::Meio);
        if piv.lv > 0.0 {
            c.linha((q.x(s - piv.lv / 2.0), q.y(piv.z) - 3.0), (q.x(s + piv.lv / 2.0), q.y(piv.z) - 3.0));
        }
    }
}

fn estacoes(q: &Quadro) -> Vec<f64> {
    let mut v = Vec::new();
    let mut s = (q.c0 / 20.0).ceil() * 20.0;
    while s <= q.c1 + 1e-9 {
        v.push(s);
        s += 20.0;
    }
    v
}

fn nome_do_elemento(e: &Elemento) -> String {
    match e.tipo {
        Tipo::Curva(i) => format!("C{}", i + 1),
        Tipo::Tangente(i) => format!("T{}", i + 1),
    }
}

/// A tabela clássica sob o gráfico. Devolve o y da última linha.
fn tabela(c: &mut Caneta, q: &Quadro, p: &Projeto) -> f64 {
    let titulos = ["ESTACA", "COTA DO TERRENO (m)", "COTA DO GREIDE (m)", "ATERRO (+) / ESCAVAÇÃO (−) (m)", "RAMPA DO GREIDE", "ALINHAMENTO HORIZONTAL", "SUPERELEVAÇÃO h (mm)", "CRITICIDADE"];
    let topo = Y_BASE - 5.0;
    c.em("PERFIL_TABELA", None);
    for (i, t) in titulos.iter().enumerate() {
        let y = topo - i as f64 * LINHA;
        c.linha((35.0, y), (q.x_fim(), y));
        c.texto((36.5, y - LINHA / 2.0), 2.2, 0.0, t, AlinhaH::Esquerda, AlinhaV::Meio);
    }
    let fundo = topo - titulos.len() as f64 * LINHA;
    c.linha((35.0, fundo), (q.x_fim(), fundo));
    c.linha((35.0, topo), (35.0, fundo));
    c.linha((X0, topo), (X0, fundo));
    c.linha((q.x_fim(), topo), (q.x_fim(), fundo));
    let centro_de = |linha: usize| topo - (linha as f64 + 0.5) * LINHA;
    for s in estacoes(q) {
        let x = q.x(s);
        c.em("PERFIL_TABELA", None);
        let cem = (s / 100.0 - (s / 100.0).round()).abs() < 1e-6;
        c.texto((x, centro_de(0)), 2.1, 90.0, &if cem { format!("{}+00", (s / 20.0).round() as i64) } else { format!("{}", (s / 20.0).round() as i64) }, AlinhaH::Centro, AlinhaV::Meio);
        if let Some(ze) = p.ze(s) {
            let (zg, lev) = (p.zg(s), p.zg(s) - ze);
            c.texto((x, centro_de(1)), 2.1, 90.0, &virgula(ze, 2), AlinhaH::Centro, AlinhaV::Meio);
            c.texto((x, centro_de(2)), 2.1, 90.0, &virgula(zg, 2), AlinhaH::Centro, AlinhaV::Meio);
            c.em("PERFIL_TABELA", Some(if lev >= 0.0 { COR_ATERRO } else { COR_ESCAVACAO }));
            c.texto((x, centro_de(3)), 2.1, 90.0, &format!("{}{}", if lev >= 0.0 { "+" } else { "−" }, virgula(lev.abs(), 2)), AlinhaH::Centro, AlinhaV::Meio);
        }
        let h = h_em(p.a, p.tabela, s);
        if h > 0.5 {
            c.em("PERFIL_TABELA", None);
            c.texto((x, centro_de(6)), 2.1, 90.0, &virgula(h, 0), AlinhaH::Centro, AlinhaV::Meio);
        }
    }
    rampas(c, q, p, centro_de(4));
    horizontal(c, q, p, (centro_de(5), LINHA));
    criticidade(c, q, p, centro_de(7));
    fundo
}

fn rampas(c: &mut Caneta, q: &Quadro, p: &Projeto, y: f64) {
    let p0 = p.a.progressiva_inicial;
    c.em("PERFIL_TABELA", Some(COR_PROJETO));
    for j in p.greide.nos_s.windows(2).zip(p.greide.nos_z.windows(2)) {
        let (s0, s1) = (p0 + j.0[0], p0 + j.0[1]);
        let g = (j.1[1] - j.1[0]) / (j.0[1] - j.0[0]).max(1e-9) * 100.0;
        let (a, b) = (s0.max(q.c0), s1.min(q.c1));
        if b - a < 30.0 {
            continue;
        }
        let xm = q.x((a + b) / 2.0);
        c.texto((xm, y + 1.6), 2.3, 0.0, &format!("{}{}%", if g >= 0.0 { "+" } else { "−" }, virgula(g.abs(), 3)), AlinhaH::Centro, AlinhaV::Meio);
        c.texto((xm, y - 1.7), 2.0, 0.0, &format!("L = {:.0} m", s1 - s0), AlinhaH::Centro, AlinhaV::Meio);
        c.linha((q.x(a), y - 3.6), (q.x(b), y - 3.6));
    }
}

/// Tangente = traço fino; espirais = faixa fina; trecho circular = faixa cheia, com R, Ls, Lc e Δ.
fn horizontal(c: &mut Caneta, q: &Quadro, p: &Projeto, (y, altura): (f64, f64)) {
    c.em("PERFIL_HORIZONTAL", None);
    c.linha((q.x(q.c0), y), (q.x(q.c1), y));
    for (i, cv) in p.a.curvas.iter().enumerate() {
        if cv.st.progressiva < q.c0 || cv.ts_marco.progressiva > q.c1 {
            continue;
        }
        let lim = |s: f64| s.clamp(q.c0, q.c1);
        let (ts, sc, cs, st) = (lim(cv.ts_marco.progressiva), lim(cv.sc.progressiva), lim(cv.cs.progressiva), lim(cv.st.progressiva));
        let (m, g) = (altura * 0.18, altura * 0.38);
        let lado = f64::from(cv.lado);
        c.em("PERFIL_HORIZONTAL", Some(0xE0_B0_00));
        c.fechada(&[(q.x(ts), y), (q.x(sc), y + lado * m), (q.x(sc), y), ]);
        c.fechada(&[(q.x(cs), y), (q.x(cs), y + lado * m), (q.x(st), y)]);
        c.em("PERFIL_HORIZONTAL", Some(0xE0_50_30));
        c.ret((q.x(sc), y), (q.x(cs), y + lado * g));
        c.em("PERFIL_HORIZONTAL", None);
        c.texto((q.x((ts + st) / 2.0), y - lado * altura * 0.3), 2.1, 0.0, &format!("C{} R={:.0} Ls={:.0} Lc={:.0} Δ={:.1}° {}", i + 1, cv.raio, cv.ls, cv.lc, cv.delta_graus, if cv.lado > 0 { "E" } else { "D" }), AlinhaH::Centro, AlinhaV::Meio);
    }
}

fn criticidade(c: &mut Caneta, q: &Quadro, p: &Projeto, y: f64) {
    for e in &p.analise.elementos {
        if e.ate < q.c0 || e.de > q.c1 {
            continue;
        }
        let (a, b) = (q.x(e.de.max(q.c0)), q.x(e.ate.min(q.c1)));
        c.em("PERFIL_CRITICIDADE", Some(cor_do_nivel(e.nivel)));
        c.hachura(&[(a, y - LINHA / 2.0 + 0.6), (b, y - LINHA / 2.0 + 0.6), (b, y + LINHA / 2.0 - 0.6), (a, y + LINHA / 2.0 - 0.6)], "SOLID", 1.0);
        c.em("PERFIL_CRITICIDADE", None);
        if b - a > 14.0 {
            c.centro(((a + b) / 2.0, y), 2.2, &format!("{} {}", nome_do_elemento(e), e.nivel.nome()));
        }
    }
}

/// Levante (greide − existente) como diagrama de áreas: aterro para cima (vermelho), escavação para baixo (azul).
fn diagrama_de_levante(c: &mut Caneta, q: &Quadro, p: &Projeto, y0: f64) -> f64 {
    let (meia, mm_por_m) = (30.0, 100.0);
    c.em("PERFIL_LEVANTE", Some(COR_GRADE));
    c.texto((36.5, y0 + 2.0), 2.4, 0.0, "DIAGRAMA DE LEVANTE (m)", AlinhaH::Esquerda, AlinhaV::Meio);
    c.texto((36.5, y0 - 2.0), 2.0, 0.0, "aterro ↑  /  escavação ↓", AlinhaH::Esquerda, AlinhaV::Meio);
    c.ret((X0, y0 - meia), (q.x_fim(), y0 + meia));
    for m in [-0.3, -0.2, -0.1, 0.0, 0.1, 0.2, 0.3] {
        c.linha((X0, y0 + m * mm_por_m), (q.x_fim(), y0 + m * mm_por_m));
        c.texto((X0 - 1.5, y0 + m * mm_por_m), 2.0, 0.0, &virgula(m, 1), AlinhaH::Direita, AlinhaV::Meio);
    }
    let n = ((q.c1 - q.c0) / 5.0).ceil() as usize;
    let pts: Vec<(f64, f64, f64)> = (0..=n).filter_map(|i| { let s = (q.c0 + i as f64 * 5.0).min(q.c1); p.ze(s).map(|ze| (s, p.zg(s) - ze, 0.0)) }).collect();
    for (acima, v) in faixas(&pts, 0.005, 5.0) {
        let mut contorno: Vec<Mm> = v.iter().map(|(s, l, _)| (q.x(*s), y0 + l.clamp(-0.3, 0.3) * mm_por_m)).collect();
        contorno.extend(v.iter().rev().map(|(s, _, _)| (q.x(*s), y0)));
        c.em("PERFIL_LEVANTE", Some(if acima { COR_ATERRO } else { COR_ESCAVACAO }));
        c.hachura(&contorno, "ANSI31", 0.9);
    }
    y0 - meia
}

/// Os elementos de atenção ou críticos do trecho, com o motivo, em colunas.
fn pontos_de_atencao(c: &mut Caneta, q: &Quadro, p: &Projeto, y_topo: f64) {
    c.em("PERFIL_ATENCAO", None);
    c.rotulo((36.5, y_topo), 3.2, "PONTOS CRÍTICOS E DE ATENÇÃO NESTE TRECHO (aterro e escavação incluídos)");
    let mut y = y_topo - 6.0;
    let (altura, largura) = (2.4, 250.0);
    let mut coluna = 0usize;
    for e in p.analise.elementos.iter().filter(|e| e.nivel > Nivel::Normal && e.ate >= q.c0 && e.de <= q.c1) {
        let x = 36.5 + coluna as f64 * 262.0;
        c.em("PERFIL_ATENCAO", Some(cor_do_nivel(e.nivel)));
        c.rotulo((x, y), altura + 0.4, &format!("{} — {} — est. {} a {}", nome_do_elemento(e), e.nivel.nome(), estaca_texto(e.de, 20.0), estaca_texto(e.ate, 20.0)));
        y -= altura * 1.5;
        c.em("PERFIL_ATENCAO", None);
        for m in e.motivos.iter().filter(|m| m.nivel > Nivel::Normal) {
            for (k, linha) in quebrar(&format!("• {}", m.texto), altura * c.k, largura * c.k).iter().enumerate() {
                c.rotulo((x + if k == 0 { 0.0 } else { 2.0 }, y), altura, linha);
                y -= altura * 1.4;
            }
        }
        y -= altura;
        if y < 125.0 {
            coluna += 1;
            y = y_topo - 6.0;
            if coluna >= 3 {
                break;
            }
        }
    }
}

/// Todas as folhas de perfil. `primeira` = o número da primeira folha no carimbo.
pub(crate) fn perfis(ag: &mut Agente, fonte: &Fonte, ctx: &Contexto, p: &Projeto, primeira: usize) -> Res<()> {
    let (p0, pf) = (p.a.progressiva_inicial, p.a.progressiva_final());
    let n = numero_de_folhas(pf - p0);
    for i in 0..n {
        let (c0, c1) = (p0 + (pf - p0) * i as f64 / n as f64, p0 + (pf - p0) * (i + 1) as f64 / n as f64);
        let zs: Vec<f64> = p.obs.iter().filter(|o| o.s >= c0 && o.s <= c1).map(|o| o.z).chain((0..=((c1 - c0) / 5.0) as usize).map(|k| p.zg(c0 + k as f64 * 5.0))).collect();
        let (zmin, zmax) = zs.iter().fold((f64::MAX, f64::MIN), |m, z| (m.0.min(*z), m.1.max(*z)));
        let esc_v = escala_vertical(zmin, zmax);
        let (z0, z1) = ((zmin - 0.5).floor(), (zmax + 0.5).ceil());
        let f = inserir_folha(ag, fonte, ctx, "A1", Some(ESCALA_H), "Perfil longitudinal", primeira + i)?;
        let q = Quadro { c0, c1, z0, esc_v, altura: (z1 - z0) * 1000.0 / esc_v };
        let mut c = Caneta::nova(&f, "PERFIL_TITULO");
        c.rotulo((X0, 566.0), 4.2, &format!("PERFIL LONGITUDINAL — FOLHA {} DE {} — EST. {} A {} — KM {} A {}", i + 1, n, estaca_texto(c0, 20.0), estaca_texto(c1, 20.0), km_texto(c0), km_texto(c1)));
        c.em("PERFIL_TITULO", None);
        c.rotulo((X0, 559.5), 2.6, &format!("Escalas: horizontal 1:{} — vertical 1:{} (exagero de {:.0}×). Cotas em metros; progressivas em metros.", ESCALA_H as i64, esc_v as i64, ESCALA_H / esc_v));
        legenda(&mut c, &q);
        grade(&mut c, &q, z1);
        preencher(&mut c, &q, p);
        linhas_do_perfil(&mut c, &q, p);
        pivs(&mut c, &q, p);
        let fundo = tabela(&mut c, &q, p);
        let base_levante = diagrama_de_levante(&mut c, &q, p, fundo - 40.0);
        pontos_de_atencao(&mut c, &q, p, base_levante - 8.0);
        c.gravar(ag)?;
    }
    Ok(())
}

/// Legenda do que o gráfico desenha, no canto direito do título.
fn legenda(c: &mut Caneta, q: &Quadro) {
    let x = (q.x_fim() - 330.0).max(X0 + 300.0);
    let itens: [(&str, u32, &str); 4] = [("Terreno existente (cota do trilho)", COR_EXISTENTE, "linha"), ("Greide de projeto", COR_PROJETO, "linha"), ("Aterro (greide acima)", COR_ATERRO, "area"), ("Escavação (greide abaixo)", COR_ESCAVACAO, "area")];
    for (k, (texto, cor, forma)) in itens.iter().enumerate() {
        let (xx, yy) = (x + (k % 2) as f64 * 165.0, 566.0 - (k / 2) as f64 * 6.5);
        c.em("PERFIL_LEGENDA", Some(*cor));
        if *forma == "linha" {
            c.linha((xx, yy + 1.0), (xx + 10.0, yy + 1.0));
        } else {
            c.hachura(&[(xx, yy - 0.5), (xx + 10.0, yy - 0.5), (xx + 10.0, yy + 2.5), (xx, yy + 2.5)], "ANSI31", 0.8);
            c.ret((xx, yy - 0.5), (xx + 10.0, yy + 2.5));
        }
        c.em("PERFIL_LEGENDA", None);
        c.rotulo((xx + 13.0, yy), 2.4, texto);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn faixas_separam_aterro_de_escavacao_e_ignoram_ruido() {
        let pts: Vec<(f64, f64, f64)> = vec![(0.0, 1.0, 1.005), (2.0, 1.2, 1.0), (4.0, 1.3, 1.0), (6.0, 0.9, 1.0), (8.0, 0.8, 1.0), (10.0, 0.7, 1.0)];
        let f = faixas(&pts, 0.01, 2.0);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f[0].0 && !f[1].0);
        assert_eq!(f[0].1.len(), 2);
    }

    #[test]
    fn escolhe_a_escala_vertical_pelo_tamanho_do_perfil() {
        assert_eq!(escala_vertical(37.0, 60.0), 200.0);
        assert_eq!(escala_vertical(45.0, 47.5), 50.0, "perfil quase plano: escala bem detalhada");
        assert_eq!(escala_vertical(0.0, 40.0), 500.0);
        assert_eq!(numero_de_folhas(6230.0), 5);
    }
}
