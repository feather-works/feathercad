//! Mede a seleção em desenho grande, pelo mesmo caminho JSON da interface: clique num objeto, janela que pega tudo,
//! o quadro com muita coisa selecionada, mover e apagar a seleção. Uso (release, que é o que importa):
//! `cargo run --release --example medir_selecao -- [arquivo.dxf|.fcdz]` (sem arquivo: 50 mil linhas sintéticas).

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

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() -> Result<(), String> {
    let bytes = match std::env::args().nth(1) {
        Some(caminho) => std::fs::read(&caminho).map_err(|e| format!("{caminho}: {e}"))?,
        None => sintetico(50_000),
    };
    let mut ponte = Ponte::default();
    let t = Instant::now();
    ponte.chamar("documento_abrir", &bytes)?;
    println!("abrir ({} bytes): {:.1} ms", bytes.len(), ms(t));

    let vista0 = r#"{"centro":{"x":0,"y":0},"escala":1,"largura":1400,"altura":800}"#;
    let ajustada = ponte.chamar("vista_ajustar", format!(r#"{{"vista":{vista0}}}"#).as_bytes())?;
    let vista = String::from_utf8(ajustada).map_err(|e| e.to_string())?;
    let mouse = |ponte: &mut Ponte, nome: &str, x: f64, y: f64| -> Result<Vec<u8>, String> { ponte.chamar(nome, format!(r#"{{"vista":{vista},"x":{x},"y":{y}}}"#).as_bytes()) };
    let quadro = |ponte: &mut Ponte| -> Result<usize, String> { Ok(ponte.chamar("quadro", format!(r#"{{"vista":{vista},"versao_conhecida":null,"fundo_claro":false}}"#).as_bytes())?.len()) };

    // Clique num lugar qualquer do meio (acerta ou erra; mede a escolha, que varre as entidades).
    let t = Instant::now();
    const N: usize = 20;
    for i in 0..N {
        mouse(&mut ponte, "clique", 700.0 + i as f64, 400.0)?;
        ponte.chamar("cancelar", b"{}")?;
    }
    println!("clique (escolher objeto): {:.2} ms em média ({N})", ms(t) / N as f64);

    // Janela esquerda→direita, de ponta a ponta da tela: pega tudo que cabe.
    let t = Instant::now();
    mouse(&mut ponte, "clique", 1.0, 1.0)?;
    let r = mouse(&mut ponte, "clique", 1399.0, 799.0)?;
    println!("janela que pega tudo: {:.1} ms", ms(t));
    let texto = String::from_utf8_lossy(&r);
    if let Some(i) = texto.find("selecionados") {
        println!("  resposta: …{}…", &texto[i.saturating_sub(20)..(i + 40).min(texto.len())]);
    }

    let t = Instant::now();
    let tam = quadro(&mut ponte)?;
    println!("quadro com a seleção: {:.1} ms ({tam} bytes)", ms(t));

    let t = Instant::now();
    ponte.chamar("propriedades_selecao", b"{}")?;
    println!("propriedades da seleção: {:.1} ms", ms(t));

    let t = Instant::now();
    ponte.chamar("executar", br#"{"comando":"apagar"}"#)?;
    println!("apagar a seleção: {:.1} ms", ms(t));

    let t = Instant::now();
    ponte.chamar("desfazer", b"{}")?;
    println!("desfazer: {:.1} ms", ms(t));
    Ok(())
}
