//! Folhas (pranchetas), PDF e contagem para a IA: as mesmas contas do aplicativo (`feathercad-core::folha`, `contagem` e o
//! `pdf` do `feathercad-io`), sem passar por script. Uma folha é um bloco `FOLHA-n` inserido na escala do desenho.

use feathercad_core::contagem::{iguais_a, por_medida};
use feathercad_core::documento::{Documento, Geometria, Mudanca};
use feathercad_core::folha::{Campos, ESCALAS, PAPEIS};
use feathercad_core::geom::Ponto;
use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{numero_de, numero_opcional, ponto, texto_de, texto_opcional, Res};

fn campos_de(v: Option<&Value>) -> Campos {
    let t = |nome: &str| v.and_then(|o| o.get(nome)).and_then(Value::as_str).unwrap_or("").to_string();
    Campos {
        cliente: t("cliente"),
        fase: t("fase"),
        disciplina: t("disciplina"),
        titulo: t("titulo"),
        subtitulo: t("subtitulo"),
        responsavel: t("responsavel"),
        numero: t("numero"),
        revisao: t("revisao"),
        revisao_descricao: t("revisao_descricao"),
        folha: t("folha"),
        divisoria: v.and_then(|o| o.get("divisoria")).and_then(Value::as_bool).unwrap_or(false),
    }
}

fn indice_de(args: &Value) -> Res<usize> {
    let n = numero_de(args, "indice")?;
    if n < 0.0 || n.fract() != 0.0 {
        return Err("`indice` é a posição da folha, a partir de 0 (veja `acao: listar`)".into());
    }
    Ok(n as usize)
}

/// Põe o canto inferior esquerdo da folha `id` em `destino`.
fn levar_para(doc: &mut Documento, id: u64, destino: Ponto) -> Res<()> {
    let info = doc.folhas_inseridas().into_iter().find(|f| f.id == id).ok_or("a folha sumiu")?;
    let Some(Geometria::Insercao { bloco, ponto, escala_x, escala_y, rotacao }) = doc.entidade(id).map(|e| e.geometria.clone()) else { return Err("a folha sumiu".into()) };
    let novo = Ponto::novo(ponto.x + destino.x - info.min.x, ponto.y + destino.y - info.min.y);
    doc.aplicar_mudancas(vec![Mudanca::Trocar(id, Geometria::Insercao { bloco, ponto: novo, escala_x, escala_y, rotacao })]);
    Ok(())
}

/// As folhas inseridas, cada uma com `pdf_indice`: a posição no `cad_pdf` (`folhas: [i]`), que segue o número do carimbo.
fn com_indice_de_pdf(doc: &Documento) -> Value {
    let ordem = feathercad_io::pdf::folhas(doc);
    let mut lista = serde_json::to_value(doc.folhas_inseridas()).unwrap_or(Value::Null);
    for f in lista.as_array_mut().into_iter().flatten() {
        let (x, y) = (f["min"]["x"].as_f64().unwrap_or(f64::NAN), f["min"]["y"].as_f64().unwrap_or(f64::NAN));
        f["pdf_indice"] = ordem.iter().position(|(a, _)| (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6).map_or(Value::Null, |i| json!(i));
    }
    lista
}

/// `cad_folha`: `listar` (padrão), `inserir`, `escala`, `ajustar`, `campos`, `campos_definir`.
pub fn folha(doc: &mut Documento, args: &Value) -> Res<Value> {
    let acao = texto_opcional(args, "acao").unwrap_or("listar");
    let json_de = |f: &feathercad_core::folha::InfoFolha| serde_json::to_value(f).unwrap_or(Value::Null);
    match acao {
        "listar" => Ok(json!({
            "folhas": com_indice_de_pdf(doc),
            "papeis": PAPEIS.iter().map(|(n, w, h)| json!({ "nome": n, "largura_mm": w, "altura_mm": h })).collect::<Vec<_>>(),
            "escalas": ESCALAS,
        })),
        "inserir" => {
            let papel = texto_opcional(args, "papel").unwrap_or("A1");
            let ids_vec: Option<Vec<u64>> = args.get("ids").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_u64).collect()
            });
            let mut info = doc.folha_inserir_ids(papel, numero_opcional(args, "escala"), &campos_de(args.get("campos")), ids_vec.as_deref())?;
            let destino = match args.get("ponto") {
                Some(p) => Some(ponto(p, "ponto")?),
                None => args.get("origem").and_then(Value::as_bool).unwrap_or(false).then(|| Ponto::novo(0.0, 0.0)),
            };
            if let Some(d) = destino {
                levar_para(doc, info.id, d)?;
                info = doc.folhas_inseridas().into_iter().find(|f| f.id == info.id).ok_or("a folha não apareceu")?;
            }
            Ok(json!({ "folha": json_de(&info) }))
        }
        "escala" => Ok(json!({ "folha": json_de(&doc.folha_escala(indice_de(args)?, numero_de(args, "escala")?)?) })),
        "ajustar" => Ok(json!({ "folha": json_de(&doc.folha_ajustar(indice_de(args)?)?) })),
        "campos" => Ok(json!({ "campos": doc.folha_campos(indice_de(args)?)? })),
        "campos_definir" => Ok(json!({ "folha": json_de(&doc.folha_campos_definir(indice_de(args)?, &campos_de(args.get("campos")))?) })),
        outra => Err(format!("ação `{outra}` não existe (listar, inserir, escala, ajustar, campos, campos_definir)")),
    }
}

