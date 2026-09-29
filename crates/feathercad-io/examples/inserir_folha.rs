//! Insere uma folha (prancheta) num desenho e grava o resultado — o mesmo motor do botão "Inserir folha" do aplicativo.
//!
//! `cargo run --example inserir_folha -- entrada.fcdz saida.fcdz A1 1 [--origem] [campo=valor ...]`
//! (`1` = escala; `--origem` põe o canto inferior esquerdo da folha em (0, 0); campos: cliente, fase, disciplina, titulo,
//! subtitulo, numero, revisao, folha, divisoria=1)

use feathercad_core::documento::{Geometria, Mudanca};
use feathercad_core::folha::Campos;
use feathercad_core::geom::Ponto;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(entrada), Some(saida), Some(papel), Some(escala)) = (args.first(), args.get(1), args.get(2), args.get(3)) else {
        return Err("uso: inserir_folha <entrada.fcdz> <saida.fcdz> <papel> <escala> [--origem] [campo=valor ...]".into());
    };
    let escala: f64 = escala.replace(',', ".").parse().map_err(|_| "escala inválida".to_string())?;
    let mut campos = Campos::default();
    let mut origem = false;
    for a in &args[4..] {
        if a == "--origem" {
            origem = true;
        } else if let Some((k, v)) = a.split_once('=') {
            match k {
                "cliente" => campos.cliente = v.into(),
                "fase" => campos.fase = v.into(),
                "disciplina" => campos.disciplina = v.into(),
                "titulo" => campos.titulo = v.into(),
                "subtitulo" => campos.subtitulo = v.into(),
                "numero" => campos.numero = v.into(),
                "revisao" => campos.revisao = v.into(),
                "folha" => campos.folha = v.into(),
                "divisoria" => campos.divisoria = v != "0",
                outro => return Err(format!("campo `{outro}` não existe")),
            }
        }
    }
    let bytes = std::fs::read(entrada).map_err(|e| format!("não consegui ler `{entrada}`: {e}"))?;
    let mut doc = feathercad_io::abrir(&bytes)?.doc;
    let mut info = doc.folha_inserir(papel, Some(escala), &campos)?;
    if origem {
        let Some(Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao }) = doc.entidade(info.id).map(|e| e.geometria.clone()) else { return Err("a folha sumiu".into()) };
        let novo = Ponto::novo(ponto.x - info.min.x, ponto.y - info.min.y);
        doc.aplicar_mudancas(vec![Mudanca::Trocar(info.id, Geometria::Insercao { bloco, ponto: novo, escala_x, escala_y, rotacao })]);
        info = doc.folhas_inseridas().into_iter().find(|f| f.id == info.id).ok_or("a folha não apareceu")?;
    }
    let gravado = feathercad_io::fcdz::gravar(&doc)?;
    std::fs::write(saida, gravado).map_err(|e| format!("não consegui gravar `{saida}`: {e}"))?;
    println!("{}: folha {} ({}), de ({:.1}; {:.1}) a ({:.1}; {:.1})", saida, info.nome, info.escala_texto, info.min.x, info.min.y, info.max.x, info.max.y);
    Ok(())
}
