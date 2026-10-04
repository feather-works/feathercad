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
    let isometria = args.get("isometria").and_then(Value::as_bool).unwrap_or(false);
    // No modo projeto os padrões são os baratos: 800 px, sem grade nem rótulos (qualquer argumento explícito vale).
    let leve = ag.projeto;
    let o = Opcoes {
        janela: args.get("janela").map(janela).transpose()?,
        largura_px: numero_opcional(args, "largura").map_or(if leve { 800 } else { 1000 }, |n| n as u32),
        rotulos: args.get("rotulos").and_then(Value::as_bool).unwrap_or(!leve),
        grade: args.get("grade").and_then(Value::as_bool).unwrap_or(!leve),
        destacar: ids_de(args, "destacar"),
        rotacao_x: numero_opcional(args, "rotacao_x").unwrap_or(if isometria { 55.0 } else { 0.0 }),
        rotacao_z: numero_opcional(args, "rotacao_z").unwrap_or(if isometria { 45.0 } else { 0.0 }),
        isometria,
    };
    let img = desenhar(&ag.doc, &o, fonte)?;
    let legenda = json!({
        "janela": { "min": par(img.janela.0), "max": par(img.janela.1) }, "pixels": [img.largura, img.altura],
        "unidades_por_pixel": arred(img.unidades_por_pixel), "unidade": ag.doc.unidade().map(feathercad_core::documento::Unidade::nome),
        "rotacao_x": o.rotacao_x, "rotacao_z": o.rotacao_z, "isometria": isometria,
        "como_ler": if o.rotacao_x.abs() > 1e-4 || o.rotacao_z.abs() > 1e-4 {
            "vista 3D em órbita/isometria; indicador de eixos no canto inferior esquerdo (X vermelho, Y verde, Z azul)"
        } else {
            "números nas bordas = coordenadas do desenho; linhas mais escuras = eixos x=0 e y=0; #N = id da entidade; barra preta no canto = comprimento de escala"
        },
    });
    if let Some(caminho) = texto_opcional(args, "arquivo") {
        if !caminho.to_lowercase().ends_with(".png") {
            return Err("`arquivo` precisa terminar em `.png`".into());
        }
        std::fs::write(caminho, &img.png).map_err(|e| format!("não consegui gravar `{caminho}`: {e}"))?;
        let mut l = legenda;
        l["arquivo"] = json!(caminho);
        l["bytes"] = json!(img.png.len());
        l["nota"] = json!("a imagem foi gravada no disco e não entrou na conversa; abra o PNG só se precisar olhar");
        return Ok(texto(&l));
    }
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
    let do_disco;
    let fonte = match texto_opcional(args, "caminho") {
        Some(c) => {
            if !c.to_lowercase().ends_with(".fct") {
                return Err("`caminho` só aceita `.fct` (para DXF/.fcdz use cad_abrir ou cad_converter)".into());
            }
            do_disco = std::fs::read_to_string(c).map_err(|e| format!("não consegui ler `{c}`: {e}"))?;
            do_disco.as_str()
        }
        None => texto_de(args, "texto")?,
    };
    let lido = importar(fonte)?;
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
        "cad_folha" => crate::folhas::folha(&mut ag.doc, args),
        "cad_simbolo" => simbolo(&mut ag.doc, args),
        "cad_componente" => crate::componentes::componente(&mut ag.doc, args),
        "cad_eletroduto" => eletroduto(&mut ag.doc, args),
        "cad_nuvem_secoes" => crate::nuvem_ferramentas::secoes(ag, args),
        "cad_nuvem_feicoes" => crate::nuvem_ferramentas::feicoes(ag, args),
        "cad_nuvem_qualidade" => crate::nuvem_ferramentas::qualidade_ferramenta(ag, args),
        "cad_alinhamento_ajustar" => crate::viario_projeto::alinhamento_ajustar(ag, args),
        "cad_greide" => crate::viario_projeto::greide(ag, args),
        "cad_tabela_estacas" => crate::viario_projeto::tabela_estacas(ag, args),
        "cad_via" => crate::via::via(ag, args),
        "cad_alinhamento" => crate::viario::alinhamento(ag, args),
        "cad_superelevacao" => crate::viario::superelevacao(ag, args),
        "cad_ao_longo" => crate::ao_longo::ao_longo(&mut ag.doc, args),
        "cad_malha_coordenadas" => crate::malha_norte::malha(&mut ag.doc, args),
        "cad_norte" => crate::malha_norte::norte(&mut ag.doc, args),
        "cad_projetar" => crate::projecao::projetar_ferramenta(&mut ag.doc, args),
        "cad_detalhe" => crate::detalhe::detalhe(&mut ag.doc, args),
        "cad_tabela" => crate::tabela::tabela(&mut ag.doc, args),
        "cad_grafico" => crate::grafico::grafico(&mut ag.doc, args),
        "cad_texto_bloco" => crate::texto_bloco::texto_bloco(&mut ag.doc, args),
        "cad_topografia" => crate::topografia::topografia(ag, args),
        "cad_desfazer" => Ok(desfazer(&mut ag.doc, args.get("refazer").and_then(Value::as_bool).unwrap_or(false))),
        _ => Err(format!("ferramenta desconhecida `{nome}`")),
    }
}