/// `cad_pdf`: grava o desenho em PDF vetorial, uma página por folha (blocos `FOLHA-n` e retângulos da camada `FOLHA`).
pub fn pdf(ag: &Agente, args: &Value) -> Res<Value> {
    let caminho = texto_de(args, "caminho")?;
    if std::path::Path::new(caminho).exists() && !args.get("sobrescrever").and_then(Value::as_bool).unwrap_or(false) {
        return Err(format!("`{caminho}` já existe: use `sobrescrever: true`"));
    }
    let papel_nome = texto_opcional(args, "papel").unwrap_or("A3");
    let papel = feathercad_io::pdf::Papel::de_nome(papel_nome).ok_or_else(|| format!("papel `{papel_nome}` não existe (A4, A3, A2, A1, A0, 2A0, 4A0, 8A0, 16A0, FOLHA ou auto)"))?;
    let orientacao = match texto_opcional(args, "orientacao") {
        Some(o) => feathercad_io::pdf::Orientacao::de_nome(o).ok_or_else(|| format!("orientação `{o}` não existe (automatica, horizontal ou vertical)"))?,
        None => feathercad_io::pdf::Orientacao::Automatica,
    };
    let indices: Option<Vec<usize>> = args.get("folhas").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).map(|n| n as usize).collect());
    let opcoes = feathercad_io::pdf::Opcoes { papel, orientacao, modo_papel: !args.get("cores").and_then(Value::as_bool).unwrap_or(false) };
    let bytes = feathercad_io::pdf::exportar_folhas_escolhidas(&ag.doc, indices.as_deref(), &opcoes)?;
    std::fs::write(caminho, &bytes).map_err(|e| format!("não consegui gravar `{caminho}`: {e}"))?;
    Ok(json!({ "caminho": caminho, "bytes": bytes.len(), "paginas": indices.as_ref().map_or_else(|| feathercad_io::pdf::descrever_folhas(&ag.doc).len(), Vec::len), "papel": papel_nome }))
}

/// `cad_contar`: quantos objetos são iguais a `id` (mesmo tipo e medida) ou têm a medida pedida.
pub fn contar(doc: &Documento, args: &Value) -> Res<Value> {
    let tolerancia = numero_opcional(args, "tolerancia").unwrap_or(0.01).clamp(0.0, 0.5);
    let c = if let Some(id) = args.get("igual_a").and_then(Value::as_u64) {
        iguais_a(doc, id).ok_or_else(|| format!("não existe a entidade {id}"))?
    } else {
        let tipo = texto_de(args, "tipo")?;
        let m = args.get("medida");
        let a = m.and_then(|o| o.get("largura").or_else(|| o.get("raio")).or_else(|| o.get("comprimento"))).and_then(Value::as_f64).ok_or("falta `medida` ({largura, altura} | {raio} | {comprimento}) ou `igual_a`")?;
        let b = m.and_then(|o| o.get("altura")).and_then(Value::as_f64).unwrap_or(a);
        por_medida(doc, tipo, a, b, tolerancia, texto_opcional(args, "camada"))?
    };
    let limite = numero_opcional(args, "ids").map_or(0, |n| n as usize);
    Ok(json!({
        "total": c.ids.len(),
        "por_camada": c.por_camada.iter().map(|(n, q)| json!({ "camada": n, "quantidade": q })).collect::<Vec<_>>(),
        "ids": c.ids.iter().take(limite).collect::<Vec<_>>(),
        "folga": tolerancia,
    }))
}
