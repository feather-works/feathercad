//! O pior caso do cursor: varre uma grade de posições da tela em vários zooms e mostra média, p99 e máximo do
//! `cursor_mover` (o snap por interseção é quadrático onde há muita coisa perto do cursor).
//! `cargo run --release --example varrer_cursor -- arquivo.dxf`

use std::time::Instant;

use feathercad_ponte::Ponte;

fn main() -> Result<(), String> {
    let caminho = std::env::args().nth(1).ok_or("uso: varrer_cursor <arquivo.dxf|.fcdz>")?;
    let bytes = std::fs::read(&caminho).map_err(|e| format!("{caminho}: {e}"))?;
    let mut ponte = Ponte::default();
    ponte.chamar("documento_abrir", &bytes)?;
    let vista0 = r#"{"centro":{"x":0,"y":0},"escala":1,"largura":1400,"altura":800}"#;
    let ajustada = String::from_utf8(ponte.chamar("vista_ajustar", format!(r#"{{"vista":{vista0}}}"#).as_bytes())?).map_err(|e| e.to_string())?;
    let mut vista = ajustada.clone();
    for (nome, fator) in [("desenho inteiro", 1.0), ("zoom 4x no centro", 4.0), ("zoom 16x no centro", 4.0)] {
        if fator != 1.0 {
            let r = ponte.chamar("vista_zoom_centro", format!(r#"{{"vista":{vista},"fator":{fator}}}"#).as_bytes())?;
            vista = String::from_utf8(r).map_err(|e| e.to_string())?;
        }
        let mut tempos: Vec<f64> = Vec::new();
        for gx in 0..40 {
            for gy in 0..30 {
                let (x, y) = (20.0 + gx as f64 * 34.0, 20.0 + gy as f64 * 25.0);
                let t = Instant::now();
                ponte.chamar("cursor_mover", format!(r#"{{"vista":{vista},"x":{x},"y":{y}}}"#).as_bytes())?;
                tempos.push(t.elapsed().as_secs_f64() * 1000.0);
            }
        }
        tempos.sort_by(f64::total_cmp);
        let media = tempos.iter().sum::<f64>() / tempos.len() as f64;
        println!("{nome:20} média {media:8.2} ms | p99 {:8.2} ms | máximo {:8.2} ms  ({} posições)", tempos[tempos.len() * 99 / 100], tempos[tempos.len() - 1], tempos.len());
    }
    Ok(())
}
