//! As ferramentas como o MCP as anuncia: nome, descrição (o que a IA lê para decidir) e o formato dos argumentos.

use serde_json::{json, Value};

const PONTO: &str = "ponto [x, y]";

fn def(nome: &str, descricao: &str, propriedades: &Value, obrigatorios: &[&str]) -> Value {
    json!({ "name": nome, "description": descricao, "inputSchema": { "type": "object", "properties": propriedades, "required": obrigatorios } })
}

fn pt() -> Value {
    json!({ "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": PONTO })
}

fn janela() -> Value {
    json!({ "type": "object", "properties": { "x1": { "type": "number" }, "y1": { "type": "number" }, "x2": { "type": "number" }, "y2": { "type": "number" } }, "required": ["x1", "y1", "x2", "y2"], "description": "retângulo do desenho, em coordenadas do desenho" })
}

fn filtros() -> Value {
    json!({
        "tipo": { "type": "string", "enum": ["linha", "polilinha", "circulo", "arco", "texto", "ponto", "hachura", "bloco"] },
        "camada": { "type": "string" },
        "ids": { "type": "array", "items": { "type": "integer" } },
        "janela": janela(),
        "modo": { "type": "string", "enum": ["dentro", "toca"], "description": "com `janela`: `dentro` = só o que cabe inteiro (padrão); `toca` = basta tocar" }
    })
}

/// Os arquivos e o estado geral.
fn de_arquivo() -> Vec<Value> {
    vec![
        def("cad_novo", "Começa um desenho em branco (descarta o atual). Sempre pergunte/escolha a unidade: as coordenadas são números puros e a unidade diz o que eles significam. Convenção: X para a direita, Y para cima, ângulos em graus anti-horário a partir do eixo X.", &json!({ "unidade": { "type": "string", "enum": ["mm", "cm", "m"] } }), &["unidade"]),
        def("cad_abrir", "Abre um arquivo `.dxf`, `.fcdz` ou `.fct` (formato compacto) do disco no lugar do desenho atual. Devolve avisos do que não foi possível ler.", &json!({ "caminho": { "type": "string" } }), &["caminho"]),
        def("cad_salvar", "Grava o desenho em `.dxf`, `.fcdz` ou `.fct` (pela extensão). Sem `caminho`, grava onde foi aberto. Não sobrescreve arquivo existente sem `sobrescrever: true`.", &json!({ "caminho": { "type": "string" }, "sobrescrever": { "type": "boolean" } }), &[]),
        def("cad_converter", "Converte um arquivo em outro **sem** passar pelo desenho aberto nem gastar tokens: DXF/FCDZ → `.fct` (texto compacto, muito menor) e o contrário. Devolve os tamanhos antes e depois.", &json!({ "origem": { "type": "string" }, "destino": { "type": "string" }, "sobrescrever": { "type": "boolean" } }), &["origem", "destino"]),
        def("cad_pdf", "Grava o desenho em PDF VETORIAL (zoom sem perder nitidez; linha fina em fio de cabelo), uma página por folha (blocos `FOLHA-n` de cad_folha e retângulos fechados da camada FOLHA; sem folha, o desenho inteiro). `papel` A4, A3 (padrão), A2, A1, A0, 2A0, 4A0, 8A0, 16A0 ou FOLHA (tamanho da folha, 1 unidade = 1 mm); `orientacao` automatica|horizontal|vertical; `cores` true mantém as cores da tela (senão preto sobre branco); `folhas` [índices] escolhe páginas; não sobrescreve sem `sobrescrever: true`.", &json!({ "caminho": { "type": "string" }, "papel": { "type": "string" }, "orientacao": { "type": "string", "enum": ["automatica", "horizontal", "vertical"] }, "cores": { "type": "boolean" }, "folhas": { "type": "array", "items": { "type": "integer" } }, "sobrescrever": { "type": "boolean" } }), &["caminho"]),
        def("cad_resumo", "Visão geral barata (comece por aqui): unidade, extensão (caixa do desenho), quantas entidades por tipo e por camada, camadas, arquivo, se há alterações não salvas.", &json!({}), &[]),
        def("cad_desfazer", "Desfaz o último passo (cada chamada de criar/editar/hachurar é um passo só). Com `refazer: true`, refaz.", &json!({ "refazer": { "type": "boolean" } }), &[]),
    ]
}

