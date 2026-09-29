//! Exporta um desenho (`.fcdz` ou `.dxf`) para PDF, uma página por folha (camada `FOLHA`).
//!
//! `cargo run --release --example exportar_pdf -- desenho.fcdz saida.pdf [A4|A3|A2|A1] [--cores]`
//! (`--cores` mantém as cores do desenho; sem ele, o modo papel deixa tudo legível no branco).

use feathercad_io::pdf::{exportar_folhas, Opcoes, Papel};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(entrada), Some(saida)) = (args.first(), args.get(1)) else {
        return Err("uso: exportar_pdf <entrada.fcdz|.dxf> <saida.pdf> [A4|A3|A2|A1] [--cores]".into());
    };
    let papel = args.iter().skip(2).find_map(|a| Papel::de_nome(a)).unwrap_or(Papel::A3);
    let bytes = std::fs::read(entrada).map_err(|e| format!("não consegui ler `{entrada}`: {e}"))?;
    let aberto = feathercad_io::abrir(&bytes)?;
    let pdf = exportar_folhas(&aberto.doc, &Opcoes { papel, modo_papel: !args.iter().any(|a| a == "--cores"), ..Opcoes::default() })?;
    std::fs::write(saida, &pdf).map_err(|e| format!("não consegui gravar `{saida}`: {e}"))?;
    println!("{saida}: {} bytes", pdf.len());
    Ok(())
}
