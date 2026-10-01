//! Modelos de exemplo de projetos (arquitetura, elétrica, hidráulica, estrutural, ferrovia, mecânica)
//! para dar contexto de boas práticas e moldes prontos para a IA.

use serde_json::{json, Value};
use crate::json::{texto_opcional, Res};

/// Lista os nomes dos modelos disponíveis ou devolve o modelo pedido por `nome`.
fn lista_modelos() -> Value {
    json!({
        "modelos_disponiveis": [
            { "nome": "arquitetura", "descricao": "Residência contemporânea de 2 pavimentos: sapatas, baldrame, térreo, superior, escada 3D, cotas, carimbos e pranchetas A1 (1:50)." },
            { "nome": "eletrica", "descricao": "Projeto elétrico NBR 5444 / NBR 5410: QDC, pontos de luz, tomadas TUG/TUE, interruptores, circuitos e fiação em eletrodutos." },
            { "nome": "hidraulica", "descricao": "Instalações de Água Fria e Esgoto: reservatório, coluna AF-1, ramais, caixas de gordura (CG), sifonadas (CS), inspeção (CI) e ventilação (TEV)." },
            { "nome": "estrutural", "descricao": "Projeto de Formas e Fundações: eixos de locação, sapatas isoladas, estacas, colunas/pilares e vigas baldrame." },
            { "nome": "ferrovia", "descricao": "Projeto de Infraestrutura Ferroviária: via permanente (bitola 1,60m / 1,00m), dormentes a cada 0,60m, trilhos TR-68, AMVs e cotas de eixo." },
            { "nome": "mecanica", "descricao": "Desenho Mecânico Industrial 2D/3D: eixos, flanges, furações em círculo de furos (PCD), cotas funcionais e hachuras de corte." }
        ],
        "como_usar": "Chame `cad_exemplos` com `nome: \"arquitetura\"` (ou outro) para receber a especificação técnica completa, camadas ACI e o modelo de código em `cad_lote`."
    })
}

fn modelo_arquitetura() -> Value {
    json!({
        "nome": "arquitetura",
        "titulo": "Residência Contemporânea de 2 Pavimentos em Pranchetas A1",
        "unidade": "m",
        "escala": 50,
        "camadas": [
            { "nome": "FUND_ESTACAS_SAPATAS", "cor": "#808080", "aci": 8, "descricao": "Sapatas isoladas (Z = -1.20 a -0.80) e estacas (Z = -2.50 a -1.20)" },
            { "nome": "FUND_BALDRAME", "cor": "#C0C0C0", "aci": 9, "descricao": "Vigas baldrame (0.20x0.40m) interligando fundações em Z = -0.40 a 0.00" },
            { "nome": "ARQ_PAREDES_TERREO", "cor": "#FFFFFF", "aci": 7, "descricao": "Alvenarias do térreo (15cm) com vãos de portas e janelas (Z = 0.00 a 2.80)" },
            { "nome": "ARQ_PAREDES_SUPERIOR", "cor": "#FFFFFF", "aci": 7, "descricao": "Alvenarias do 2º pavimento com sacada em balanço (Z = 2.95 a 5.65)" },
            { "nome": "ARQ_ESCADAS", "cor": "#00FFFF", "aci": 4, "descricao": "Escada em 2 lances com patamar (espelho 17.5cm, piso 28cm, corrimão)" },
            { "nome": "COTAS_TEXTOS", "cor": "#00FF00", "aci": 3, "descricao": "Cotas lineares, marcação de níveis Z, nomes de ambientes com área em m² e carimbos" }
        ],
        "pranchetas": [
            "PR-01/04: Planta de Fundações e Baldrames",
            "PR-02/04: Planta Baixa do Pavimento Térreo",
            "PR-03/04: Planta Baixa do Pavimento Superior",
            "PR-04/04: Volumetria 3D e Perspectiva Axonométrica"
        ],
        "exemplo_cad_lote": {
            "dados": { "OX": 0.0, "OY": 0.0, "W": 10.0, "H": 8.0 },
            "passos": [
                { "ferramenta": "cad_folha", "argumentos": { "acao": "inserir", "papel": "A1", "escala": 50, "campos": { "titulo": "RESIDÊNCIA CONTEMPORÂNEA", "numero": "PR-01" }, "origem": true } },
                { "ferramenta": "cad_criar", "argumentos": { "camada": "ARQ_PAREDES_TERREO", "formas": [ { "tipo": "retangulo", "canto": "=[OX, OY]", "largura": 10.0, "altura": 8.0 } ] } }
            ]
        }
    })
}