/// As que dão noção de espaço: ver, listar, medir, regiões, verificar, texto compacto.
fn de_consulta() -> Vec<Value> {
    let ver = json!({
        "janela": janela(),
        "largura": { "type": "integer", "description": "largura da imagem em pixels (300 a 2400; padrão 1000)" },
        "rotulos": { "type": "boolean", "description": "escreve o #id de cada entidade ao lado dela (padrão sim)" },
        "grade": { "type": "boolean", "description": "grade com números nas bordas (padrão sim)" },
        "destacar": { "type": "array", "items": { "type": "integer" }, "description": "ids a realçar em azul" }
    });
    let perto = json!({ "type": "object", "properties": { "ponto": pt(), "raio": { "type": "number" } }, "required": ["ponto", "raio"], "description": "só o que está a até `raio` do ponto (traz `distancia` e `direcao_do_ponto`)" });
    let mut ent = filtros();
    ent["perto"] = perto;
    ent["limite"] = json!({ "type": "integer", "description": "máximo de entidades (padrão 100, até 500)" });
    let mut exp = filtros();
    exp["com_ids"] = json!({ "type": "boolean", "description": "escreve @id em cada linha, para poder editar depois (padrão não: mais barato)" });
    vec![
        def("cad_ver", "Uma IMAGEM do desenho (PNG) para você enxergar: grade com coordenadas nas bordas, eixos x/y, barra de escala e o #id de cada entidade. Sem `janela` mostra tudo; com `janela` dá zoom numa região. Depois de desenhar, confira com isto. A resposta diz a janela exata e quantas unidades vale cada pixel.", &ver, &[]),
        def("cad_entidades", "Lista entidades com medidas exatas (comprimento, área, centro, raio, ângulo, caixa…), filtrando por tipo, camada, ids, janela ou proximidade de um ponto. É a forma de saber ONDE as coisas estão e QUANTO medem.", &ent, &[]),
        def("cad_exportar", "O desenho em TEXTO COMPACTO (uma linha por objeto: `L x1 y1 x2 y2`, `C cx cy r`, `R x y l a`…), várias vezes menor que DXF (e o DXF de outros programas, como o do AutoCAD, é ainda mais pesado). Use para ler o desenho inteiro ou uma parte gastando poucos tokens. Blocos são abertos em formas soltas.", &exp, &[]),
        def("cad_importar", "Cria formas a partir de texto compacto (o mesmo de `cad_exportar`). Uma chamada = um passo de desfazer. Com `substituir: true`, apaga o desenho antes. Erros dizem a linha.", &json!({ "texto": { "type": "string" }, "substituir": { "type": "boolean" } }), &["texto"]),
        def("cad_medir", "Medições exatas. `{de, para}` (cada um ponto [x,y] ou {id}) → distância, dx, dy, ângulo; entre entidades, a distância mínima. `{pontos:[…]}` → área e perímetro do polígono. `{vertice, a, b}` → ângulo entre as duas direções.", &json!({ "de": {}, "para": {}, "pontos": { "type": "array", "items": pt() }, "vertice": pt(), "a": pt(), "b": pt() }), &[]),
        def("cad_regioes", "As áreas FECHADAS que os traços formam (salas, peças), com área, perímetro, centro e os textos que caem dentro; `area_util` já desconta vãos. Com `ponto`, só a que contém o ponto (o que uma hachura preencheria). Se uma região esperada não aparece, há abertura no contorno: use cad_verificar.", &json!({ "ponto": pt(), "limite": { "type": "integer" } }), &[]),
        def("cad_verificar", "Procura defeitos: entidades de comprimento zero, duplicadas e FOLGAS (extremidade que quase toca outra forma sem tocar). Rode depois de desenhar contornos que deveriam fechar.", &json!({ "tolerancia": { "type": "number", "description": "distância máxima que conta como folga (padrão 0,2% da diagonal do desenho)" } }), &[]),
    ]
}

