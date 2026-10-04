//! `cad_ao_longo`: coisas ao longo de um eixo — repetir uma forma ou componente a cada `passo` (dormentes, postes),
//! paralelas a ±d (trilhos, meio-fio) e marcas com rótulo (estacas, km). Distâncias pelo comprimento do eixo.

use feathercad_core::documento::{AlinhaH, AlinhaV, Documento, Geometria};
use feathercad_core::edicao::transformar;
use feathercad_core::geom::{Ponto, Transf};
use feathercad_core::trajeto::{estaca_texto, km_texto, Trajeto};
use serde_json::{json, Value};

use crate::json::{lista_de_pontos, numero_de, numero_opcional, texto_opcional, Res};
use crate::mutacoes::forma;
use crate::pecas::{aplicar, cor_de, fator_papel, formatar, Pincel};

const MAXIMO: usize = 100_000;

/// O eixo: `eixo` (id de linha, polilinha ou arco) ou `pontos`.
pub fn trajeto_de(doc: &Documento, args: &Value) -> Res<Trajeto> {
    if let Some(id) = args.get("eixo").and_then(Value::as_u64) {
        let e = doc.entidade(id).ok_or_else(|| format!("a entidade #{id} não existe"))?;
        return Trajeto::de_geometria(&e.geometria).ok_or_else(|| format!("#{id} não é linha, polilinha nem arco"));
    }
    let pts = lista_de_pontos(args.get("pontos").ok_or("diga o `eixo` (id) ou os `pontos`")?, "pontos")?;
    Trajeto::novo(&pts).ok_or_else(|| "o eixo precisa de ao menos 2 pontos diferentes".into())
}

/// Posição de uma forma local (x ao longo do eixo, y à esquerda) sobre o eixo.
fn no_eixo(g: &Geometria, ponto: Ponto, angulo: f64) -> Geometria {
    let o = Ponto::novo(0.0, 0.0);
    transformar(&transformar(g, &Transf::girar(o, angulo)), &Transf::mover(ponto.x, ponto.y))
}

fn repetir(doc: &mut Documento, t: &Trajeto, args: &Value, de: f64, ate: f64) -> Res<Value> {
    let passo = numero_de(args, "passo")?;
    if passo <= 0.0 {
        return Err("`passo` precisa ser maior que zero".into());
    }
    let n = ((ate - de) / passo + 1e-9).floor() as usize + 1;
    if n > MAXIMO {
        return Err(format!("{n} repetições passam do limite de {MAXIMO}: aumente o `passo`"));
    }
    let lado = numero_opcional(args, "defasagem").unwrap_or(0.0);
    let giro_extra = numero_opcional(args, "rotacao").unwrap_or(0.0);
    let posicoes: Vec<(Ponto, f64)> = (0..n)
        .map(|i| {
            let (p, ang) = t.em(de + i as f64 * passo);
            let a = ang.to_radians();
            (Ponto::novo(p.x - a.sin() * lado, p.y + a.cos() * lado), ang + giro_extra)
        })
        .collect();
    if let Some(c) = args.get("componente") {
        let marca = doc.marca_do_historico();
        for (p, ang) in &posicoes {
            let mut a = c.clone();
            a["ponto"] = json!([p.x, p.y]);
            a["rotacao"] = json!(ang);
            crate::componentes::componente(doc, &a)?;
        }
        doc.juntar_historico(marca);
        return Ok(json!({ "repeticoes": n, "componente": c["nome"] }));
    }
    let molde = forma(args.get("forma").ok_or("diga a `forma` (como em cad_criar, em coordenadas locais: x ao longo do eixo, y à esquerda) ou o `componente` {nome,…}")?)?;
    let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("AO_LONGO"));
    p.cor = cor_de(args, "cor")?;
    for (pos, ang) in &posicoes {
        p.pecas.push(crate::pecas::Peca { camada: p.camada.clone(), cor: p.cor, g: no_eixo(&molde, *pos, *ang) });
    }
    let ids = aplicar(doc, p.pecas)?;
    Ok(json!({ "repeticoes": n, "criadas": ids.len() }))
}

fn paralelas(doc: &mut Documento, t: &Trajeto, args: &Value, de: f64, ate: f64) -> Res<Value> {
    let lista = args.get("distancias").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("`distancias`: lista de números (positivo = esquerda) ou {d, camada, cor}")?;
    let base = texto_opcional(args, "camada").unwrap_or("PARALELAS");
    let cor_base = cor_de(args, "cor")?;
    let mut p = Pincel::novo(base);
    let sub = Trajeto::novo(&t.trecho(de, ate)).ok_or("o trecho pedido é vazio")?;
    let mut feitas = Vec::new();
    for (i, d) in lista.iter().enumerate() {
        let (dist, camada, cor) = match d {
            Value::Number(n) => (n.as_f64().unwrap_or(0.0), base.to_string(), cor_base),
            o => (numero_de(o, "d")?, texto_opcional(o, "camada").unwrap_or(base).to_string(), cor_de(o, "cor")?.or(cor_base)),
        };
        if dist.abs() < 1e-12 {
            return Err(format!("distância {i} é zero: use o próprio eixo"));
        }
        p.em(&camada, cor).polilinha(&sub.paralela(dist));
        feitas.push(dist);
    }
    let ids = aplicar(doc, p.pecas)?;
    Ok(json!({ "paralelas": feitas, "criadas": ids.len(), "comprimento_do_trecho": sub.comprimento() }))
}

