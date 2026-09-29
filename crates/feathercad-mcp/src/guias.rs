//! Os **prompts** do MCP: guias que o cliente oferece ao usuário/IA para uma família de projetos.
//! Hoje só há o de **projeto elétrico**; ele vale apenas quando o projeto é elétrico (NBR 5444/5410) e
//! deve ser ignorado em qualquer outro tipo de desenho.

use serde_json::{json, Value};

pub const ELETRICO: &str = "projeto_eletrico";

/// A lista de prompts (`prompts/list`).
pub fn listar() -> Value {
    json!([{
        "name": ELETRICO,
        "title": "Projeto elétrico (NBR 5444 / 5410)",
        "description": "Use SOMENTE quando o projeto for elétrico (planta de instalações, quadro de distribuição, diagramas). Simbologia, folhas, eletrodutos, condutores, multifilar e verificações. Ignore em qualquer outro tipo de projeto.",
        "arguments": []
    }])
}

/// O conteúdo de um prompt (`prompts/get`).
pub fn obter(nome: &str) -> Option<Value> {
    (nome == ELETRICO).then(|| json!({
        "description": "Guia de projeto elétrico no FeatherCAD",
        "messages": [{ "role": "user", "content": { "type": "text", "text": TEXTO_ELETRICO } }]
    }))
}

const TEXTO_ELETRICO: &str = "\
GUIA DE PROJETO ELÉTRICO (valha só se o projeto for elétrico; senão ignore).

1. FOLHA. Todo projeto vai numa FOLHA: moldura com margem, carimbo (título, folha n/N, escala, data, unidade) e um assunto por folha \
(planta baixa; iluminação e interruptores; tomadas e circuitos; quadro de cargas + unifilar/multifilar). Folhas ficam lado a lado no mesmo \
desenho. A3 a 1:50 em cm = 2100 x 1485 cm; a 1:100 = 4200 x 2970. O tamanho de texto/símbolo é medido NO PAPEL: altura_desenho = mm_papel x escala \
(em cm, 1 mm de papel a 1:50 = 5 cm): texto 1,8 a 2,5 mm de papel.

2. UNIDADE E ESCALA. Pergunte a unidade (cad_novo) e a escala (padrão 1:50). Os símbolos e eletrodutos (cad_simbolo, cad_eletroduto) já recebem `escala` \
e desenham no tamanho certo; não os redesenhe com linhas soltas.

3. SIMBOLOGIA (NBR 5444) — use as ferramentas, não desenhe à mão:
   - cad_simbolos lista o catálogo. Tomada: tomada_baixa (triângulo vazio, ~30 cm), tomada_media (meio cheio, ~110 cm), tomada_alta (cheio, ~200 cm), \
tomada_piso; `quantidade` 1/2/3 = simples/dupla/tripla; `parede` diz de que lado está a parede; `circuito` e `potencia` viram rótulo.
   - Ponto de luz (unifilar): ponto_luz_teto / ponto_luz_embutido / arandela / ponto_luz_vapor_mercurio com `comando` (letra minúscula do interruptor que o comanda), \
`circuito` (aparece como -4-) e `potencia` (com `quantidade` de lâmpadas: 2x100W).
   - Interruptores: simples (S), paralelo/three-way (S3, 2 pontos), intermediário/four-way (S4, 3 ou mais: S3 - S4 - S3), duas seções (SS); sempre com a letra do comando.
   - Quadros: quadro_geral_embutido/aparente, quadro_parcial_*; caixa_medidor, caixa_telefone, caixa_passagem_piso.
   - Multifilar: luz_multifilar, disjuntor, no_de_ligacao.

4. ELETRODUTOS E CONDUTORES — cad_eletroduto desenha o eletroduto (`tipo`: teto/parede = linha contínua; piso = tracejada), a bandeirinha com o diâmetro Ø \
e os traços dos condutores: FASE = traço que cruza; NEUTRO = traço que cruza com bandeirinha; RETORNO = meio traço; TERRA = meio traço com T. Em cima vão os números dos \
circuitos que passam ali; embaixo, a seção (só quando não é 1,5 mm²). Um eletroduto físico compartilhado por vários circuitos é UMA linha com todos os condutores dentro \
(ex.: circuitos 1, 3, 4 = fases e neutros de cada um + um terra). Diâmetro: deixe em automático (ocupação máx. 40 %) ou informe. Ao chegar num interruptor levam-se \
retorno(s) e, no paralelo/intermediário, dois retornos entre os interruptores.

5. CIRCUITOS E FASES. Numere os circuitos (1, 2, 3…), dê a cada ponto de luz a letra do seu comando e o número do circuito, e distribua as cargas entre as fases \
(A/B/C ou F1/F2) para equilibrar. Bifásico = 2 fases + neutro + terra (127/220 V); chuveiro entre as duas fases, sem neutro. Calcule a corrente (P/V), escolha o \
disjuntor e a seção, e verifique a queda de tensão no ponto mais distante de cada circuito (dV% = 2·ρ·L·I / (S·V), ρ = 0,0225 Ω·mm²/m; máx. 4 % no circuito terminal). \
Indique o PIOR PONTO. Cargas: iluminação 100 VA por ponto; TUG 100 VA (cozinha/banho: 600 VA nas 3 primeiras); TUE pela potência do aparelho.

6. QUADRO. Desenhe o quadro de cargas (circuito, descrição, potência, tensão, fase, corrente, disjuntor, seção, comprimento, dV %), o diagrama MULTIFILAR \
(barras verticais A B C N T, disjunto geral, DPS, IDR, e uma linha horizontal por circuito partindo de um nó de ligação na barra da sua fase, com o disjuntor e as tabelas \
LOCAL / SEÇÃO / PROTEÇÃO / CIRC. dos dois lados) e o quadro em vista frontal. Circuitos de bipolar (220 V) ligam-se às duas barras de fase.

7. CONFERIR. Depois de desenhar: cad_ver (uma folha por vez, com `janela`), cad_verificar, cad_resumo. Acentos e símbolos como Ø, ², ° funcionam. Para repetir muitos símbolos/eletrodutos, \
use cad_lote (dados + para_cada): manda a tabela de pontos uma vez e o molde uma vez, em vez de centenas de chamadas. Para geometria solta, escreva um .fct e abra com cad_abrir.
";
