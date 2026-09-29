//! Cada ferramenta: lê os argumentos, chama o motor e monta a resposta (texto JSON e, se for o caso, imagem).

use std::collections::BTreeSet;

use base64::Engine;
use feathercad_core::documento::Id;
use serde_json::{json, Value};

use crate::agente::{converter, Agente};
use crate::compacto::{exportar, importar};
use crate::consulta::{entidades, medir, passa_filtros, regioes, resumo, verificar};
use crate::fonte::Fonte;
use crate::imagem::{desenhar, Opcoes};
use crate::lote::lote;
use crate::json::{arred, janela, numero_opcional, par, texto_de, texto_opcional, Res};
use crate::eletroduto::eletroduto;
use crate::simbolos::{listar, simbolo};
use crate::mutacoes::{aplicar_compacto, camadas, criar, desfazer, editar, hachurar};

/// Os blocos de conteúdo da resposta (`text` e, às vezes, `image`).
pub type Conteudo = Vec<Value>;

fn texto(v: &Value) -> Conteudo {
    vec![json!({ "type": "text", "text": serde_json::to_string(v).unwrap_or_default() })]
}

fn ids_de(args: &Value, nome: &str) -> BTreeSet<Id> {
    args.get(nome).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default()
}

/// A imagem do desenho e uma legenda de como lê-la.
fn ver(ag: &Agente, fonte: &Fonte, args: &Value) -> Res<Conteudo> {
    let o = Opcoes {
        janela: args.get("janela").map(janela).transpose()?,
        largura_px: numero_opcional(args, "largura").map_or(1000, |n| n as u32),
        rotulos: args.get("rotulos").and_then(Value::as_bool).unwrap_or(true),
        grade: args.get("grade").and_then(Value::as_bool).unwrap_or(true),
        destacar: ids_de(args, "destacar"),
    };
    let img = desenhar(&ag.doc, &o, fonte)?;
    let legenda = json!({
        "janela": { "min": par(img.janela.0), "max": par(img.janela.1) }, "pixels": [img.largura, img.altura],
        "unidades_por_pixel": arred(img.unidades_por_pixel), "unidade": ag.doc.unidade().map(feathercad_core::documento::Unidade::nome),
        "como_ler": "números nas bordas = coordenadas do desenho; linhas mais escuras = eixos x=0 e y=0; #N = id da entidade; barra preta no canto = comprimento de escala",
    });
    let b64 = base64::engine::general_purpose::STANDARD.encode(&img.png);
    Ok(vec![json!({ "type": "image", "data": b64, "mimeType": "image/png" }), json!({ "type": "text", "text": legenda.to_string() })])
}

/// O texto compacto do desenho (ou do que passa nos filtros).
fn exportar_texto(ag: &Agente, args: &Value) -> Res<Conteudo> {
    let filtrado = ["ids", "tipo", "camada", "janela"].iter().any(|c| args.get(*c).is_some());
    let escolhidas: Option<BTreeSet<Id>> = if filtrado { Some(passa_filtros(&ag.doc, args)?.into_iter().collect()) } else { None };
    let com_ids = args.get("com_ids").and_then(Value::as_bool).unwrap_or(false);
    let (t, est) = exportar(&ag.doc, escolhidas.as_ref(), com_ids);
    let cab = format!("// {} objetos{}{}\n", est.objetos, if est.blocos_abertos > 0 { format!(", {} blocos abertos em formas soltas", est.blocos_abertos) } else { String::new() }, if est.ignorados > 0 { format!(", {} ignorados", est.ignorados) } else { String::new() });
    Ok(vec![json!({ "type": "text", "text": format!("{cab}{t}") })])
}

fn importar_texto(ag: &mut Agente, args: &Value) -> Res<Conteudo> {
    let lido = importar(texto_de(args, "texto")?)?;
    if args.get("substituir").and_then(Value::as_bool).unwrap_or(false) {
        let un = lido.unidade.as_deref().and_then(feathercad_core::documento::Unidade::de_nome).or(ag.doc.unidade());
        ag.doc = feathercad_core::documento::Documento::default().com_unidade(un);
        ag.arquivo = None;
    } else if ag.doc.unidade().is_none() {
        ag.doc.definir_unidade(lido.unidade.as_deref().and_then(feathercad_core::documento::Unidade::de_nome));
    }
    let ids = aplicar_compacto(&mut ag.doc, &lido)?;
    Ok(texto(&json!({ "criadas": ids.len(), "ids": ids })))
}

fn editar_ou_criar(ag: &mut Agente, nome: &str, args: &Value) -> Res<Value> {
    match nome {
        "cad_criar" => criar(&mut ag.doc, args),
        "cad_editar" => editar(&mut ag.doc, args),
        "cad_camadas" => camadas(&mut ag.doc, args),
        "cad_hachurar" => hachurar(&mut ag.doc, args),
        "cad_simbolo" => simbolo(&mut ag.doc, args),
        "cad_eletroduto" => eletroduto(&mut ag.doc, args),
        "cad_desfazer" => Ok(desfazer(&mut ag.doc, args.get("refazer").and_then(Value::as_bool).unwrap_or(false))),
        _ => Err(format!("ferramenta desconhecida `{nome}`")),
    }
}

/// Executa a ferramenta `nome`.
pub fn chamar(ag: &mut Agente, fonte: &Fonte, nome: &str, args: &Value) -> Res<Conteudo> {
    match nome {
        "cad_novo" => ag.novo(texto_de(args, "unidade")?).map(|v| texto(&v)),
        "cad_abrir" => ag.abrir(texto_de(args, "caminho")?).map(|v| texto(&v)),
        "cad_salvar" => ag.salvar(texto_opcional(args, "caminho"), args.get("sobrescrever").and_then(Value::as_bool).unwrap_or(false)).map(|v| texto(&v)),
        "cad_converter" => converter(texto_de(args, "origem")?, texto_de(args, "destino")?, args.get("sobrescrever").and_then(Value::as_bool).unwrap_or(false)).map(|v| texto(&v)),
        "cad_resumo" => Ok(texto(&resumo(ag))),
        "cad_simbolos" => Ok(texto(&listar())),
        "cad_ver" => ver(ag, fonte, args),
        "cad_entidades" => entidades(&ag.doc, args).map(|v| texto(&v)),
        "cad_exportar" => exportar_texto(ag, args),
        "cad_importar" => importar_texto(ag, args),
        "cad_medir" => medir(&ag.doc, args).map(|v| texto(&v)),
        "cad_regioes" => regioes(&ag.doc, args).map(|v| texto(&v)),
        "cad_verificar" => Ok(texto(&verificar(&ag.doc, args))),
        "cad_lote" => lote(ag, fonte, args).map(|v| texto(&v)),
        outra => editar_ou_criar(ag, outra, args).map(|v| texto(&v)),
    }
}
