//! `cad_malha_coordenadas` e `cad_norte`: a graduação de coordenadas de uma folha e o símbolo do norte.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria};
use feathercad_core::geom::{Ponto, Vertice};
use serde_json::{json, Value};

use crate::json::{janela, numero_opcional, ponto_de, texto_opcional, Res};
use crate::pecas::{aplicar, casas_do_passo, cor_de, fator_papel, formatar, relatorio, caixa_das, Peca, Pincel};

/// Malha de coordenadas sobre a `caixa` (em unidades do desenho): traços nas quatro bordas, rótulos embaixo e à
/// esquerda, e cruzetas (ou linhas finas, com `cruzetas: false`) nas interseções.
pub fn malha(doc: &mut Documento, args: &Value) -> Res<Value> {
    let (min, max) = janela(args.get("caixa").ok_or("falta `caixa` (janela do desenho: [[x1,y1],[x2,y2]])")?)?;
    let passo = numero_opcional(args, "passo").filter(|p| *p > 0.0).ok_or("falta `passo` (maior que zero)")?;
    let k = fator_papel(doc, args)?;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(2.5) * k;
    let virgula = args.get("virgula").and_then(Value::as_bool).unwrap_or(false);
    let casas = casas_do_passo(passo);
    let prefixos = match texto_opcional(args, "rotulos").unwrap_or("E/N") {
        "E/N" => ("E ", "N "),
        "xy" => ("X ", "Y "),
        "nenhum" => ("", ""),
        o => return Err(format!("`rotulos`: E/N, xy ou nenhum, não `{o}`")),
    };
    let escrever = prefixos != ("", "");
    let cruzetas = args.get("cruzetas").and_then(Value::as_bool).unwrap_or(true);
    let n = |de: f64, ate: f64| -> Vec<f64> { (((de / passo) - 1e-9).ceil() as i64..=((ate / passo) + 1e-9).floor() as i64).take(2000).map(|i| i as f64 * passo).collect() };
    let (xs, ys) = (n(min.x, max.x), n(min.y, max.y));
    let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("MALHA"));
    p.cor = cor_de(args, "cor")?;
    p.retangulo(min, max);
    let t = 0.8 * ht;
    for x in &xs {
        p.linha(Ponto::novo(*x, min.y), Ponto::novo(*x, min.y + t));
        p.linha(Ponto::novo(*x, max.y), Ponto::novo(*x, max.y - t));
        if escrever {
            p.texto(Ponto::novo(*x, min.y - 0.5 * ht), ht, 0.0, &format!("{}{}", prefixos.0, formatar(*x, casas, virgula)), AlinhaH::Centro, AlinhaV::Topo);
        }
    }
    for y in &ys {
        p.linha(Ponto::novo(min.x, *y), Ponto::novo(min.x + t, *y));
        p.linha(Ponto::novo(max.x, *y), Ponto::novo(max.x - t, *y));
        if escrever {
            p.texto(Ponto::novo(min.x - 0.5 * ht, *y), ht, 90.0, &format!("{}{}", prefixos.1, formatar(*y, casas, virgula)), AlinhaH::Centro, AlinhaV::Base);
        }
    }
    for x in xs.iter().filter(|x| **x > min.x && **x < max.x) {
        for y in ys.iter().filter(|y| **y > min.y && **y < max.y) {
            if cruzetas {
                p.linha(Ponto::novo(x - t, *y), Ponto::novo(x + t, *y));
                p.linha(Ponto::novo(*x, y - t), Ponto::novo(*x, y + t));
            }
        }
        if !cruzetas {
            p.linha(Ponto::novo(*x, min.y), Ponto::novo(*x, max.y));
        }
    }
    if !cruzetas {
        for y in ys.iter().filter(|y| **y > min.y && **y < max.y) {
            p.linha(Ponto::novo(min.x, *y), Ponto::novo(max.x, *y));
        }
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    let mut r = relatorio(&ids, caixa);
    r["linhas_x"] = json!(xs.len());
    r["linhas_y"] = json!(ys.len());
    Ok(r)
}

fn girar(p: (f64, f64), centro: Ponto, graus: f64) -> Ponto {
    let (s, c) = graus.to_radians().sin_cos();
    Ponto::novo(centro.x + p.0 * c - p.1 * s, centro.y + p.0 * s + p.1 * c)
}