fn modelo_eletrica() -> Value {
    json!({
        "nome": "eletrica",
        "titulo": "Projeto Elétrico Residencial NBR 5444 / NBR 5410",
        "unidade": "m",
        "escala": 50,
        "camadas": [
            { "nome": "ELE_PONTOS", "cor": "#FF0000", "aci": 1, "espessura": 0.5, "descricao": "QDC, Tomadas TUG/TUE, Interruptores, Pontos de Iluminação" },
            { "nome": "ELE_ELETRODUTOS", "cor": "#FF0000", "aci": 1, "espessura": 0.35, "descricao": "Tubulações embutidas em laje e parede" },
            { "nome": "ELE_CIRCUITOS", "cor": "#00FF00", "aci": 3, "descricao": "Marcação de circuitos, condutores (fase, neutro, terra, retorno)" }
        ],
        "simbolos_nbr5444": [
            "quadro_geral_embutido", "ponto_luz_teto", "tomada_baixa", "tomada_media", "tomada_alta", "interruptor_simples", "interruptor_paralelo"
        ],
        "exemplo_cad_lote": {
            "dados": { "QDC": [1.5, 5.0], "LUZ": [5.0, 4.0] },
            "passos": [
                { "ferramenta": "cad_simbolo", "argumentos": { "nome": "quadro_geral_embutido", "ponto": "=QDC", "escala": 50 } },
                { "ferramenta": "cad_simbolo", "argumentos": { "nome": "ponto_luz_teto", "ponto": "=LUZ", "escala": 50, "circuito": 1, "potencia": "100W" } },
                { "ferramenta": "cad_eletroduto", "argumentos": { "caminho": "= [QDC, LUZ]", "circuito": 1, "fase": 1, "neutro": 1, "terra": 1 } }
            ]
        }
    })
}

fn modelo_hidraulica() -> Value {
    json!({
        "nome": "hidraulica",
        "titulo": "Projeto Hidrossanitário: Água Fria e Esgoto",
        "unidade": "m",
        "escala": 50,
        "camadas": [
            { "nome": "HID_AGUA_FRIA", "cor": "#0000FF", "aci": 5, "descricao": "Reservatório 1000L, Coluna AF-1, Ramais e Sub-ramais" },
            { "nome": "HID_ESGOTO", "cor": "#FFFF00", "aci": 2, "descricao": "Tubulação de esgoto primário (DN100), secundário (DN50), Caixas Sifonadas (CS), Caixa de Gordura (CG) e Inspeção (CI)" },
            { "nome": "HID_VENTILACAO", "cor": "#00FFFF", "aci": 4, "descricao": "Coluna de Ventilação TEV Ø50mm subindo até o telhado" }
        ],
        "exemplo_cad_lote": {
            "dados": { "CG": [8.5, 8.5], "CI": [-0.5, -0.5] },
            "passos": [
                { "ferramenta": "cad_criar", "argumentos": { "camada": "HID_ESGOTO", "formas": [ { "tipo": "retangulo", "canto": "=CG", "largura": 0.6, "altura": 0.6 } ] } },
                { "ferramenta": "cad_criar", "argumentos": { "camada": "HID_ESGOTO", "formas": [ { "tipo": "polilinha", "pontos": "= [CG, CI]" } ] } }
            ]
        }
    })
}

fn modelo_estrutural() -> Value {
    json!({
        "nome": "estrutural",
        "titulo": "Projeto Estrutural e Locação de Fundações",
        "unidade": "m",
        "escala": 50,
        "camadas": [
            { "nome": "ESTR_EIXOS", "cor": "#FF00FF", "aci": 6, "tipo_linha": "eixo", "descricao": "Eixos de locação A, B, C... e 1, 2, 3..." },
            { "nome": "ESTR_PILARES", "cor": "#0000FF", "aci": 5, "descricao": "Pilares/Colunas estruturais (20x30cm)" },
            { "nome": "ESTR_SAPATAS", "cor": "#808080", "aci": 8, "descricao": "Sapatas isoladas e blocos de coroamento" },
            { "nome": "ESTR_VIGAS", "cor": "#00FFFF", "aci": 4, "descricao": "Vigas estruturais e lajes" }
        ]
    })
}

