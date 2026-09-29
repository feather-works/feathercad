//! A biblioteca de **componentes** em arquivos: cada peça (disjuntor modular, DR, DPS, transformador…) é um `.fct`
//! guardado em `feathercad/biblioteca/**` (embutido no binário) ou em `~/.config/feathercad/biblioteca/` (do usuário, vale
//! sobre a de fábrica). `cad_componentes` lista e `cad_componente` coloca pelo nome — a IA não redesenha nada: gasta uma
//! chamada curta em vez de centenas de linhas.
//!
//! Formato do arquivo: texto compacto comum (`cad_exportar`), em **milímetros reais** com a origem no canto inferior esquerdo, mais
//! um cabeçalho de comentários `// chave: valor` (`descricao`, `categoria`, `tamanho`, `parametros: a=1 b=texto`). No corpo,
//! `{{a}}` é trocado pelo valor do parâmetro (o padrão do cabeçalho ou o que a IA mandar em `valores`).

use std::fs;
use std::path::PathBuf;

use feathercad_core::documento::Documento;
use feathercad_core::edicao::transformar;
use feathercad_core::geom::{Ponto, Transf};
use serde_json::{json, Value};

use crate::compacto::importar;
use crate::json::{numero_opcional, ponto_de, texto_de, texto_opcional, Res};
use crate::mutacoes::aplicar_compacto;

include!(concat!(env!("OUT_DIR"), "/biblioteca.rs"));

struct Componente {
    nome: String,
    descricao: String,
    categoria: String,
    tamanho: String,
    parametros: Vec<(String, String)>,
    corpo: String,
    origem: &'static str,
}

fn ler(nome: &str, texto: &str, origem: &'static str) -> Componente {
    let cab = |chave: &str| {
        texto.lines().find_map(|l| l.trim().strip_prefix("//").and_then(|r| r.trim().strip_prefix(chave)).and_then(|r| r.strip_prefix(':')).map(|v| v.trim().to_string())).unwrap_or_default()
    };
    let parametros = cab("parametros").split_whitespace().filter_map(|p| p.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect();
    Componente { nome: nome.to_string(), descricao: cab("descricao"), categoria: cab("categoria"), tamanho: cab("tamanho"), parametros, corpo: texto.to_string(), origem }
}

fn pasta_do_usuario() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FEATHERCAD_BIBLIOTECA") {
        return Some(PathBuf::from(p));
    }
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/feathercad/biblioteca"))
}

fn do_usuario(dir: &std::path::Path, saida: &mut Vec<Componente>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            do_usuario(&p, saida);
        } else if p.extension().is_some_and(|x| x == "fct") {
            if let (Some(nome), Ok(t)) = (p.file_stem(), fs::read_to_string(&p)) {
                saida.push(ler(&nome.to_string_lossy(), &t, "usuario"));
            }
        }
    }
}

/// Todos os componentes; os do usuário vêm depois e valem sobre os de fábrica de mesmo nome.
fn todos() -> Vec<Componente> {
    let mut v: Vec<Componente> = FABRICA.iter().map(|(n, t)| ler(n, t, "fabrica")).collect();
    if let Some(dir) = pasta_do_usuario() {
        do_usuario(&dir, &mut v);
    }
    v
}

fn achar(nome: &str) -> Res<Componente> {
    let mut v = todos();
    v.reverse();
    v.into_iter().find(|c| c.nome.eq_ignore_ascii_case(nome)).ok_or_else(|| format!("componente `{nome}` não existe; veja cad_componentes"))
}

