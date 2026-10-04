//! `cad_grafico`: gráfico X×Y a partir de séries de dados (alinhamento, superelevação, perfil, ripagem…).
//! A área de plotagem tem tamanho em mm de papel (com `escala`); vários gráficos empilhados com o mesmo eixo X
//! são várias chamadas com a mesma `escala`, `largura` e `eixo_x`.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{numero_de, numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, caixa_das, casas_do_passo, cor_de, fator_papel, formatar, relatorio, Pincel};

struct Eixo {
    de: f64,
    ate: f64,
    passo: Option<f64>,
    rotulo: String,
    decimais: Option<usize>,
}

fn eixo(args: &Value, nome: &str) -> Res<Eixo> {
    let e = args.get(nome).ok_or_else(|| format!("falta `{nome}` {{de, ate, marcas, rotulo}}"))?;
    let (de, ate) = (numero_de(e, "de").map_err(|m| format!("{nome}: {m}"))?, numero_de(e, "ate").map_err(|m| format!("{nome}: {m}"))?);
    if ate <= de {
        return Err(format!("{nome}: `ate` precisa ser maior que `de`"));
    }
    Ok(Eixo { de, ate, passo: numero_opcional(e, "marcas").filter(|m| *m > 0.0), rotulo: texto_opcional(e, "rotulo").unwrap_or("").to_string(), decimais: e.get("decimais").and_then(Value::as_u64).map(|d| d as usize) })
}

/// Pedaço do segmento a–b que cabe em [0,w]×[0,h] (Liang–Barsky), ou `None`.
pub fn recortar(a: (f64, f64), b: (f64, f64), w: f64, h: f64) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0), (dx, w - a.0), (-dy, a.1), (dy, h - a.1)] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    Some(((a.0 + t0 * dx, a.1 + t0 * dy), (a.0 + t1 * dx, a.1 + t1 * dy)))
}

fn marcas(e: &Eixo) -> Vec<f64> {
    let Some(passo) = e.passo else { return vec![e.de, e.ate] };
    let mut v = Vec::new();
    let mut n = (e.de / passo - 1e-9).ceil();
    while n * passo <= e.ate + passo * 1e-9 && v.len() < 2000 {
        v.push(n * passo);
        n += 1.0;
    }
    v
}

/// A área de plotagem: tamanho, origem e a escala dos dados.
struct Area {
    w: f64,
    h: f64,
    ht: f64,
    origem: Ponto,
    ex: f64,
    ey: f64,
    sx: f64,
    sy: f64,
}

/// Desenha uma série; devolve quantos pontos caíram fora da caixa.
fn serie(p: &mut Pincel, a: &Area, i: usize, s: &Value, camada: &str) -> Res<usize> {
    let (w, h, ht) = (a.w, a.h, a.ht);
    let ao_desenho = |q: (f64, f64)| Ponto::novo(a.origem.x + q.0, a.origem.y + q.1);
    let pts: Vec<(f64, f64)> = s
        .get("pontos")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("série {i}: falta `pontos` [[x,y],…]"))?
        .iter()
        .map(|q| match (q.get(0).and_then(Value::as_f64), q.get(1).and_then(Value::as_f64)) {
            (Some(x), Some(y)) => Ok(((x - a.ex) * a.sx, (y - a.ey) * a.sy)),
            _ => Err(format!("série {i}: ponto inválido")),
        })
        .collect::<Res<_>>()?;
    p.em(texto_opcional(s, "camada").unwrap_or(camada), cor_de(s, "cor")?);
    let estilo = texto_opcional(s, "estilo").unwrap_or("linha");
    let dentro = |q: (f64, f64)| q.0 >= -1e-9 && q.0 <= w + 1e-9 && q.1 >= -1e-9 && q.1 <= h + 1e-9;
    let fora = pts.iter().filter(|q| !dentro(**q)).count();
    match estilo {
        "linha" | "degrau" => {
            let mut caminho = pts.clone();
            if estilo == "degrau" {
                caminho = pts.windows(2).flat_map(|j| [j[0], (j[1].0, j[0].1)]).chain(pts.last().copied()).collect();
            }
            let mut corrente: Vec<Ponto> = Vec::new();
            let mut fim_anterior: Option<(f64, f64)> = None;
            for j in caminho.windows(2) {
                match recortar(j[0], j[1], w, h) {
                    Some((q0, q1)) => {
                        if fim_anterior != Some(q0) && !corrente.is_empty() {
                            p.polilinha(&corrente);
                            corrente.clear();
                        }
                        if corrente.is_empty() {
                            corrente.push(ao_desenho(q0));
                        }
                        corrente.push(ao_desenho(q1));
                        fim_anterior = Some(q1);
                    }
                    None => {
                        p.polilinha(&corrente);
                        corrente.clear();
                        fim_anterior = None;
                    }
                }
            }
            p.polilinha(&corrente);
        }
        "cruz" => {
            let m = 0.6 * ht;
            for q in pts.iter().filter(|q| dentro(**q)) {
                p.linha(ao_desenho((q.0 - m, q.1)), ao_desenho((q.0 + m, q.1)));
                p.linha(ao_desenho((q.0, q.1 - m)), ao_desenho((q.0, q.1 + m)));
            }
        }
        outro => return Err(format!("série {i}: estilo `{outro}` (linha, cruz, degrau)")),
    }
    if let Some(rotulos) = s.get("rotulos").and_then(Value::as_array) {
        for (q, t) in pts.iter().zip(rotulos) {
            if let (true, Some(t)) = (dentro(*q), t.as_str()) {
                p.texto(ao_desenho((q.0, q.1 + 0.6 * ht)), ht * 0.8, 0.0, t, AlinhaH::Centro, AlinhaV::Base);
            }
        }
    }
    Ok(fora)
}

