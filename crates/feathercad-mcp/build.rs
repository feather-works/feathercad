//! Embute no binário todos os `.fct` de `feathercad/biblioteca/**` (a biblioteca de componentes de fábrica).
//! Basta largar um arquivo novo na pasta e recompilar: nenhuma lista para editar.

use std::{env, fs, path::Path};

fn coletar(dir: &Path, saida: &mut Vec<(String, String)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut itens: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
    itens.sort();
    for p in itens {
        if p.is_dir() {
            coletar(&p, saida);
        } else if p.extension().is_some_and(|e| e == "fct") {
            let nome = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            saida.push((nome, p.canonicalize().unwrap_or(p).to_string_lossy().replace('\\', "/")));
        }
    }
}

fn main() {
    let raiz = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../biblioteca");
    println!("cargo:rerun-if-changed={}", raiz.display());
    let mut arquivos = Vec::new();
    coletar(&raiz, &mut arquivos);
    let mut codigo = String::from("pub const FABRICA: &[(&str, &str)] = &[\n");
    for (nome, caminho) in &arquivos {
        codigo.push_str(&format!("    ({nome:?}, include_str!({caminho:?})),\n"));
        println!("cargo:rerun-if-changed={caminho}");
    }
    codigo.push_str("];\n");
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("biblioteca.rs"), codigo).unwrap();
}
