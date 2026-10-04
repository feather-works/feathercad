//! Ferramenta MCP para geração de levantamento topográfico a partir de nuvem de pontos.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::agente::Agente;
use crate::json::{numero_opcional, Res};

pub fn topografia(ag: &mut Agente, args: &Value) -> Res<Value> {
    let pontos = if let Some(caminho) = args.get("caminho").and_then(Value::as_str) {
        let conteudo = std::fs::read_to_string(caminho).map_err(|e| format!("não consegui ler `{caminho}`: {e}"))?;
        feathercad_core::topografia::ler_nuvem_pontos(&conteudo)?
    } else if let Some(texto) = args.get("texto").and_then(Value::as_str) {
        feathercad_core::topografia::ler_nuvem_pontos(texto)?
    } else {
        return Err("informe `caminho` para o arquivo ou `texto` com os pontos".into());
    };

    let opcoes = feathercad_core::topografia::OpcoesTopografia {
        equidistancia: numero_opcional(args, "equidistancia"),
        passo_mestra: args.get("passo_mestra").and_then(Value::as_u64).map_or(5, |v| v as usize),
        max_comprimento_aresta: numero_opcional(args, "max_aresta"),
        incluir_tin: args.get("incluir_tin").and_then(Value::as_bool).unwrap_or(true),
        incluir_faces_3d: args.get("incluir_faces_3d").and_then(Value::as_bool).unwrap_or(true),
        incluir_pontos: args.get("incluir_pontos").and_then(Value::as_bool).unwrap_or(true),
        rotular_pontos: args.get("rotular_pontos").and_then(Value::as_bool).unwrap_or(true),
        incluir_quadro: args.get("incluir_quadro").and_then(Value::as_bool).unwrap_or(true),
        ponto_quadro: args.get("quadro_ponto").map(|p| crate::json::ponto(p, "quadro_ponto")).transpose()?,
        exagero_vertical: numero_opcional(args, "exagero_vertical"),
    };

    // `so_estatisticas`: faz a conta num rascunho e devolve os números sem tocar no desenho aberto.
    let so_estatisticas = args.get("so_estatisticas").and_then(Value::as_bool).unwrap_or(false);
    let resumo = if so_estatisticas {
        let mut rascunho = feathercad_core::documento::Documento::default();
        feathercad_core::topografia::criar_levantamento_topografico(&mut rascunho, &pontos, &opcoes)?
    } else {
        feathercad_core::topografia::criar_levantamento_topografico(&mut ag.doc, &pontos, &opcoes)?
    };
    if so_estatisticas {
        return Ok(json!({ "so_estatisticas": true, "total_pontos": resumo.total_pontos, "cota_min": resumo.cota_min, "cota_max": resumo.cota_max, "desnivel": resumo.desnivel, "cota_media": resumo.cota_media, "equidistancia": resumo.equidistancia, "area_m2": resumo.area_m2, "area_ha": resumo.area_m2 / 10_000.0, "extensao_x": resumo.max_x - resumo.min_x, "extensao_y": resumo.max_y - resumo.min_y, "total_curvas": resumo.total_curvas, "desenho_alterado": false }));
    }
    if let Some(caminho) = args.get("caminho").and_then(Value::as_str) {
        ag.arquivo = Some(PathBuf::from(caminho));
    }

    Ok(json!({
        "total_pontos": resumo.total_pontos,
        "total_triangulos": resumo.total_triangulos,
        "total_curvas": resumo.total_curvas,
        "cota_min": resumo.cota_min,
        "cota_max": resumo.cota_max,
        "desnivel": resumo.desnivel,
        "cota_media": resumo.cota_media,
        "equidistancia": resumo.equidistancia,
        "passo_mestra": resumo.passo_mestra,
        "exagero_vertical": opcoes.exagero_vertical.unwrap_or(1.0),
        "area_m2": resumo.area_m2,
        "area_ha": resumo.area_m2 / 10_000.0,
        "extensao_x": resumo.max_x - resumo.min_x,
        "extensao_y": resumo.max_y - resumo.min_y,
        "entidades_criadas": resumo.ids_criados.len(),
    }))
}