fn marcas(doc: &mut Documento, t: &Trajeto, args: &Value, de: f64, ate: f64) -> Res<Value> {
    let k = fator_papel(doc, args)?;
    let passo = numero_de(args, "passo")?;
    if passo <= 0.0 {
        return Err("`passo` precisa ser maior que zero".into());
    }
    let tam = numero_opcional(args, "tamanho").unwrap_or(3.0) * k;
    let ht = numero_opcional(args, "altura_texto").unwrap_or(2.5) * k;
    let inicial = numero_opcional(args, "progressiva_inicial").unwrap_or(0.0);
    let passo_estaca = numero_opcional(args, "passo_estaca").unwrap_or(20.0);
    let rotulo = texto_opcional(args, "rotulo").unwrap_or("distancia");
    let a_cada = args.get("rotular_a_cada").and_then(Value::as_u64).unwrap_or(1).max(1) as usize;
    let lado = match texto_opcional(args, "lado").unwrap_or("esquerda") {
        "esquerda" => 1.0,
        "direita" => -1.0,
        o => return Err(format!("`lado`: esquerda ou direita, não `{o}`")),
    };
    let n = ((ate - de) / passo + 1e-9).floor() as usize + 1;
    if n > MAXIMO {
        return Err(format!("{n} marcas passam do limite de {MAXIMO}"));
    }
    let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("MARCAS"));
    p.cor = cor_de(args, "cor")?;
    let mut textos = Vec::new();
    for i in 0..n {
        let s = de + i as f64 * passo;
        let (pt, ang) = t.em(s);
        let a = ang.to_radians();
        let normal = (-a.sin(), a.cos());
        p.linha(Ponto::novo(pt.x - normal.0 * tam / 2.0, pt.y - normal.1 * tam / 2.0), Ponto::novo(pt.x + normal.0 * tam / 2.0, pt.y + normal.1 * tam / 2.0));
        if rotulo == "nenhum" || i % a_cada != 0 {
            continue;
        }
        let texto = match rotulo {
            "estaca" => estaca_texto(s + inicial, passo_estaca),
            "km" => km_texto(s + inicial),
            "distancia" => formatar(s + inicial, crate::pecas::casas_do_passo(passo), false),
            o => return Err(format!("`rotulo`: estaca, km, distancia ou nenhum, não `{o}`")),
        };
        let afastamento = lado * (tam / 2.0 + 0.4 * ht);
        let base = Ponto::novo(pt.x + normal.0 * afastamento, pt.y + normal.1 * afastamento);
        // texto perpendicular ao eixo, saindo da marca; o lado que fica legível (nunca de cabeça para baixo)
        let (g2, h2) = {
            let g = (ang + 90.0 * lado).rem_euclid(360.0);
            if g > 90.0 + 1e-6 && g <= 270.0 + 1e-6 { (g - 180.0, AlinhaH::Direita) } else { (g, AlinhaH::Esquerda) }
        };
        p.texto(base, ht, g2, &texto, h2, AlinhaV::Meio);
        textos.push(texto);
    }
    let ids = aplicar(doc, p.pecas)?;
    Ok(json!({ "marcas": n, "criadas": ids.len(), "primeiro_rotulo": textos.first(), "ultimo_rotulo": textos.last() }))
}

pub fn ao_longo(doc: &mut Documento, args: &Value) -> Res<Value> {
    let t = trajeto_de(doc, args)?;
    let de = numero_opcional(args, "de").unwrap_or(0.0).max(0.0);
    let ate = numero_opcional(args, "ate").unwrap_or(t.comprimento()).min(t.comprimento());
    if ate <= de {
        return Err(format!("`ate` ({ate}) precisa ser maior que `de` ({de}); o eixo mede {}", t.comprimento()));
    }
    let mut r = match texto_opcional(args, "modo").ok_or("diga o `modo`: repetir, paralelas ou marcas")? {
        "repetir" => repetir(doc, &t, args, de, ate)?,
        "paralelas" => paralelas(doc, &t, args, de, ate)?,
        "marcas" => marcas(doc, &t, args, de, ate)?,
        o => return Err(format!("`modo` `{o}` não existe (repetir, paralelas, marcas)")),
    };
    r["comprimento_do_eixo"] = json!(t.comprimento());
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn dormentes_trilhos_e_estacas_sobre_um_eixo() {
        let mut d = Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let eixo = json!([[0, 0], [100, 0], [100, 100]]);
        let r = ao_longo(&mut d, &json!({ "pontos": eixo, "modo": "repetir", "passo": 20, "forma": { "tipo": "linha", "a": [0, -1.3], "b": [0, 1.3] } })).unwrap();
        assert_eq!(r["repeticoes"], 11);
        assert_eq!(r["comprimento_do_eixo"], 200.0);
        let r = ao_longo(&mut d, &json!({ "pontos": eixo, "modo": "paralelas", "distancias": [0.5, -0.5] })).unwrap();
        assert_eq!(r["criadas"], 2);
        let r = ao_longo(&mut d, &json!({ "pontos": eixo, "modo": "marcas", "passo": 50, "rotulo": "estaca", "escala": 1000 })).unwrap();
        assert_eq!(r["primeiro_rotulo"], "0+00,00");
        assert_eq!(r["ultimo_rotulo"], "10+00,00");
        // nenhum texto de marca fica de cabeça para baixo
        let v = crate::leiaute::verificar_leiaute(&d);
        assert!(v["contagem"].get("texto_de_cabeca_para_baixo").is_none(), "{v}");
    }

    #[test]
    fn modo_desconhecido_e_eixo_curto_dizem_o_erro() {
        let mut d = Documento::default();
        assert!(ao_longo(&mut d, &json!({ "pontos": [[0, 0]], "modo": "repetir" })).is_err());
        assert!(ao_longo(&mut d, &json!({ "pontos": [[0, 0], [1, 0]], "modo": "x" })).unwrap_err().contains("não existe"));
    }
}
