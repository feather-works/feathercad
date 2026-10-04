//! `cad_projetar`: pontos e linhas 3D projetados para 2D (isométrica, oblíqua ou câmera livre). O DXF segue 2D:
//! o que se desenha é a projeção. Sem remoção de linhas ocultas.

use feathercad_core::documento::Documento;
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::json::{numero_de, numero_opcional, par, ponto, texto_opcional, Res};
use crate::pecas::{aplicar, caixa_das, cor_de, relatorio, Pincel};

#[derive(Clone, Copy)]
pub(crate) enum Camera {
    /// Azimute e elevação em graus, e escala.
    Orbita(f64, f64, f64),
    /// Cavaleira: o eixo Y recua a 45° pela metade.
    Obliqua(f64),
}

pub(crate) fn camera(v: Option<&Value>) -> Res<Camera> {
    match v {
        None => Ok(Camera::Orbita(45.0, 35.264, 1.0)),
        Some(Value::String(s)) => match s.as_str() {
            "iso" => Ok(Camera::Orbita(45.0, 35.264, 1.0)),
            "obliqua" => Ok(Camera::Obliqua(1.0)),
            o => Err(format!("câmera `{o}`: iso, obliqua ou {{azimute, elevacao, escala}}")),
        },
        Some(o) => Ok(Camera::Orbita(numero_de(o, "azimute")?, numero_opcional(o, "elevacao").unwrap_or(35.264), numero_opcional(o, "escala").unwrap_or(1.0))),
    }
}

/// Projeta (x, y, z) em coordenadas de tela. Elevação 90° = vista de topo; 0° = de frente.
pub(crate) fn projetar(c: Camera, x: f64, y: f64, z: f64) -> (f64, f64) {
    match c {
        Camera::Orbita(az, el, k) => {
            let (a, e) = (az.to_radians(), el.to_radians());
            let (u, v) = (x * a.cos() + y * a.sin(), -x * a.sin() + y * a.cos());
            (k * u, k * (v * e.sin() + z * e.cos()))
        }
        Camera::Obliqua(k) => {
            let (s, c) = 45f64.to_radians().sin_cos();
            (k * (x + 0.5 * y * c), k * (z + 0.5 * y * s))
        }
    }
}

fn ponto3d(v: &Value) -> Res<(f64, f64, f64)> {
    let a = v.as_array().filter(|a| a.len() >= 3).ok_or("ponto 3D precisa ser [x, y, z]")?;
    match (a[0].as_f64(), a[1].as_f64(), a[2].as_f64()) {
        (Some(x), Some(y), Some(z)) => Ok((x, y, z)),
        _ => Err("ponto 3D precisa ser [x, y, z] com números".into()),
    }
}

pub fn projetar_ferramenta(doc: &mut Documento, args: &Value) -> Res<Value> {
    let cam = camera(args.get("camera"))?;
    let origem = args.get("origem").map(|o| ponto(o, "origem")).transpose()?.unwrap_or(Ponto::novo(0.0, 0.0));
    let projeta = |p: (f64, f64, f64)| {
        let (x, y) = projetar(cam, p.0, p.1, p.2);
        Ponto::novo(origem.x + x, origem.y + y)
    };
    let mut r = json!({});
    if let Some(pts) = args.get("pontos3d").and_then(Value::as_array) {
        r["pontos"] = pts.iter().map(|p| ponto3d(p).map(|q| par(projeta(q)))).collect::<Res<Vec<_>>>()?.into();
    }
    if let Some(linhas) = args.get("linhas").and_then(Value::as_array) {
        let mut p = Pincel::novo(texto_opcional(args, "camada").unwrap_or("PROJECAO"));
        p.cor = cor_de(args, "cor")?;
        for (i, l) in linhas.iter().enumerate() {
            let pts: Vec<Ponto> = l.as_array().ok_or_else(|| format!("linha {i}: lista de pontos [x,y,z]"))?.iter().map(|q| ponto3d(q).map(projeta)).collect::<Res<_>>()?;
            p.polilinha(&pts);
        }
        let caixa = caixa_das(&p.pecas);
        let ids = aplicar(doc, p.pecas)?;
        let rel = relatorio(&ids, caixa);
        r["desenho"] = rel;
    }
    if r.as_object().is_some_and(serde_json::Map::is_empty) {
        return Err("diga `pontos3d` (só devolve os 2D) e/ou `linhas` (desenha: cada uma é uma lista de [x,y,z])".into());
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn topo_e_frente_e_desenho() {
        let topo = projetar(Camera::Orbita(0.0, 90.0, 1.0), 3.0, 4.0, 9.0);
        assert!((topo.0 - 3.0).abs() < 1e-9 && (topo.1 - 4.0).abs() < 1e-9);
        let frente = projetar(Camera::Orbita(0.0, 0.0, 1.0), 3.0, 4.0, 9.0);
        assert!((frente.1 - 9.0).abs() < 1e-9);
        let mut d = Documento::default();
        let r = projetar_ferramenta(&mut d, &json!({ "camera": "iso", "pontos3d": [[0, 0, 0], [1, 0, 0]], "linhas": [[[0, 0, 0], [10, 0, 0], [10, 0, 5]]] })).unwrap();
        assert_eq!(r["pontos"].as_array().unwrap().len(), 2);
        assert_eq!(r["desenho"]["criadas"], 1);
        assert!(projetar_ferramenta(&mut d, &json!({})).is_err());
    }
}