/// As que mudam o desenho.
fn de_edicao() -> Vec<Value> {
    let forma = json!({ "type": "object", "properties": { "tipo": { "type": "string", "enum": ["linha", "retangulo", "poligono", "polilinha", "circulo", "arco", "hachura", "texto"] } }, "required": ["tipo"], "description": "linha: {a,b} ou {de,distancia,angulo}; retangulo: {canto1,canto2} ou {canto,largura,altura,rotacao?} (+ raio: cantos arredondados); hachura: {pontos} ou {contornos:[[pts],[pts]]} (o 2º em diante são ilhas), padrao (solida, concreto, terra…), escala; poligono: {centro,raio,lados,rotacao?}; polilinha: {pontos,fechada?}; circulo: {centro,raio}; arco: {centro,raio,inicio,fim} (graus, anti-horário) ou {p1,p2,p3}; texto: {ponto,altura,conteudo,rotacao?}" });
    let mut ed = filtros();
    ed["operacao"] = json!({ "type": "string", "enum": ["mover", "copiar", "girar", "escala", "espelhar", "apagar", "propriedades", "texto"], "description": "texto: {conteudo?, altura?} troca o texto (e/ou a altura) dos textos escolhidos; mover/copiar: {dx,dy} ou {de,para} (copiar aceita `quantidade`); girar: {base,graus,copiar?}; escala: {base,fator}; espelhar: {p1,p2,manter_original?=true}; propriedades: {para_camada?, para_cor? (#RRGGBB ou por_camada)}" });
    for c in ["dx", "dy", "graus", "fator", "quantidade"] {
        ed[c] = json!({ "type": "number" });
    }
    ed["altura"] = json!({ "type": "number", "description": "operacao texto: nova altura do texto" });
    ed["conteudo"] = json!({ "type": "string", "description": "operacao texto: o novo conteúdo" });
    for c in ["base", "p1", "p2", "de", "para"] {
        ed[c] = pt();
    }
    vec![
        def("cad_criar", "Desenha uma ou mais formas de uma vez (um passo de desfazer). Coordenadas na unidade do desenho. `camada` (criada se não existir) e `cor` (#RRGGBB) valem para todas as formas da chamada. Confira depois com cad_ver.", &json!({ "formas": { "type": "array", "items": forma }, "camada": { "type": "string" }, "cor": { "type": "string" } }), &["formas"]),
        def("cad_editar", "Transforma ou apaga entidades já existentes, escolhidas por `ids` ou por filtros (tipo, camada, janela). Entidades de camada bloqueada são puladas.", &ed, &["operacao"]),
        def("cad_camadas", "Camadas: `listar` (padrão), `criar` {nome, cor?, espessura?, tipo_linha?}, `atual` {nome}, `propriedade` {nome, ligada?, congelada?, bloqueada?, cor?, espessura?, tipo_linha?}. Espessura em mm (paredes ~0,5; eletroduto ~0,35); tipo_linha tracejada para tubulação no piso.", &json!({ "acao": { "type": "string", "enum": ["listar", "criar", "atual", "propriedade"] }, "nome": { "type": "string" }, "cor": { "type": "string" }, "ligada": { "type": "boolean" }, "congelada": { "type": "boolean" }, "bloqueada": { "type": "boolean" }, "espessura": { "type": "number", "description": "espessura de linha da camada em mm (0 = padrão); vale para os objetos \"por camada\"" }, "tipo_linha": { "type": "string", "enum": ["continua", "tracejada", "oculta", "eixo", "pontilhada", "fantasma"] } }), &[]),
        def("cad_hachurar", "Preenche a região fechada que contém `ponto` (contorno de linhas, arcos, polilinhas ou círculo). `solida` padrão sim; senão, linhas a 45°. `padrao` escolhe da biblioteca: solida, ansi31 (45°), ansi32 (aço), ansi37 (cruzada), horizontal, vertical, grade, pontos, concreto, areia, brita, terra, tijolo; `escala` (padrão 1) multiplica o espaço do padrão (maior = mais aberto). Falha se o contorno tem abertura.", &json!({ "ponto": pt(), "solida": { "type": "boolean" }, "padrao": { "type": "string", "description": "solida, ansi31, ansi32, ansi37, horizontal, vertical, grade, pontos, concreto, areia, brita, terra, tijolo" }, "escala": { "type": "number", "description": "multiplica o espaço do padrão (0,25 a 4)" } }), &["ponto"]),
        def("cad_folha", "FOLHAS (pranchetas) — a mesma do botão Inserir folha do aplicativo: moldura, marcas de centro, faixa de notas/legenda/revisões e carimbo, como um bloco `FOLHA-n` na escala do desenho. `acao`: `listar` (padrão: folhas, papéis e escalas), `inserir` {papel A4…16A0 (padrão A1), escala (denominador; sem ela usa a menor em que o desenho cabe), campos {cliente, fase, disciplina, titulo, subtitulo, numero, revisao, folha, divisoria}, origem:true (canto inferior esquerdo em 0,0) ou ponto [x,y]}, `escala` {indice, escala} (troca mantendo o centro da área útil), `ajustar` {indice} (menor escala em que o desenho todo cabe), `campos` {indice}, `campos_definir` {indice, campos}. A área útil de uma A1 vai de (35;109) a (821;574) mm de papel; a faixa de notas/legenda fica em y 10 a 99. Em desenho em mm a 1:1, a folha tem o tamanho do papel.", &json!({ "acao": { "type": "string", "enum": ["listar", "inserir", "escala", "ajustar", "campos", "campos_definir"] }, "papel": { "type": "string" }, "escala": { "type": "number" }, "indice": { "type": "integer" }, "campos": { "type": "object" }, "origem": { "type": "boolean" }, "ponto": pt() }), &[]),
        def("cad_contar", "CONTA objetos: `igual_a` {id} (todos com o mesmo tipo e medida, por camada) ou `tipo` (retangulo | circulo | linha) + `medida` ({largura, altura} | {raio} | {comprimento}) + `tolerancia` (relativa, padrão 0,01) e `camada` opcional. Retângulo vale em qualquer giro e deitado ou em pé. `ids: N` devolve os N primeiros ids. Ex.: dormentes = tipo retangulo, medida {largura:0.24, altura:2}.", &json!({ "igual_a": { "type": "integer" }, "tipo": { "type": "string", "enum": ["retangulo", "circulo", "linha"] }, "medida": { "type": "object" }, "tolerancia": { "type": "number" }, "camada": { "type": "string" }, "ids": { "type": "integer" } }), &[]),
    ]
}

