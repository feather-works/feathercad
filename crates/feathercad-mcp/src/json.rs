//! Leitura dos argumentos das ferramentas: pontos, números e listas, com mensagens de erro em português.

use feathercad_core::geom::Ponto;
use serde_json::Value;

pub type Res<T> = Result<T, String>;

/// Um ponto: `[x, y]` ou `{"x": 1, "y": 2}`.
pub fn ponto(v: &Value, nome: &str) -> Res<Ponto> {
    let par = match v {
        Value::Array(a) if a.len() >= 2 => (a[0].as_f64(), a[1].as_f64()),
        Value::Object(o) => (o.get("x").and_then(Value::as_f64), o.get("y").and_then(Value::as_f64)),
        _ => (None, None),
    };
    match par {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok(Ponto::novo(x, y)),
        _ => Err(format!("`{nome}` precisa ser um ponto [x, y] com números")),
    }
}

/// O ponto do campo `nome` de `args` (obrigatório).
pub fn ponto_de(args: &Value, nome: &str) -> Res<Ponto> {
    ponto(args.get(nome).ok_or_else(|| format!("falta `{nome}`"))?, nome)
}

pub fn numero_de(args: &Value, nome: &str) -> Res<f64> {
    args.get(nome).and_then(Value::as_f64).filter(|n| n.is_finite()).ok_or_else(|| format!("falta `{nome}` (número)"))
}

pub fn numero_opcional(args: &Value, nome: &str) -> Option<f64> {
    args.get(nome).and_then(Value::as_f64).filter(|n| n.is_finite())
}

pub fn texto_de<'a>(args: &'a Value, nome: &str) -> Res<&'a str> {
    args.get(nome).and_then(Value::as_str).ok_or_else(|| format!("falta `{nome}` (texto)"))
}

pub fn texto_opcional<'a>(args: &'a Value, nome: &str) -> Option<&'a str> {
    args.get(nome).and_then(Value::as_str)
}

pub fn lista_de_pontos(v: &Value, nome: &str) -> Res<Vec<Ponto>> {
    let a = v.as_array().ok_or_else(|| format!("`{nome}` precisa ser uma lista de pontos"))?;
    a.iter().map(|p| ponto(p, nome)).collect()
}

/// Uma janela retangular `{"x1":,"y1":,"x2":,"y2":}` ou `[[x1,y1],[x2,y2]]`, como (mínimo, máximo).
pub fn janela(v: &Value) -> Res<(Ponto, Ponto)> {
    let (a, b) = match v {
        Value::Array(p) if p.len() == 2 => (ponto(&p[0], "janela")?, ponto(&p[1], "janela")?),
        Value::Object(_) => (Ponto::novo(numero_de(v, "x1")?, numero_de(v, "y1")?), Ponto::novo(numero_de(v, "x2")?, numero_de(v, "y2")?)),
        _ => return Err("`janela` precisa ser {x1,y1,x2,y2} ou [[x1,y1],[x2,y2]]".into()),
    };
    Ok((Ponto::novo(a.x.min(b.x), a.y.min(b.y)), Ponto::novo(a.x.max(b.x), a.y.max(b.y))))
}

/// Arredonda para o que uma pessoa lê (até 6 casas), sem ruído de ponto flutuante.
pub fn arred(v: f64) -> f64 {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

pub fn par(p: Ponto) -> Value {
    serde_json::json!([arred(p.x), arred(p.y)])
}