/// `cad_componentes`: a lista (curta) do que existe, com filtro opcional por texto.
pub fn listar(args: &Value) -> Value {
    let busca = texto_opcional(args, "busca").map(str::to_lowercase);
    let mut vistos: Vec<Componente> = Vec::new();
    for c in todos() {
        vistos.retain(|o| o.nome != c.nome);
        vistos.push(c);
    }
    let itens: Vec<Value> = vistos
        .iter()
        .filter(|c| busca.as_ref().map_or(true, |b| c.nome.to_lowercase().contains(b) || c.descricao.to_lowercase().contains(b) || c.categoria.to_lowercase().contains(b)))
        .map(|c| {
            let par: Vec<String> = c.parametros.iter().map(|(k, v)| format!("{k}={v}")).collect();
            json!({ "nome": c.nome, "categoria": c.categoria, "descricao": c.descricao, "tamanho_mm": c.tamanho, "parametros": par.join(" "), "origem": c.origem })
        })
        .collect();
    json!({ "componentes": itens, "uso": "cad_componente {nome, ponto:[x,y], valores?:{parametro:valor}, ampliacao?:1, rotacao?:0, camada?}. Medidas em mm reais; ponto = canto inferior esquerdo." })
}

fn valores_finais(c: &Componente, args: &Value) -> Res<Vec<(String, String)>> {
    let mut v = c.parametros.clone();
    if let Some(o) = args.get("valores").and_then(Value::as_object) {
        for (k, val) in o {
            let txt = val.as_str().map_or_else(|| val.to_string(), str::to_string);
            match v.iter_mut().find(|(n, _)| n == k) {
                Some(p) => p.1 = txt,
                None => return Err(format!("`{k}` não é parâmetro de `{}` (aceita: {})", c.nome, c.parametros.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", "))),
            }
        }
    }
    Ok(v)
}

/// `cad_componente`: põe o componente `nome` com o canto inferior esquerdo em `ponto`.
pub fn componente(doc: &mut Documento, args: &Value) -> Res<Value> {
    let c = achar(texto_de(args, "nome")?)?;
    let mut corpo = c.corpo.clone();
    for (k, v) in valores_finais(&c, args)? {
        corpo = corpo.replace(&format!("{{{{{k}}}}}"), &v);
    }
    let mut lido = importar(&corpo).map_err(|e| format!("componente `{}`: {e}", c.nome))?;
    let un = doc.unidade().ok_or("o desenho ainda não tem unidade: use cad_novo primeiro")?;
    let s = numero_opcional(args, "ampliacao").filter(|a| *a > 0.0).unwrap_or(1.0) / un.em_milimetros();
    let giro = numero_opcional(args, "rotacao").unwrap_or(0.0);
    let alvo = ponto_de(args, "ponto")?;
    let o = Ponto::novo(0.0, 0.0);
    for f in &mut lido.formas {
        f.2 = transformar(&transformar(&transformar(&f.2, &Transf::escalar(o, s)), &Transf::girar(o, giro)), &Transf::mover(alvo.x, alvo.y));
    }
    if let Some(camada) = texto_opcional(args, "camada") {
        lido.camadas.clear();
        for f in &mut lido.formas {
            f.0 = camada.to_string();
        }
    }
    let ids = aplicar_compacto(doc, &lido)?;
    Ok(json!({ "componente": c.nome, "ids": ids, "ponto": [alvo.x, alvo.y] }))
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathercad_core::documento::Unidade;

    #[test]
    fn a_biblioteca_de_fabrica_carrega_e_coloca_com_parametros() {
        let lista = listar(&json!({}));
        assert!(lista["componentes"].as_array().is_some_and(|a| !a.is_empty()), "biblioteca vazia");
        let mut d = Documento::default().com_unidade(Some(Unidade::Milimetro));
        let r = componente(&mut d, &json!({ "nome": "disjuntor_modular_2p", "ponto": [100.0, 50.0], "valores": { "corrente": "C25" } })).unwrap();
        assert!(!r["ids"].as_array().unwrap().is_empty());
        let (txt, _) = crate::compacto::exportar(&d, None, false);
        assert!(txt.contains("C25"), "{txt}");
        assert!(componente(&mut d, &json!({ "nome": "disjuntor_modular_2p", "ponto": [0, 0], "valores": { "xis": 1 } })).is_err());
    }
}