pub fn grafico(doc: &mut Documento, args: &Value) -> Res<Value> {
    let origem = ponto_de(args, "origem")?;
    let k = fator_papel(doc, args)?;
    let (w, h) = (numero_de(args, "largura")? * k, numero_de(args, "altura")? * k);
    if w <= 0.0 || h <= 0.0 {
        return Err("`largura` e `altura` precisam ser maiores que zero".into());
    }
    let (ex, ey) = (eixo(args, "eixo_x")?, eixo(args, "eixo_y")?);
    let ht = numero_opcional(args, "altura_texto").unwrap_or(2.5) * k;
    let virgula = args.get("virgula").and_then(Value::as_bool).unwrap_or(false);
    let camada = texto_opcional(args, "camada").unwrap_or("GRAFICO");
    let grade = args.get("grade").and_then(Value::as_bool).unwrap_or(true);
    let (sx, sy) = (w / (ex.ate - ex.de), h / (ey.ate - ey.de));
    let ao_desenho = |p: (f64, f64)| Ponto::novo(origem.x + p.0, origem.y + p.1);

    let mut p = Pincel::novo(camada);
    p.cor = cor_de(args, "cor")?;
    p.retangulo(origem, Ponto::novo(origem.x + w, origem.y + h));
    let grade_camada = format!("{camada}_GRADE");
    let dec_x = ex.decimais.unwrap_or_else(|| ex.passo.map_or(0, casas_do_passo));
    let dec_y = ey.decimais.unwrap_or_else(|| ey.passo.map_or(0, casas_do_passo));
    for x in marcas(&ex) {
        let px = (x - ex.de) * sx;
        if grade && x > ex.de && x < ex.ate {
            p.em(&grade_camada, Some(0x80_80_80)).linha(ao_desenho((px, 0.0)), ao_desenho((px, h)));
        }
        p.em(camada, None).linha(ao_desenho((px, 0.0)), ao_desenho((px, -0.5 * ht)));
        p.texto(ao_desenho((px, -0.7 * ht)), ht, 0.0, &formatar(x, dec_x, virgula), AlinhaH::Centro, AlinhaV::Topo);
    }
    for y in marcas(&ey) {
        let py = (y - ey.de) * sy;
        if grade && y > ey.de && y < ey.ate {
            p.em(&grade_camada, Some(0x80_80_80)).linha(ao_desenho((0.0, py)), ao_desenho((w, py)));
        }
        p.em(camada, None).linha(ao_desenho((0.0, py)), ao_desenho((-0.5 * ht, py)));
        p.texto(ao_desenho((-0.7 * ht, py)), ht, 0.0, &formatar(y, dec_y, virgula), AlinhaH::Direita, AlinhaV::Meio);
    }
    p.em(camada, None);
    if !ex.rotulo.is_empty() {
        p.texto(ao_desenho((w / 2.0, -2.6 * ht)), ht, 0.0, &ex.rotulo, AlinhaH::Centro, AlinhaV::Topo);
    }
    if !ey.rotulo.is_empty() {
        let largura_dos_numeros = marcas(&ey).iter().map(|y| feathercad_io::pdf::largura_do_conteudo(&formatar(*y, dec_y, virgula), ht)).fold(0.0, f64::max);
        p.texto(ao_desenho((-(0.7 * ht + largura_dos_numeros + 0.8 * ht), h / 2.0)), ht, 90.0, &ey.rotulo, AlinhaH::Centro, AlinhaV::Meio);
    }
    if let Some(t) = texto_opcional(args, "titulo") {
        p.texto(ao_desenho((w / 2.0, h + 0.8 * ht)), ht * 1.3, 0.0, t, AlinhaH::Centro, AlinhaV::Base);
    }

    let mut fora = 0usize;
    let area = Area { w, h, ht, origem, ex: ex.de, ey: ey.de, sx, sy };
    for (i, s) in args.get("series").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]).iter().enumerate() {
        fora += serie(&mut p, &area, i, s, camada)?;
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["unidades_por_unidade_de_dado"] = json!({ "x": sx, "y": sy });
    r["area_de_plotagem"] = json!({ "min": [origem.x, origem.y], "max": [origem.x + w, origem.y + h] });
    r["pontos_fora_da_caixa"] = json!(fora);
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn plota_serie_recortada_com_marcas_e_devolve_a_escala() {
        let mut d = Documento::default();
        let r = grafico(&mut d, &json!({ "origem": [0, 0], "largura": 100, "altura": 50,
            "eixo_x": { "de": 0, "ate": 1000, "marcas": 250, "rotulo": "estaca" }, "eixo_y": { "de": 0, "ate": 10, "marcas": 5, "rotulo": "h (mm)" },
            "series": [{ "pontos": [[0, 0], [500, 20], [1000, 10]], "cor": "#FF0000" }, { "pontos": [[100, 5]], "estilo": "cruz" }] })).unwrap();
        assert_eq!(r["unidades_por_unidade_de_dado"]["x"], 0.1);
        assert_eq!(r["pontos_fora_da_caixa"], 1);
        assert!(r["criadas"].as_u64().unwrap() > 15, "{r}");
        // a série recortada não sai da área de plotagem
        let (a, b) = d.caixa().unwrap();
        assert!(b.y - a.y > 50.0 && a.x < 0.0, "rótulos ficam fora da caixa, o traço não");
    }

    #[test]
    fn recorta_segmento_na_caixa() {
        let (a, b) = recortar((-10.0, 5.0), (10.0, 5.0), 5.0, 10.0).unwrap();
        assert_eq!((a.0, b.0), (0.0, 5.0));
        assert!(recortar((-1.0, -1.0), (-2.0, -3.0), 5.0, 5.0).is_none());
    }
}