/// Executa a ferramenta `nome`.
pub fn chamar(ag: &mut Agente, fonte: &Fonte, nome: &str, args: &Value) -> Res<Conteudo> {
    match nome {
        #[cfg(test)]
        "cad_teste_panico" => {
            ag.doc.adicionar_linha(feathercad_core::geom::Ponto::novo(0.0, 0.0), feathercad_core::geom::Ponto::novo(1.0, 1.0));
            panic!("pânico de teste");
        }
        "cad_novo" => ag.novo(texto_de(args, "unidade")?).map(|v| texto(&v)),
        "cad_abrir" => ag.abrir(texto_de(args, "caminho")?).map(|v| texto(&v)),
        "cad_salvar" => ag.salvar(texto_opcional(args, "caminho"), args.get("sobrescrever").and_then(Value::as_bool).unwrap_or(false), args.get("verificar_checklist").and_then(Value::as_bool).unwrap_or(false), texto_opcional(args, "dominio")).map(|v| texto(&v)),
        "cad_converter" => converter(texto_de(args, "origem")?, texto_de(args, "destino")?, args.get("sobrescrever").and_then(Value::as_bool).unwrap_or(false)).map(|v| texto(&v)),
        "cad_resumo" => Ok(texto(&resumo(ag))),
        "cad_exemplos" => crate::exemplos::exemplos(args).map(|v| texto(&v)),
        "cad_simbolos" => Ok(texto(&listar())),
        "cad_componentes" => Ok(texto(&crate::componentes::listar(args))),
        "cad_ver" => ver(ag, fonte, args),
        "cad_modo" => {
            if let Some(p) = args.get("projeto").and_then(Value::as_bool) {
                ag.projeto = p;
            }
            Ok(texto(&json!({ "projeto": ag.projeto, "efeito": "no modo projeto cad_ver usa 800 px, sem grade nem rótulos (passe largura/grade/rotulos para mudar); `arquivo: caminho.png` grava a imagem no disco sem pô-la na conversa" })))
        }
        "cad_entidades" => entidades(&ag.doc, args).map(|v| texto(&v)),
        "cad_exportar" => exportar_texto(ag, args),
        "cad_importar" => importar_texto(ag, args),
        "cad_modelos" => Ok(texto(&crate::modelos::listar())),
        "cad_modelo" => crate::modelos::modelo(ag, fonte, args).map(|v| texto(&v)),
        "cad_alinhamento_ponto" => crate::viario::alinhamento_ponto(ag, args).map(|v| texto(&v)),
        "cad_medir" => medir(&ag.doc, args).map(|v| texto(&v)),
        "cad_regioes" => regioes(&ag.doc, args).map(|v| texto(&v)),
        "cad_verificar" => Ok(texto(&verificar(&ag.doc, args))),
        "cad_lote" => lote(ag, fonte, args).map(|v| texto(&v)),
        "cad_pdf" => crate::folhas::pdf(ag, args).map(|v| texto(&v)),
        "cad_contar" => crate::folhas::contar(&ag.doc, args).map(|v| texto(&v)),
        outra => editar_ou_criar(ag, outra, args).map(|v| texto(&v)),
    }
}