/// Projeto elétrico (NBR 5444): símbolos prontos e eletroduto com condutores. Só para projeto elétrico.
fn de_eletrica() -> Vec<Value> {
    let simbolo = json!({
        "nome": { "type": "string", "description": "nome do símbolo (veja cad_simbolos): tomada_baixa|tomada_media|tomada_alta|tomada_piso|ponto_luz_teto|ponto_luz_embutido|arandela|ponto_luz_vapor_mercurio|interruptor_simples|interruptor_paralelo|interruptor_intermediario|interruptor_duas_secoes|quadro_geral_embutido|quadro_geral_aparente|quadro_parcial_embutido|quadro_parcial_aparente|caixa_telefone|caixa_medidor|caixa_passagem_piso|luz_multifilar|disjuntor|no_de_ligacao" },
        "ponto": pt(),
        "escala": { "type": "number", "description": "denominador da escala do desenho (50 = 1:50, padrão 50): o símbolo tem o tamanho de papel certo" },
        "parede": { "type": "string", "enum": ["norte", "sul", "leste", "oeste"], "description": "tomadas: de que lado do símbolo está a parede (ele aponta para o lado oposto)" },
        "rotacao": { "type": "number", "description": "giro em graus anti-horário (alternativa a `parede`); o texto continua na horizontal" },
        "quantidade": { "type": "integer", "description": "tomada: 1/2/3 = simples/dupla/tripla; luz: número de lâmpadas (vira 2x100W)" },
        "circuito": { "type": ["integer", "string"], "description": "número do circuito (na luz aparece como -4-)" },
        "comando": { "type": "string", "description": "letra minúscula do interruptor que comanda o ponto (a, b, c…)" },
        "potencia": { "type": ["number", "string"], "description": "100 vira 100W; ou texto como 600VA" },
        "altura_texto": { "type": "number", "description": "altura do texto em mm de papel (padrão 1,8)" },
        "camada": { "type": "string" },
        "cor": { "type": "string", "description": "#RRGGBB" }
    });
    let conduto = json!({ "type": "object", "properties": {
        "circuito": { "type": ["integer", "string"] },
        "tipo": { "type": "string", "enum": ["fase", "neutro", "retorno", "terra"] },
        "secao": { "type": "number", "description": "mm² (padrão 1,5)" },
        "quantidade": { "type": "integer", "description": "quantos condutores iguais (ex.: 2 fases)" }
    }, "required": ["tipo"] });
    let eletroduto = json!({
        "pontos": { "type": "array", "items": pt(), "minItems": 2, "description": "caminho do eletroduto, em coordenadas do desenho" },
        "condutores": { "type": "array", "items": conduto, "description": "o que passa dentro: um item por tipo/circuito (fase, neutro, retorno, terra)" },
        "tipo": { "type": "string", "enum": ["teto", "parede", "piso"], "description": "teto/parede = linha contínua; piso = tracejada (padrão teto)" },
        "diametro": { "type": "number", "description": "Ø em mm; sem ele, calcula pela ocupação de 40 %" },
        "terra": { "type": "boolean", "description": "acrescenta um condutor terra (padrão sim)" },
        "secao_terra": { "type": "number" },
        "escala": { "type": "number", "description": "denominador da escala (padrão 50)" },
        "posicao": { "type": "number", "description": "0 a 1: onde, ao longo do trecho mais comprido, ficam os traços (padrão 0,5)" },
        "rotulo": { "type": "boolean", "description": "escreve circuitos, seções e Ø (padrão sim)" },
        "curva": { "type": "number", "description": "só com 2 pontos: faz o eletroduto curvo, como no unifilar da NBR (0,1 a 0,3; + curva para a esquerda de a→b, − para a direita). Evita linhas retas cruzando a planta." },
        "espessura": { "type": "number", "description": "espessura da linha do eletroduto em mm (ex. 0,35), mais grossa que o resto" },
        "altura_texto": { "type": "number" },
        "camada": { "type": "string" },
        "cor": { "type": "string" }
    });
    vec![
        def("cad_simbolos", "PROJETO ELÉTRICO (NBR 5444) — use só se o projeto for elétrico. Lista a biblioteca de símbolos prontos (tomada baixa/média/alta/piso, ponto de luz, interruptores S/S3/S4, quadros, multifilar…) e o que cada um aceita.", &json!({}), &[]),
        def("cad_simbolo", "PROJETO ELÉTRICO (NBR 5444) — use só se o projeto for elétrico. Coloca um símbolo pronto pelo nome, no tamanho certo para a escala, com os rótulos (circuito, comando, potência). Exemplo: {nome:\"tomada_media\", ponto:[100,20], parede:\"sul\", circuito:3}. Um passo de desfazer.", &simbolo, &["nome", "ponto"]),
        def("cad_componentes", "BIBLIOTECA DE COMPONENTES (arquivos .fct prontos: disjuntor modular, DR, DPS, trilho, barras, caixa de quadro, transformador…). Lista o que existe com parâmetros e tamanho; `busca` filtra por texto. Use ANTES de desenhar qualquer equipamento elétrico: chamar o componente gasta muito menos que redesenhar.", &json!({ "busca": { "type": "string" } }), &[]),
        def("cad_componente", "Coloca um COMPONENTE pronto da biblioteca (veja cad_componentes) com o canto inferior esquerdo em `ponto`, em milímetros reais. Exemplo: {nome:\"disjuntor_modular_2p\", ponto:[62,110], valores:{corrente:\"C25\", icc:\"6kA\"}}. `ampliacao` (padrão 1) e `rotacao` (graus) opcionais. Um passo de desfazer; funciona dentro de cad_lote.", &json!({ "nome": { "type": "string" }, "ponto": pt(), "valores": { "type": "object", "description": "parâmetros do componente {nome: valor}, os que cad_componentes mostra" }, "ampliacao": { "type": "number" }, "rotacao": { "type": "number" }, "camada": { "type": "string" } }), &["nome", "ponto"]),
        def("cad_eletroduto", "PROJETO ELÉTRICO (NBR 5444) — use só se o projeto for elétrico. Desenha um eletroduto (contínuo no teto/parede, tracejado no piso) com os traços dos condutores (fase cruza; neutro cruza com bandeira; retorno meio traço; terra meio traço com T), números dos circuitos em cima, seção embaixo e Ø. Um eletroduto compartilhado por vários circuitos é uma chamada só, com todos os condutores.", &eletroduto, &["pontos", "condutores"]),
    ]
}