fn modelo_ferrovia() -> Value {
    json!({
        "nome": "ferrovia",
        "titulo": "Projeto de Via Permanente / Ferrovia",
        "unidade": "m",
        "escala": 100,
        "camadas": [
            { "nome": "FERROVIA_EIXO", "cor": "#FF00FF", "aci": 6, "tipo_linha": "eixo", "descricao": "Eixo da via e estaqueamento (estaca de 20 em 20m)" },
            { "nome": "FERROVIA_TRILHOS", "cor": "#FFFFFF", "aci": 7, "espessura": 0.5, "descricao": "Trilhos paralelos na bitola (1.60m ou 1.00m)" },
            { "nome": "FERROVIA_DORMENTES", "cor": "#808080", "aci": 8, "descricao": "Dormentes de concreto/madeira (2.80x0.24m a cada 0.60m)" },
            { "nome": "FERROVIA_AMV", "cor": "#00FFFF", "aci": 4, "descricao": "Aparelho de Mudança de Via (AMV / Chave)" }
        ],
        "exemplo_cad_lote": {
            "dados": { "BITOLA": 1.60, "PASSO_DORMENTE": 0.60, "NUM_DORMENTES": 20 },
            "passos": [
                { "definir": "trilho_dir", "valor": "= [[0.0, BITOLA/2], [NUM_DORMENTES * PASSO_DORMENTE, BITOLA/2]]" },
                { "definir": "trilho_esq", "valor": "= [[0.0, -BITOLA/2], [NUM_DORMENTES * PASSO_DORMENTE, -BITOLA/2]]" },
                { "ferramenta": "cad_criar", "argumentos": { "camada": "FERROVIA_TRILHOS", "formas": [ { "tipo": "polilinha", "pontos": "=trilho_dir" }, { "tipo": "polilinha", "pontos": "=trilho_esq" } ] } },
                { "para_cada": "=intervalo(NUM_DORMENTES)", "como": "i", "faca": [
                    { "definir": "x", "valor": "= i * PASSO_DORMENTE" },
                    { "ferramenta": "cad_criar", "argumentos": { "camada": "FERROVIA_DORMENTES", "formas": [ { "tipo": "retangulo", "canto": "=[x, -1.40]", "largura": 0.24, "altura": 2.80 } ] } }
                ] }
            ]
        }
    })
}

fn modelo_mecanica() -> Value {
    json!({
        "nome": "mecanica",
        "titulo": "Desenho Mecânico Industrial 2D/3D",
        "unidade": "mm",
        "escala": 1,
        "camadas": [
            { "nome": "MEC_CONTORNO", "cor": "#FFFFFF", "aci": 7, "espessura": 0.7, "descricao": "Arestas e contornos visíveis" },
            { "nome": "MEC_OCULTA", "cor": "#FFFF00", "aci": 2, "tipo_linha": "oculta", "descricao": "Arestas e furos ocultos" },
            { "nome": "MEC_EIXO", "cor": "#FF0000", "aci": 1, "tipo_linha": "eixo", "descricao": "Linhas de centro e eixos de simetria" },
            { "nome": "MEC_HACHURA", "cor": "#00FFFF", "aci": 4, "espessura": 0.25, "descricao": "Hachuras de corte (ANSI32 aço, ANSI31 ferro)" },
            { "nome": "MEC_COTAS", "cor": "#00FF00", "aci": 3, "descricao": "Cotas funcionais e tolerâncias" }
        ]
    })
}

/// Lista os nomes dos modelos disponíveis ou devolve o modelo pedido por `nome`.
pub fn exemplos(args: &Value) -> Res<Value> {
    let nome = texto_opcional(args, "nome");
    match nome.unwrap_or("").trim().to_lowercase().as_str() {
        "" | "listar" | "todos" => Ok(lista_modelos()),
        "arquitetura" => Ok(modelo_arquitetura()),
        "eletrica" => Ok(modelo_eletrica()),
        "hidraulica" => Ok(modelo_hidraulica()),
        "estrutural" => Ok(modelo_estrutural()),
        "ferrovia" => Ok(modelo_ferrovia()),
        "mecanica" => Ok(modelo_mecanica()),
        _ => Err(format!("Modelo `{}` não encontrado. Use `cad_exemplos` sem argumentos para listar todos.", nome.unwrap_or("")))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cad_exemplos_sem_argumentos_lista_modelos() {
        let res = exemplos(&json!({})).unwrap();
        let lista = res.get("modelos_disponiveis").unwrap().as_array().unwrap();
        assert!(!lista.is_empty());
        assert!(lista.iter().any(|m| m["nome"] == "arquitetura"));
        assert!(lista.iter().any(|m| m["nome"] == "ferrovia"));
    }

    #[test]
    fn cad_exemplos_com_nome_devolve_modelo() {
        let arq = exemplos(&json!({ "nome": "arquitetura" })).unwrap();
        assert_eq!(arq["nome"], "arquitetura");
        assert_eq!(arq["unidade"], "m");

        let fer = exemplos(&json!({ "nome": "ferrovia" })).unwrap();
        assert_eq!(fer["nome"], "ferrovia");
        assert_eq!(fer["unidade"], "m");
    }
}

