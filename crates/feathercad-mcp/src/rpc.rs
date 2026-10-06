//! O protocolo MCP (JSON-RPC 2.0, uma mensagem por linha, por stdin/stdout). Só o necessário para
//! ferramentas e guias: `initialize`, `tools/list`, `tools/call`, `prompts/list`, `prompts/get` e `ping`. Nada de rede.

use serde_json::{json, Value};

use crate::agente::Agente;
use crate::esquemas;
use crate::ferramentas::chamar;
use crate::fonte::Fonte;
use crate::guias;

const VERSOES: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCOES: &str = "FeatherCAD: CAD 2D local. COMO TRABALHAR: pedido SIMPLES (desenhar uma forma, mover, editar, medir) vai direto às ferramentas de desenho; pedido de PROJETO (várias folhas, quadros, legendas) começa por cad_projeto: ele pergunta a disciplina e o tipo (você pergunta ao usuário) e entrega a receita com os itens obrigatórios; confira com cad_projeto status antes de entregar. Convenção: X para a direita, Y para cima, ângulos em graus anti-horário a partir do eixo X, medidas na unidade do desenho (cad_resumo diz qual). Para ter noção de espaço: comece por cad_resumo, use cad_ver (imagem com coordenadas) e cad_entidades/cad_regioes (números exatos); depois de desenhar, confira com cad_ver e cad_verificar. Para ler/escrever desenhos gastando poucos tokens use o texto compacto (cad_exportar, cad_importar, .fct) em vez de DXF; desenho grande: escreva um .fct e abra com cad_abrir. No .fct a cor de uma camada vale a da primeira declaração; para outra cor na mesma camada termine a linha com #RRGGBB. Texto aceita acentos e Ø ² ° (também no DXF). CORES: o desenho aparece sobre o fundo do tema do aplicativo, claro ou escuro (cad_resumo diz `tema`). #FFFFFF e #000000 são a cor automática e trocam sozinhos com o tema; para as demais, escolha cores que contrastem com o fundo: no tema claro NADA quase branco (amarelo-limão, cinza claro, ciano pálido); no escuro NADA quase preto (azul-marinho, marrom escuro, cinza escuro). Cores médias e saturadas (laranja, ciano, magenta, verde, vermelho) servem nos dois temas; mude o tom pela camada, não por brancos e pretos puros. cad_criar avisa quando a cor não contrasta. Projeto vai numa FOLHA (moldura, carimbo, escala). Se salvar por cima do arquivo que o aplicativo tem aberto (sobrescrever), ele recarrega sozinho. PROJETO ELÉTRICO: só quando o projeto for elétrico, leia o prompt `projeto_eletrico` e use cad_simbolos/cad_simbolo/cad_eletroduto (simbologia NBR 5444 pronta) e, para EQUIPAMENTOS (disjuntor, DR, DPS, quadro, transformador…), cad_componentes/cad_componente (arquivos prontos: NUNCA redesenhe à mão); em qualquer outro tipo de projeto ignore esses recursos.";

fn resposta(id: &Value, resultado: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": resultado })
}

fn erro(id: &Value, codigo: i32, mensagem: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codigo, "message": mensagem } })
}

fn inicializar(params: &Value) -> Value {
    let pedida = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(VERSOES[0]);
    let versao = if VERSOES.contains(&pedida) { pedida } else { VERSOES[0] };
    json!({ "protocolVersion": versao, "capabilities": { "tools": {}, "prompts": {} }, "serverInfo": { "name": "feathercad", "version": env!("CARGO_PKG_VERSION") }, "instructions": INSTRUCOES })
}

/// O resultado de `tools/call`: erro de ferramenta volta como resultado com `isError` (não como erro de protocolo).
/// Um **pânico** na ferramenta não derruba o servidor nem o aplicativo (que hospeda este código ao vivo): vira erro, e o que a
/// ferramenta já tinha mudado no desenho é desfeito.
fn chamar_ferramenta(ag: &mut Agente, fonte: &Fonte, params: &Value) -> Value {
    let nome = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let marca = ag.doc.marca_do_historico();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| chamar(ag, fonte, nome, &args))) {
        Ok(Ok(conteudo)) => json!({ "content": conteudo, "isError": false }),
        Ok(Err(e)) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
        Err(causa) => {
            let motivo = causa.downcast_ref::<&str>().map(|s| (*s).to_string()).or_else(|| causa.downcast_ref::<String>().cloned()).unwrap_or_else(|| "motivo desconhecido".into());
            let desfeito = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ag.doc.desfazer_ate(marca))).is_ok();
            let texto = format!("erro interno na ferramenta `{nome}`: {motivo}. {}", if desfeito { "O que ela já tinha mudado no desenho foi desfeito." } else { "Confira o desenho: não consegui desfazer o que ela mudou." });
            json!({ "content": [{ "type": "text", "text": texto }], "isError": true })
        }
    }
}

/// Atende uma mensagem. `None` para notificações (que não têm resposta).
pub fn atender(ag: &mut Agente, fonte: &Fonte, msg: &Value) -> Option<Value> {
    let metodo = msg.get("method").and_then(Value::as_str)?;
    let id = msg.get("id")?; // sem id = notificação
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(match metodo {
        "initialize" => resposta(id, inicializar(&params)),
        "ping" => resposta(id, json!({})),
        "tools/list" => resposta(id, json!({ "tools": esquemas::todas() })),
        "prompts/list" => resposta(id, json!({ "prompts": guias::listar() })),
        "prompts/get" => match guias::obter(params.get("name").and_then(Value::as_str).unwrap_or("")) {
            Some(g) => resposta(id, g),
            None => erro(id, -32602, "prompt desconhecido"),
        },
        "tools/call" => resposta(id, chamar_ferramenta(ag, fonte, &params)),
        outro => erro(id, -32601, &format!("método desconhecido: {outro}")),
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn panico_na_ferramenta_vira_erro_e_desfaz_o_que_ela_fez() {
        let mut ag = Agente::default();
        let fonte = crate::fonte::carregar();
        ag.doc = feathercad_core::documento::Documento::default().com_unidade(Some(feathercad_core::documento::Unidade::Metro));
        let r = chamar_ferramenta(&mut ag, &fonte, &json!({ "name": "cad_teste_panico", "arguments": {} }));
        assert_eq!(r["isError"], true);
        assert!(r["content"][0]["text"].as_str().unwrap().contains("erro interno"), "{r}");
        assert_eq!(ag.doc.entidades().len(), 0, "a linha que a ferramenta desenhou antes do pânico foi desfeita");
        let ok = chamar_ferramenta(&mut ag, &fonte, &json!({ "name": "cad_resumo", "arguments": {} }));
        assert_eq!(ok["isError"], false, "o servidor segue funcionando");
    }
}