/// O lote: dados + laços declarativos + expressões.
fn de_lote() -> Vec<Value> {
    let passo = json!({ "type": "object", "description": "UM destes: {ferramenta, argumentos, guardar?} | {definir:\"nome\", valor} | {para_cada, como?, indice?, faca:[passos]} | {repetir, como?, faca:[passos]} | {se, entao:[passos], senao?:[passos]} | {exigir, mensagem?}" });
    let props = json!({
        "dados": { "type": "object", "description": "variáveis de partida {nome: valor}: números, textos, listas, objetos (tabelas de pontos, circuitos…)" },
        "passos": { "type": "array", "items": passo },
        "retornar": { "type": "array", "items": { "type": "string" }, "description": "nomes de variáveis a devolver no resultado (as guardadas com `guardar` ou `definir`)" }
    });
    vec![def("cad_lote", "Executa MUITAS ferramentas numa chamada só, com DADOS e LAÇOS feitos pelo servidor (você manda os dados uma vez e o molde uma vez; não repita chamadas). Tudo ou nada: se um passo falha, nada é aplicado e o erro diz o passo e a volta (`passo [2].faca(volta 4)[0]: …`); se dá certo, é UM passo de desfazer. Passos: {ferramenta:\"cad_simbolo\", argumentos:{…}, guardar:\"nome\"} chama qualquer ferramenta (menos cad_ver/cad_novo/cad_abrir/cad_desfazer/cad_lote); {definir:\"x\", valor:\"=expr\"}; {para_cada:\"lista\" (nome de variável ou =expr), como:\"l\", indice:\"i\", faca:[…]}; {repetir:\"=n\", como:\"i\", faca:[…]}; {se:\"=cond\", entao:[…], senao:[…]}; {exigir:\"=cond\", mensagem:\"…\"} valida dados. Em qualquer valor de argumento: texto que começa com `=` é EXPRESSÃO e vira o valor (número, ponto, lista…); `${expr}` dentro de um texto é trocado pelo valor; `==` no começo escreve um `=` literal. Expressões: + - * / %, comparações, && || !, c ? a : b, índice l[0] (negativo = do fim), campo c.nome, lista [x, y]; contas com listas de números valem elemento a elemento ([1,2]+[10,20], [3,4]*2), então pontos se somam; + com texto junta. Funções: abs min max soma media arredondar(x,casas) piso teto raiz potencia sen cos tan atan2 (graus) dist(a,b) angulo(a,b) ponto_polar(o,ang,d) ponto_medio comprimento(caminho) deslocar(caminho,d) (paralela à esquerda; d<0 direita) intervalo(n) tamanho texto numero virgula(x,casas) se(c,a,b) juntar contem inverter soma_campo(lista,\"campo\") onde(lista,\"campo\",valor). Exemplo: {dados:{X:300,luzes:[[250,200,\"a\"]]}, passos:[{para_cada:\"luzes\", como:\"l\", faca:[{ferramenta:\"cad_simbolo\", argumentos:{nome:\"ponto_luz_teto\", ponto:\"=[X + l[0], l[1]]\", comando:\"=l[2]\", circuito:1}}]}]}. Devolve chamadas, voltas, entidades_novas e as variáveis pedidas em `retornar`. Limites: 50 mil chamadas, 200 mil voltas, 16 níveis. Gravar arquivo (cad_salvar) não é desfeito se um passo posterior falhar: deixe por último.", &props, &["passos"])]
}

/// Todas as ferramentas, na ordem em que aparecem para a IA.
pub fn todas() -> Value {
    let mut v = de_arquivo();
    v.extend(de_consulta());
    v.extend(de_edicao());
    v.extend(de_lote());
    v.extend(de_eletrica());
    Value::Array(v)
}