/// Seta ou rosa dos ventos em `ponto`, de `tamanho` (mm de papel com `escala`), girada `rotacao` graus
/// (a declinação: o norte da folha é o do desenho girado).
pub fn norte(doc: &mut Documento, args: &Value) -> Res<Value> {
    let centro = ponto_de(args, "ponto")?;
    let k = fator_papel(doc, args)?;
    let t = numero_opcional(args, "tamanho").filter(|t| *t > 0.0).unwrap_or(15.0) * k;
    let rot = numero_opcional(args, "rotacao").unwrap_or(0.0);
    let tipo = texto_opcional(args, "tipo").unwrap_or("seta");
    let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("NORTE"));
    p.cor = cor_de(args, "cor")?;
    let v = |l: &[(f64, f64)]| -> Vec<Vertice> { l.iter().map(|q| Vertice { p: girar((q.0 * t, q.1 * t), centro, rot), bulge: 0.0 }).collect() };
    let pol = |p: &mut Pincel, l: &[(f64, f64)]| p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Polilinha { vertices: v(l), fechada: true } });
    let letra = |p: &mut Pincel, x: f64, y: f64, s: &str, h: f64, v: AlinhaV| p.texto(Ponto::novo(centro.x + girar((x * t, y * t), Ponto::novo(0.0, 0.0), rot).x, centro.y + girar((x * t, y * t), Ponto::novo(0.0, 0.0), rot).y), h * t, 0.0, s, AlinhaH::Centro, v);
    match tipo {
        "seta" => {
            p.linha(girar((0.0, -0.5 * t), centro, rot), girar((0.0, 0.1 * t), centro, rot));
            pol(&mut p, &[(0.0, 0.5), (-0.14, -0.05), (0.0, 0.05), (0.14, -0.05)]);
            let metade = v(&[(0.0, 0.5), (-0.14, -0.05), (0.0, 0.05)]);
            let (padrao, nome) = feathercad_core::padroes::para_contornos(feathercad_core::padroes::SOLIDA, 1.0, std::slice::from_ref(&metade));
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Hachura { contornos: vec![metade], padrao, nome } });
            letra(&mut p, 0.0, 0.58, "N", 0.28, AlinhaV::Base);
        }
        "rosa" => {
            p.pecas.push(Peca { camada: p.camada.clone(), cor: p.cor, g: Geometria::Circulo { centro, raio: 0.3 * t } });
            pol(&mut p, &[(0.0, 0.5), (0.06, 0.06), (0.5, 0.0), (0.06, -0.06), (0.0, -0.5), (-0.06, -0.06), (-0.5, 0.0), (-0.06, 0.06)]);
            let r = std::f64::consts::FRAC_1_SQRT_2;
            pol(&mut p, &[(0.0, 0.0), (0.25 * r, 0.25 * r)]);
            for (x, y, s) in [(0.0, 0.64, "N"), (0.6, 0.0, "L"), (0.0, -0.64, "S"), (-0.6, 0.0, "O")] {
                letra(&mut p, x, y, s, 0.2, AlinhaV::Meio);
            }
        }
        o => return Err(format!("`tipo`: seta ou rosa, não `{o}`")),
    }
    let caixa = caixa_das(&p.pecas);
    let ids = aplicar(doc, p.pecas)?;
    Ok(relatorio(&ids, caixa))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn malha_com_cruzetas_e_rotulos_e_norte_das_duas_formas() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let r = malha(&mut d, &json!({ "caixa": [[100, 200], [500, 400]], "passo": 100, "escala": 1000 })).unwrap();
        assert_eq!((r["linhas_x"].as_u64(), r["linhas_y"].as_u64()), (Some(5), Some(3)));
        assert!(norte(&mut d, &json!({ "ponto": [0, 0], "escala": 1000 })).unwrap()["criadas"].as_u64().unwrap() >= 4);
        assert!(norte(&mut d, &json!({ "ponto": [0, 0], "tipo": "rosa", "rotacao": 12.0 })).is_ok());
        assert!(norte(&mut d, &json!({ "ponto": [0, 0], "tipo": "x" })).is_err());
    }
}
