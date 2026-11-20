//! `cad_luminotecnico`: o cálculo de iluminação de um ambiente retangular, em Rust — **método dos lumens** (nº de luminárias,
//! arranjo, espaçamento, potência instalada) e **grade de iluminância ponto a ponto** (média, mínima, uniformidade), com a
//! posição de cada luminária. Pode desenhar o mapa de lux em faixas de cor. É o que antes ficava escrito à mão na folha.
//!
//! Premissas (devolvidas no resultado): luminária de distribuição `I(θ) = I0·cosⁿθ` (n = 1 é lambertiana; n maior = facho mais
//! fechado), só a componente **direta** na grade (conservador: sem inter-reflexões), fator de utilização típico por índice do
//! local quando o do fabricante não é informado.

use feathercad_core::documento::{Documento, Geometria};
use feathercad_core::geom::{Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{numero_opcional, ponto_de, Res};
use crate::mutacoes::cor_hex;
use crate::pecas::{aplicar, caixa_das, relatorio, Peca};

/// Fator de utilização típico (refletâncias 50/50/20) por índice do local K — **genérico**, confirmar com a fotometria.
const U_TIPICO: [(f64, f64); 10] = [(0.6, 0.42), (0.8, 0.50), (1.0, 0.56), (1.25, 0.62), (1.5, 0.66), (2.0, 0.72), (2.5, 0.75), (3.0, 0.78), (4.0, 0.81), (5.0, 0.83)];

struct Ambiente {
    c: f64,
    l: f64,
    h: f64,
    plano: f64,
}

struct Luminaria {
    fluxo: f64,
    potencia: f64,
    montagem: f64,
    expoente: f64,
}

struct Pedido {
    amb: Ambiente,
    lum: Luminaria,
    e_alvo: f64,
    d: f64,
    u: Option<f64>,
    uniformidade_min: f64,
    e_hm_max: f64,
    rho_parede: f64,
}

struct Arranjo {
    linhas: usize,
    colunas: usize,
}

fn pos(args: &Value, caminho: &[&str], nome: &str) -> Res<f64> {
    let mut v = args;
    for c in caminho {
        v = v.get(*c).ok_or_else(|| format!("falta `{}` ({nome})", caminho.join(".")))?;
    }
    v.as_f64().filter(|n| *n > 0.0 && n.is_finite()).ok_or_else(|| format!("`{}` precisa ser um número maior que zero ({nome})", caminho.join(".")))
}

fn pedido_de(args: &Value) -> Res<Pedido> {
    let amb = Ambiente { c: pos(args, &["ambiente", "comprimento"], "m")?, l: pos(args, &["ambiente", "largura"], "m")?, h: pos(args, &["ambiente", "pe_direito"], "m")?,
        plano: args.pointer("/ambiente/plano_trabalho").and_then(Value::as_f64).filter(|p| *p >= 0.0).unwrap_or(0.8) };
    let montagem = args.pointer("/luminaria/altura_montagem").and_then(Value::as_f64).filter(|m| *m > amb.plano).unwrap_or(amb.h);
    let lum = Luminaria { fluxo: pos(args, &["luminaria", "fluxo_lm"], "lm")?, potencia: pos(args, &["luminaria", "potencia_w"], "W")?, montagem,
        expoente: args.pointer("/luminaria/expoente").and_then(Value::as_f64).filter(|n| *n >= 0.0).unwrap_or(1.0) };
    if montagem - amb.plano < 0.3 {
        return Err("a altura de montagem precisa estar pelo menos 0,3 m acima do plano de trabalho".into());
    }
    Ok(Pedido { amb, lum, e_alvo: pos(args, &["iluminancia_lux"], "lux")?, d: numero_opcional(args, "fator_manutencao").filter(|d| *d > 0.0 && *d <= 1.0).unwrap_or(0.8),
        u: args.pointer("/luminaria/fator_utilizacao").and_then(Value::as_f64).filter(|u| *u > 0.0 && *u <= 1.0),
        uniformidade_min: numero_opcional(args, "uniformidade_minima").unwrap_or(0.6), e_hm_max: numero_opcional(args, "espacamento_max_hm").unwrap_or(1.5),
        rho_parede: args.pointer("/ambiente/refletancias/1").and_then(Value::as_f64).filter(|r| (0.0..=0.9).contains(r)).unwrap_or(0.5) })
}

fn hm(p: &Pedido) -> f64 {
    p.lum.montagem - p.amb.plano
}

/// Índice do local K = (C·L) / (hm·(C+L)).
fn indice_do_local(p: &Pedido) -> f64 {
    p.amb.c * p.amb.l / (hm(p) * (p.amb.c + p.amb.l))
}

fn u_por_indice(k: f64) -> f64 {
    let t = &U_TIPICO;
    if k <= t[0].0 {
        return t[0].1;
    }
    for w in t.windows(2) {
        if k <= w[1].0 {
            return w[0].1 + (w[1].1 - w[0].1) * (k - w[0].0) / (w[1].0 - w[0].0);
        }
    }
    t[t.len() - 1].1
}

/// O arranjo de menor quantidade em que os dois espaçamentos respeitam `e_hm_max · hm`, o mais próximo do quadrado.
fn arranjar(p: &Pedido, n_min: usize) -> Option<Arranjo> {
    let limite = p.e_hm_max * hm(p);
    for n in n_min..n_min + 60 {
        let melhor = (1..=n)
            .filter(|l| n % l == 0)
            .map(|l| Arranjo { linhas: l, colunas: n / l })
            .filter(|a| p.amb.c / a.colunas as f64 <= limite && p.amb.l / a.linhas as f64 <= limite)
            .min_by(|a, b| razao(p, a).total_cmp(&razao(p, b)));
        if melhor.is_some() {
            return melhor;
        }
    }
    None
}

/// Quão longe de espaçamentos iguais (0 = quadrado).
fn razao(p: &Pedido, a: &Arranjo) -> f64 {
    let (ex, ey) = (p.amb.c / a.colunas as f64, p.amb.l / a.linhas as f64);
    (ex - ey).abs() / ex.max(ey)
}

fn posicoes(p: &Pedido, a: &Arranjo) -> Vec<(f64, f64)> {
    let (ex, ey) = (p.amb.c / a.colunas as f64, p.amb.l / a.linhas as f64);
    (0..a.linhas).flat_map(|j| (0..a.colunas).map(move |i| ((i as f64 + 0.5) * ex, (j as f64 + 0.5) * ey))).collect()
}

/// Uma fonte de luz: posição (m) e peso (1 = a luminária; ρ = sua imagem na parede).
type Fonte = (f64, f64, f64);

/// As luminárias com peso 1 e, se `rho > 0`, a imagem de cada uma espelhada nas quatro paredes com peso ρ
/// (aproximação de 1ª ordem da luz que as paredes devolvem ao plano de trabalho).
fn fontes(p: &Pedido, luminarias: &[(f64, f64)], com_paredes: bool) -> Vec<Fonte> {
    let mut v: Vec<Fonte> = luminarias.iter().map(|(x, y)| (*x, *y, 1.0)).collect();
    if com_paredes && p.rho_parede > 0.0 {
        for (x, y) in luminarias {
            let w = p.rho_parede;
            v.extend([(-x, *y, w), (2.0 * p.amb.c - x, *y, w), (*x, -y, w), (*x, 2.0 * p.amb.l - y, w)]);
        }
    }
    v
}

/// Iluminância (lux) no ponto somando as fontes: E = d · w · Φ(n+1)/(2π) · cos^(n+3)θ / hm².
fn lux_em(p: &Pedido, fontes: &[Fonte], x: f64, y: f64) -> f64 {
    let h = hm(p);
    let i0 = p.lum.fluxo * (p.lum.expoente + 1.0) / (2.0 * std::f64::consts::PI);
    fontes
        .iter()
        .map(|(lx, ly, w)| {
            let r2 = (x - lx).powi(2) + (y - ly).powi(2);
            let cos = h / (h * h + r2).sqrt();
            w * p.d * i0 * cos.powf(p.lum.expoente + 3.0) / (h * h)
        })
        .sum()
}

struct Grade {
    passo_x: f64,
    passo_y: f64,
    colunas: usize,
    linhas: usize,
    valores: Vec<f64>,
}

fn grade(p: &Pedido, luminarias: &[Fonte], passo: f64) -> Grade {
    let colunas = ((p.amb.c / passo).round() as usize).max(1);
    let linhas = ((p.amb.l / passo).round() as usize).max(1);
    let (px, py) = (p.amb.c / colunas as f64, p.amb.l / linhas as f64);
    let valores = (0..linhas).flat_map(|j| (0..colunas).map(move |i| (i, j))).map(|(i, j)| lux_em(p, luminarias, (i as f64 + 0.5) * px, (j as f64 + 0.5) * py)).collect();
    Grade { passo_x: px, passo_y: py, colunas, linhas, valores }
}

struct Estatistica {
    media: f64,
    minima: f64,
    maxima: f64,
}

/// Média, mínima e máxima da área de tarefa: os pontos a menos de `borda` m das paredes ficam de fora (NBR ISO 8995-1).
fn estatistica(g: &Grade, p: &Pedido, borda: f64) -> Estatistica {
    let dentro = |idx: usize| {
        let (i, j) = (idx % g.colunas, idx / g.colunas);
        let (x, y) = ((i as f64 + 0.5) * g.passo_x, (j as f64 + 0.5) * g.passo_y);
        x >= borda && x <= p.amb.c - borda && y >= borda && y <= p.amb.l - borda
    };
    let mut v: Vec<f64> = g.valores.iter().enumerate().filter(|(i, _)| dentro(*i)).map(|(_, e)| *e).collect();
    if v.is_empty() {
        v = g.valores.clone();
    }
    let n = v.len() as f64;
    Estatistica { media: v.iter().sum::<f64>() / n, minima: v.iter().copied().fold(f64::MAX, f64::min), maxima: v.iter().copied().fold(f64::MIN, f64::max) }
}

fn avisos_de(p: &Pedido, a: &Arranjo, dir: &Estatistica, par: &Estatistica, e_metodo: f64) -> Vec<String> {
    let mut v = Vec::new();
    let u_par = par.minima / par.media;
    if u_par < p.uniformidade_min {
        v.push(format!("uniformidade com as paredes {u_par:.2} abaixo de {} — aproxime as luminárias, aumente a quantidade ou use facho mais aberto (`expoente` menor)", p.uniformidade_min));
    }
    if par.media < p.e_alvo * 0.95 {
        v.push(format!("a grade dá {:.0} lux médios (com as paredes) contra {:.0} pedidos; o método dos lumens dá {:.0}. Confirme com a fotometria do fabricante no DIALux/Relux", par.media, p.e_alvo, e_metodo));
    }
    if dir.minima / dir.media < p.uniformidade_min {
        v.push(format!("só com a luz direta (conservador) a uniformidade é {:.2}: depende da luz que as paredes devolvem — simule no DIALux/Relux com as refletâncias reais", dir.minima / dir.media));
    }
    if p.amb.c / a.colunas as f64 > p.e_hm_max * hm(p) || p.amb.l / a.linhas as f64 > p.e_hm_max * hm(p) {
        v.push("espaçamento acima do limite e/hm".into());
    }
    v
}

/// Faixas do mapa de lux em relação à meta: (limite superior da razão E/Emeta, cor, nome).
const FAIXAS: [(f64, &str, &str); 5] = [(0.5, "#C0392B", "abaixo de 50 % da meta"), (0.75, "#E67E22", "50 a 75 %"), (1.0, "#F1C40F", "75 a 100 %"), (1.25, "#82C97E", "100 a 125 %"), (f64::MAX, "#5DADE2", "acima de 125 %")];

fn faixa_de(razao: f64) -> usize {
    FAIXAS.iter().position(|(lim, _, _)| razao < *lim).unwrap_or(FAIXAS.len() - 1)
}

/// Desenha o mapa (células de mesma faixa, lado a lado na mesma linha, juntas num só retângulo).
fn desenhar_mapa(doc: &mut Documento, p: &Pedido, g: &Grade, a: &Value) -> Res<Value> {
    let origem = ponto_de(a, "origem")?;
    let camada = a.get("camada").and_then(Value::as_str).unwrap_or("LUX").to_string();
    let m = doc.unidade().map_or(1.0, |u| 1000.0 / u.em_milimetros());
    let mut pecas: Vec<Peca> = Vec::new();
    for j in 0..g.linhas {
        let mut i = 0;
        while i < g.colunas {
            let f = faixa_de(g.valores[j * g.colunas + i] / p.e_alvo);
            let mut fim = i + 1;
            while fim < g.colunas && faixa_de(g.valores[j * g.colunas + fim] / p.e_alvo) == f {
                fim += 1;
            }
            let (x0, x1) = (origem.x + i as f64 * g.passo_x * m, origem.x + fim as f64 * g.passo_x * m);
            let (y0, y1) = (origem.y + j as f64 * g.passo_y * m, origem.y + (j + 1) as f64 * g.passo_y * m);
            let v = [Ponto::novo(x0, y0), Ponto::novo(x1, y0), Ponto::novo(x1, y1), Ponto::novo(x0, y1)].map(|p| Vertice { p, bulge: 0.0 }).to_vec();
            pecas.push(Peca { camada: camada.clone(), cor: Some(cor_hex(FAIXAS[f].1)?), g: Geometria::Hachura { contornos: vec![v], padrao: Vec::new(), nome: "SOLID".into() } });
            i = fim;
        }
    }
    let caixa = caixa_das(&pecas);
    let ids = aplicar(doc, pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["faixas"] = json!(FAIXAS.iter().map(|(lim, cor, nome)| json!({ "cor": cor, "nome": nome, "ate_razao": if *lim > 1e9 { Value::Null } else { json!(lim) } })).collect::<Vec<_>>());
    Ok(r)
}

fn arred(v: f64, c: i32) -> f64 {
    let k = 10f64.powi(c);
    (v * k).round() / k
}

/// `cad_luminotecnico`: calcula e, com `desenhar_mapa: {origem, camada}`, desenha o mapa de lux (origem = canto inferior esquerdo do ambiente).
/// Linhas ou colunas de luminárias num `arranjo` informado (100 mil × 100 mil esgotava a memória).
const ARRANJO_MAXIMO: u64 = 200;

pub fn luminotecnico(doc: &mut Documento, args: &Value) -> Res<Value> {
    let p = pedido_de(args)?;
    let k = indice_do_local(&p);
    let u = p.u.unwrap_or_else(|| u_por_indice(k));
    let area = p.amb.c * p.amb.l;
    let fluxo_total = p.e_alvo * area / (u * p.d);
    let n_min = (fluxo_total / p.lum.fluxo).ceil().max(1.0) as usize;
    let a = match (args.pointer("/arranjo/linhas").and_then(Value::as_u64), args.pointer("/arranjo/colunas").and_then(Value::as_u64)) {
        (Some(l), Some(c)) if l > 0 && c > 0 => Arranjo { linhas: l.min(ARRANJO_MAXIMO) as usize, colunas: c.min(ARRANJO_MAXIMO) as usize },
        _ => arranjar(&p, n_min).ok_or("não achei arranjo que respeite o espaçamento máximo: aumente `espacamento_max_hm` ou informe `arranjo`")?,
    };
    let lista = posicoes(&p, &a);
    let n = lista.len();
    let e_metodo = n as f64 * p.lum.fluxo * u * p.d / area;
    // Cada ponto da grade soma todas as luminárias: o passo não desce de onde a conta passaria de ~5·10⁷ (travava o app).
    let passo_minimo = (area * n.max(1) as f64 / 5.0e7).sqrt();
    let passo = numero_opcional(args, "grade_passo").filter(|s| *s > 0.0).unwrap_or(1.0).max(passo_minimo);
    let g_dir = grade(&p, &fontes(&p, &lista, false), passo);
    let g = grade(&p, &fontes(&p, &lista, true), passo);
    let borda = numero_opcional(args, "borda_m").filter(|b| *b >= 0.0).unwrap_or(0.5);
    let (dir, est) = (estatistica(&g_dir, &p, borda), estatistica(&g, &p, borda));
    let mut r = json!({
        "indice_do_local_k": arred(k, 2), "fator_utilizacao": arred(u, 2), "fator_utilizacao_origem": if p.u.is_some() { "informado" } else { "típico genérico pelo índice do local" },
        "fator_manutencao": p.d, "altura_util_hm_m": arred(hm(&p), 2), "area_m2": arred(area, 1), "fluxo_total_necessario_lm": fluxo_total.round(),
        "luminarias_minimo": n_min, "luminarias": n, "arranjo": { "linhas_ao_longo_da_largura": a.linhas, "colunas_ao_longo_do_comprimento": a.colunas },
        "espacamento_m": { "entre_colunas": arred(p.amb.c / a.colunas as f64, 2), "entre_linhas": arred(p.amb.l / a.linhas as f64, 2),
                           "da_parede_x": arred(p.amb.c / a.colunas as f64 / 2.0, 2), "da_parede_y": arred(p.amb.l / a.linhas as f64 / 2.0, 2) },
        "relacao_e_hm": { "colunas": arred(p.amb.c / a.colunas as f64 / hm(&p), 2), "linhas": arred(p.amb.l / a.linhas as f64 / hm(&p), 2), "limite": p.e_hm_max },
        "iluminancia_media_metodo_lux": e_metodo.round(), "potencia_instalada_w": arred(n as f64 * p.lum.potencia, 0), "densidade_w_m2": arred(n as f64 * p.lum.potencia / area, 2),
        "eficiencia_lux_por_w_m2": arred(e_metodo / (n as f64 * p.lum.potencia / area), 1),
        "grade": { "passo_m": [arred(g.passo_x, 2), arred(g.passo_y, 2)], "pontos": g.valores.len(), "borda_excluida_m": borda, "refletancia_parede": p.rho_parede,
                   "media_lux": est.media.round(), "minima_lux": est.minima.round(), "maxima_lux": est.maxima.round(), "uniformidade": arred(est.minima / est.media, 2),
                   "so_direta": { "media_lux": dir.media.round(), "minima_lux": dir.minima.round(), "uniformidade": arred(dir.minima / dir.media, 2) } },
        "posicoes_m": lista.iter().map(|(x, y)| json!([arred(*x, 3), arred(*y, 3)])).collect::<Vec<_>>(),
        "premissas": ["método dos lumens: N = E·A / (u·d·Φ); u típico genérico por índice do local (refletâncias 50/50/20) quando não informado",
                      "estatística da grade na área de tarefa, sem a faixa de `borda_m` (0,5 m) junto às paredes; I(θ) = I0·cosⁿθ (n = `expoente`, 1 = lambertiana)",
                      "a grade principal soma a luz direta e a imagem de cada luminária nas 4 paredes com peso = refletância da parede (`ambiente.refletancias[1]`, 0,5): aproximação de 1ª ordem; `so_direta` é o valor conservador, sem as paredes",
                      "fator de manutenção 0,8 (LED, ambiente com poeira) se não informado; uniformidade mínima 0,6 e e/hm ≤ 1,5 são premissas, não norma",
                      "confirme com a fotometria (.ies/.ldt) do fabricante e simule no DIALux/Relux antes de comprar"],
        "avisos": avisos_de(&p, &a, &dir, &est, e_metodo),
    });
    if args.get("grade_passo").is_none() {
        r["grade"]["valores"] = Value::Null;
    }
    if let Some(m) = args.get("desenhar_mapa").filter(|m| m.is_object()) {
        r["mapa"] = desenhar_mapa(doc, &p, &g, m)?;
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn galpao() -> Value {
        json!({ "ambiente": { "comprimento": 40, "largura": 20, "pe_direito": 6 }, "iluminancia_lux": 300,
                "luminaria": { "fluxo_lm": 21000, "potencia_w": 150, "fator_utilizacao": 0.72 }, "fator_manutencao": 0.8 })
    }

    #[test]
    fn metodo_dos_lumens_do_galpao_de_800_m2() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let r = luminotecnico(&mut d, &galpao()).unwrap();
        // 300 x 800 / (0,72 x 0,8) = 416.667 lm -> 19,8 -> 20 luminárias
        assert!((r["fluxo_total_necessario_lm"].as_f64().unwrap() - 416_667.0).abs() < 1.0, "{r}");
        assert_eq!(r["luminarias_minimo"], json!(20));
        assert!(r["luminarias"].as_u64().unwrap() >= 20);
        assert!((r["indice_do_local_k"].as_f64().unwrap() - 2.56).abs() < 0.01, "{r}");
        assert_eq!(r["posicoes_m"].as_array().unwrap().len() as u64, r["luminarias"].as_u64().unwrap());
    }

    #[test]
    fn grade_simetrica_tem_minima_menor_que_a_media_e_mapa_desenha() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let mut a = galpao();
        a["grade_passo"] = json!(2);
        a["desenhar_mapa"] = json!({ "origem": [0, 0] });
        let r = luminotecnico(&mut d, &a).unwrap();
        let g = &r["grade"];
        assert!(g["minima_lux"].as_f64().unwrap() < g["media_lux"].as_f64().unwrap());
        // as paredes devolvem luz às bordas: a uniformidade com elas é maior que a só direta, e a média também
        assert!(g["uniformidade"].as_f64().unwrap() > g["so_direta"]["uniformidade"].as_f64().unwrap(), "{g}");
        assert!(g["media_lux"].as_f64().unwrap() > g["so_direta"]["media_lux"].as_f64().unwrap());
        assert!(r["mapa"]["criadas"].as_u64().unwrap() >= 10, "{r}");
        assert_eq!(r["mapa"]["faixas"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn dados_faltando_ou_absurdos_sao_recusados() {
        let mut d = Documento::default();
        assert!(luminotecnico(&mut d, &json!({ "iluminancia_lux": 300 })).is_err());
        let mut a = galpao();
        a["luminaria"]["altura_montagem"] = json!(1.0);
        assert!(luminotecnico(&mut d, &a).is_err(), "montagem a 0,2 m do plano não pode valer");
    }
}
