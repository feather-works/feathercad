//! Mede o que o mouse custa ao motor: montar o quadro depois de um pan, mover o cursor e deslocar a vista,
//! pelo mesmo caminho JSON da interface. Uso (compare debug e release; o `app:dev` roda em debug):
//! `cargo run [--release] --example medir_movimento -- [arquivo.dxf|.fcdz]` (sem arquivo: 50 mil linhas sintéticas).

use std::time::Instant;

use feathercad_ponte::Ponte;

fn sintetico(n: usize) -> Vec<u8> {
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for i in 0..n {
        let (x, y) = ((i % 250) as f64 * 40.0, (i / 250) as f64 * 40.0);
        s += &format!("0\nLINE\n8\n0\n10\n{x}\n20\n{y}\n11\n{}\n21\n{}\n", x + 30.0, y + 20.0);
    }
    s += "0\nENDSEC\n0\nEOF\n";
    s.into_bytes()
}

fn main() -> Result<(), String> {
    let bytes = match std::env::args().nth(1) {
        Some(caminho) => std::fs::read(&caminho).map_err(|e| format!("{caminho}: {e}"))?,
        None => sintetico(50_000),
    };
    let mut ponte = Ponte::default();
    ponte.chamar("documento_abrir", &bytes)?;
    let vista0 = r#"{"centro":{"x":0,"y":0},"escala":1,"largura":1400,"altura":800}"#;
    let ajustada = ponte.chamar("vista_ajustar", format!(r#"{{"vista":{vista0}}}"#).as_bytes())?;
    let mut vista = String::from_utf8(ajustada).map_err(|e| e.to_string())?;

    const N: usize = 200;
    let (mut t_pan, mut t_quadro, mut t_cursor, mut bytes_do_quadro) = (0.0, 0.0, 0.0, 0);
    for i in 0..N {
        let t = Instant::now();
        let r = ponte.chamar("vista_deslocar", format!(r#"{{"vista":{vista},"dx":{},"dy":{}}}"#, if i % 2 == 0 { 7 } else { -7 }, if i % 2 == 0 { 3 } else { -3 }).as_bytes())?;
        t_pan += t.elapsed().as_secs_f64();
        vista = String::from_utf8(r).map_err(|e| e.to_string())?;
        let t = Instant::now();
        let q = ponte.chamar("quadro", format!(r#"{{"vista":{vista},"versao_conhecida":null,"fundo_claro":false}}"#).as_bytes())?;
        t_quadro += t.elapsed().as_secs_f64();
        bytes_do_quadro = q.len();
        let t = Instant::now();
        ponte.chamar("cursor_mover", format!(r#"{{"vista":{vista},"x":{},"y":400}}"#, 300 + i).as_bytes())?;
        t_cursor += t.elapsed().as_secs_f64();
    }
    let ms = |t: f64| t / N as f64 * 1000.0;
    println!("{} bytes de arquivo, quadro de {} bytes", bytes.len(), bytes_do_quadro);
    println!("vista_deslocar {:.3} ms | quadro {:.3} ms | cursor_mover {:.3} ms (média de {N})", ms(t_pan), ms(t_quadro), ms(t_cursor));
    Ok(())
}
