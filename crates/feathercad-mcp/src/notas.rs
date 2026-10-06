//! `cad_notas_padrao`: as notas normativas que os projetos elétricos profissionais trazem (conjuntos por assunto), prontas para
//! o `cad_texto_bloco`. Com `desenhar` já escreve na folha. O texto vem do que os projetos de referência dizem
//! (`docs/REFERENCIA-ELETRICA-3-PROJETOS.md`); a norma citada deve ser conferida pelo projetista na edição vigente.

use feathercad_core::documento::Documento;
use serde_json::{json, Value};

use crate::json::{texto_opcional, Res};

const CONJUNTOS: [&str; 5] = ["geral", "aterramento", "quadros", "iluminacao", "emergencia"];

fn texto_do_conjunto(nome: &str, eletroduto: &str, fiacao: &str) -> Option<Vec<String>> {
    let v: Vec<String> = match nome {
        "geral" => vec![
            "PROJETO ELABORADO CONFORME A ABNT NBR 5410 (INSTALAÇÕES ELÉTRICAS DE BAIXA TENSÃO) E A NR-10; SIMBOLOGIA CONFORME A NBR 5444.".into(),
            format!("ELETRODUTOS NÃO COTADOS SERÃO {eletroduto}."),
            format!("FIAÇÃO NÃO COTADA SERÁ DE {fiacao}."),
            "TODOS OS CONDUTORES DOS CIRCUITOS SERÃO NÃO-PROPAGANTES DE CHAMA; CABOS DE DISTRIBUIÇÃO COM ISOLAÇÃO 0,6/1 kV E OS DEMAIS 450/750 V.".into(),
            "CORES DOS CONDUTORES: NEUTRO AZUL-CLARO; FASES PRETO, BRANCO E VERMELHO (R, S, T); PROTEÇÃO (TERRA) VERDE OU VERDE-AMARELO; RETORNO AMARELO.".into(),
            "TODAS AS PARTES METÁLICAS NORMALMENTE NÃO ENERGIZADAS DEVEM SER ATERRADAS.".into(),
            "QUALQUER DIVERGÊNCIA ENTRE ESTE PROJETO E AS NORMAS TÉCNICAS VIGENTES DEVE SER COMUNICADA AO AUTOR ANTES DA EXECUÇÃO DOS SERVIÇOS.".into(),
            "ESTE PROJETO NÃO PODERÁ SOFRER MODIFICAÇÕES SEM A PRÉVIA AUTORIZAÇÃO DO PROJETISTA.".into(),
            "OS SERVIÇOS DEVEM SER EXECUTADOS POR EMPRESA ESPECIALIZADA, COM PROFISSIONAIS QUALIFICADOS E AUTORIZADOS CONFORME A NR-10.".into(),
        ],
        "aterramento" => vec![
            "AS CONEXÕES DOS ELETRODOS DE ATERRAMENTO COM A HASTE DE TERRA DEVEM SER FEITAS POR SOLDA EXOTÉRMICA.".into(),
            "O CONDUTOR DE ATERRAMENTO É DE COBRE NU, INSTALADO A PELO MENOS 0,50 m DE PROFUNDIDADE; EMENDAS CABO-HASTE E CABO-CABO POR SOLDA EXOTÉRMICA.".into(),
            "HASTE DE ATERRAMENTO 5/8\" x 2,40 m; CAIXA DE INSPEÇÃO DE ATERRAMENTO NA SAÍDA DA HASTE.".into(),
        ],
        "quadros" => vec![
            "OS QUADROS DEVEM TER OS CIRCUITOS E OS DISJUNTORES IDENTIFICADOS COM ETIQUETAS AUTOADESIVAS RESISTENTES AO TEMPO.".into(),
            "OS QUADROS MONTADOS DEVEM POSSUIR BARREIRA DE PROTEÇÃO E SINALIZAÇÃO CONFORME A NBR 5410; ESTAR EM CONFORMIDADE COM A NBR IEC 60439.".into(),
            "NA MONTAGEM DOS QUADROS USAR TERMINAIS DO TIPO PINO, OLHAL OU GARFO EM TODOS OS CABOS LIGADOS A DISJUNTORES OU BARRAMENTOS.".into(),
            "INSTALAR DISPOSITIVO DR (SENSIBILIDADE DE 30 mA) NOS CIRCUITOS DE TOMADAS E DE ÁREAS MOLHADAS E DPS (CLASSE II) NO QUADRO.".into(),
        ],
        "iluminacao" => vec![
            "NÍVEIS DE ILUMINÂNCIA CONFORME A ABNT NBR ISO/CIE 8995-1 PARA A ATIVIDADE; CÁLCULO PELO MÉTODO DOS LUMENS COM CONFERÊNCIA PONTO A PONTO.".into(),
            "O COEFICIENTE DE UTILIZAÇÃO E O FLUXO SÃO VALORES TÍPICOS: CONFIRMAR COM A FOTOMETRIA (.IES/.LDT) DO FABRICANTE E SIMULAR A UNIFORMIDADE ANTES DA COMPRA.".into(),
            "AS LUMINÁRIAS DEVEM SER ATERRADAS; DEIXAR O CONDUTOR DE PROTEÇÃO COM FOLGA QUANDO A LUMINÁRIA NÃO FOR INSTALADA NA OBRA.".into(),
        ],
        "emergencia" => vec![
            "A ILUMINAÇÃO DE EMERGÊNCIA E A SINALIZAÇÃO DE SAÍDA OBEDECEM À ABNT NBR 10898; AUTONOMIA MÍNIMA DE 1 h E NÍVEL MÍNIMO NAS ROTAS DE FUGA CONFORME A NORMA.".into(),
            "OS BLOCOS AUTÔNOMOS E A SINALIZAÇÃO DE SAÍDA FICAM EM CIRCUITO EXCLUSIVO, ENERGIZADO EM PERMANENTE.".into(),
        ],
        _ => return None,
    };
    Some(v)
}

/// `cad_notas_padrao`: `conjuntos` (geral, aterramento, quadros, iluminacao, emergencia), `eletroduto`, `fiacao`; `desenhar` escreve na folha.
pub fn notas(doc: &mut Documento, args: &Value) -> Res<Value> {
    let nomes: Vec<String> = args.get("conjuntos").and_then(Value::as_array).map_or_else(|| vec!["geral".to_string()], |a| a.iter().filter_map(Value::as_str).map(str::to_lowercase).collect());
    let eletroduto = texto_opcional(args, "eletroduto").unwrap_or("DE PVC FLEXÍVEL Ø3/4\"");
    let fiacao = texto_opcional(args, "fiacao").unwrap_or("#2,5 mm²");
    let mut itens: Vec<Value> = Vec::new();
    let mut n = 0;
    for nome in &nomes {
        let textos = texto_do_conjunto(nome, eletroduto, fiacao).ok_or_else(|| format!("conjunto `{nome}` não existe ({})", CONJUNTOS.join(", ")))?;
        for t in textos {
            n += 1;
            itens.push(json!({ "tipo": "nota", "texto": format!("{n}. {t}") }));
        }
    }
    let mut r = json!({ "itens": itens, "total": n, "conjuntos": nomes });
    if let Some(d) = args.get("desenhar").filter(|d| d.is_object()) {
        let mut a = d.clone();
        a["itens"] = r["itens"].clone();
        if let Some(t) = texto_opcional(d, "titulo") {
            a["itens"].as_array_mut().into_iter().for_each(|l| l.insert(0, json!({ "tipo": "titulo", "texto": t })));
        }
        r["desenhado"] = crate::texto_bloco::texto_bloco(doc, &a)?;
    }
    Ok(r)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn numera_em_sequencia_entre_conjuntos_e_recusa_nome_errado() {
        let mut d = Documento::default();
        let r = notas(&mut d, &json!({ "conjuntos": ["geral", "aterramento"] })).unwrap();
        assert_eq!(r["total"].as_u64().unwrap() as usize, r["itens"].as_array().unwrap().len());
        assert!(r["itens"][9]["texto"].as_str().unwrap().starts_with("10. "));
        assert!(notas(&mut d, &json!({ "conjuntos": ["nada"] })).is_err());
    }

    #[test]
    fn eletroduto_e_fiacao_entram_no_texto() {
        let mut d = Documento::default();
        let r = notas(&mut d, &json!({ "eletroduto": "DE AÇO GALVANIZADO Ø1\"", "fiacao": "#4 mm²" })).unwrap();
        let t = r["itens"].to_string();
        assert!(t.contains("AÇO GALVANIZADO") && t.contains("#4 mm²"));
    }

    #[test]
    fn desenha_no_texto_bloco() {
        let mut d = Documento::default();
        let r = notas(&mut d, &json!({ "conjuntos": ["quadros"], "desenhar": { "origem": [0, 100], "largura": 120, "titulo": "NOTAS" } })).unwrap();
        assert!(r["desenhado"]["criadas"].as_u64().unwrap() >= 5, "{r}");
    }
}
