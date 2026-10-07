# Plano 10× — FeatherCAD contra o CADCraft (07/10/2026)

Comparação feita com um clone do `storytold/cadcraft` (MIT/Apache, ~50 mil linhas de Rust, 290 comandos,
47% dos menus do AutoCAD). **Regra deste plano: o que já existe no FeatherCAD só melhora; nada é refeito do zero.**

## 1. O que "10×" quer dizer (metas que se medem)

"10× melhor" não cabe em tudo; o CADCraft tem 3 anos de AutoCAD na cabeça e nós temos 8 dias. Então a meta é
**10× onde o CADCraft é fraco ou ausente** e **paridade onde ele é forte e o usuário usa todo dia**.

| Eixo | CADCraft hoje | Meta FeatherCAD | Como medir |
|---|---|---|---|
| Seleção em desenho pesado (`projeto_pesado`, 250 mil objetos) | clique 0,001 ms; depois de editar ≈ 4 ms | **clique ≤ 0,001 ms; selecionar tudo ≤ 50 ms; editar sem recriar índice (≤ 1 ms)** | `cargo bench` com `projeto_pesado.fcdz` |
| Abrir arquivo grande | não publicado | `projeto_pesado.dxf` (34 MB) em **≤ 1 s**; `.fcdz` em ≤ 300 ms | benchmark de abrir |
| Projeto feito pela IA (MCP) | ~500 linhas: digita na linha de comando | **projeto completo numa chamada** (já temos `cad_lote`, `cad_projeto`, modelos) | tokens e chamadas por projeto-modelo |
| Disciplinas de engenharia | nenhuma | elétrica, viário, ferrovia, topografia, nuvem de pontos (**já temos**) | pacote de prancha por disciplina |
| Entrega para o projetista brasileiro | em inglês, sem norma | pt-BR, NBR 5444, folha/carimbo ABNT | projeto-modelo aceito sem retrabalho |
| Formatos | DXF R12–2018 (ASCII+binário), DWG | **DXF R2018 gravando e lendo sem perda; DWG pelo `acadrust`** | ida e volta + abrir no AutoCAD/LibreCAD |
| Comandos de desenho 2D de uso diário | 290 | **os ~60 que cobrem 95% do uso**, todos com mouse, teclado e MCP | lista em `docs/FUNCIONALIDADES.md` |

O que **não** perseguimos: 3D (200 h só nisso), AutoLISP, folhas de conjunto (sheet sets). Fora do foco.

## 2. Fase 0 — Desempenho de seleção (primeiro; é o que você sentiu travar)

O desenho renderiza bem, mas selecionar trava no `projeto_pesado`. Diagnóstico por leitura de código, **ainda não medido**:
`Documento::entidade(id)` é busca linear (`documento.rs:617`) e `podar_selecao`, `expandir_grupos`, a prévia, mover,
apagar e esticar chamam isso por objeto selecionado → N × 250 mil.

0. **O que já está no código:** pintura em Worker, LOD (decimação, traço de 1 px, bloco < 2 px vira ponto), snap limitado a 48 primitivas, quadro binário por IPC. Ou seja, **o quadro já é rápido; o que resta linear é a escolha de objetos e a seleção em lote** (o próprio `CLAUDE.md` anota: "o motor varre todas as entidades por quadro e por movimento do cursor… índice espacial só se isso pesar"). Pesou.
1. **Medir antes:** benchmark que abre `projeto_pesado.fcdz` e cronometra clique, janela, selecionar tudo, mover, apagar. Guardar o número.
1b. **Linha de base medida (07/10/2026, release, `projeto_pesado.fcdz`, 200 983 objetos;** `cargo run --release --example medir_selecao -- arquivo` em `crates/feathercad-ponte`**):** abrir 542 ms · clique 38 ms · janela que pega tudo **29,4 s** · propriedades da seleção **26,2 s** · apagar a seleção **230,9 s** · quadro com tudo selecionado 81 ms · desfazer 8 ms. Confirma: o quadro é rápido, a seleção em lote é quadrática.
   **✅ Item 2 feito (07/10/2026, não commitado):** `Documento::posicao`/`entidade` com índice id → posição por versão do desenho (`RwLock`, montado sob demanda) e `aplicar_mudancas` com remoção em lote (`documento.rs`). Medido no `projeto_pesado`: janela que pega tudo 29,4 s → **115 ms**, propriedades da seleção 26,2 s → **32 ms**, apagar a seleção 230,9 s → **166 ms**. Clique continua em 38 ms (item 3, R-tree). Testes core/io/mcp verdes.
2. **Mapa id → posição** no `Documento` (`HashMap<Id, usize>` ou `Vec` indexado por id), mantido em inserir/remover/desfazer. Troca `entidade()` de O(n) para O(1). Efeito imediato na seleção.
3. **R-tree** (`rstar`) sobre as caixas das entidades: `escolher_com` e `escolher_na_janela` deixam de varrer tudo. Atualização **incremental** por `Mudanca` (o histórico já descreve cada mudança, então o índice acompanha o mesmo caminho).
4. **Seleção em lote:** `expandir_grupos` com índice grupo → ids (hoje varre a lista toda); `podar_selecao` só roda sobre o que mudou.
5. **Quadro:** a seleção grande não pode refazer o quadro inteiro; reaproveitar o cache por versão e só reenviar a camada de seleção.
6. **Depois do ganho:** `cargo bench` entra no `cargo test`/CI como alerta de regressão (falha se passar de 2× o número guardado).

Pronto quando: selecionar tudo no `projeto_pesado` ≤ 50 ms e nenhum clique > 1 ms. Estimativa: 1 sessão.

## 3. Fase 1 — Terminar o 2D diário (o que já existe só melhora)

Inventário em 07/10 (motor; o `CLAUDE.md` é a fonte, e a parte dele sobre o leitor de DXF está defasada): **tem** grab, janela por arrasto, linha, polilinha, círculo, arco, retângulo, elipse (aproximada por polilinha), ponto, texto, hachura
(pick-point com ilhas), bloco/inserção, grupos, cotas linear/alinhada/raio/diâmetro, chamada, tabela, medir, mover, copiar, girar,
escala, espelhar, esticar, matriz, aparar, estender, concordância (fillet), deslocar (offset), camadas, folhas, viewports (9 usos).

| Falta / fraco | Ação |
|---|---|
| **Pegas (grips)** nos vértices/pontos médios | Hoje só há *grab* (arrastar o objeto inteiro) e janela por arrasto, ambos prontos desde 29/09. A pega edita um vértice: esticar/mover/girar/escalar/espelhar. O motor já tem `esticar` e `Transf`; falta a pega escolher o vértice. |
| **Rastreio polar e de objeto** | Já existem: Orto com passo de ângulo, `@d<a`, `<a` travando direção e snap de 9 tipos. Falta a **linha-guia de rastreio** que aparece ao passar perto do ângulo/de um ponto de snap (polar tracking e object snap tracking). |
| **Chanfro, quebrar, unir, explodir, alongar (LENGTHEN)** | Cada um é uma função de geometria pura em `edicao.rs`; ganha teste, atalho (`ATALHOS.md`) e linha na Ribbon. |
| **Spline, MTEXT, XLINE, polígono, donut** | Spline e MTEXT primeiro (o leitor de DXF já lê as duas e hoje simplifica). Spline vira `Geometria::Spline` com avaliação em Rust. |
| **Elipse de verdade** | Hoje é polilinha; trocar por `Geometria::Elipse` evita perda no DXF e melhora aparar/estender. |
| **Cotas angular, ordenada, contínua, em linha de base** | Reaproveitar a base da cota linear. |
| **PEDIT, MATCHPROP, QSELECT** | Editar polilinha (juntar, afinar, bulge), copiar propriedades, selecionar por propriedade. Tudo comando do motor. |
| **Estender e deslocar com polilinha de arco** | Estender e Deslocar já existem (`estender.rs`, `deslocar.rs`) mas só tratam polilinha de trechos retos; o mesmo vale para aparar. Completar o bulge nos três. |
| **Vários desenhos abertos** (fila 4) | `Ponte` com N documentos; aba por desenho. |
| **Desfazer/refazer** | Já existe por passos; conferir que a seleção grande desfaz em um passo só. |

## 4. Fase 2 — DXF: ler tudo e não perder nada (DWG depois)

Decisão do usuário (07/10): **ler todos os DXF**; DWG fica para ver depois (o `acadrust` é ideia guardada, não compromisso).

Estado: **lê** LINE, LWPOLYLINE, POLYLINE/VERTEX 2D, CIRCLE, ARC, POINT, TEXT, MTEXT, HATCH, INSERT/MINSERT/BLOCK, ATTRIB, ELLIPSE, SPLINE,
SOLID/TRACE, 3DFACE, DIMENSION (pelo bloco `*D`), MULTILEADER, ACAD_TABLE. **Grava só R12** (LINE, ARC, CIRCLE, POINT, TEXT, POLYLINE, SOLID, INSERT, BLOCK).
Tudo o mais cai em `ignorar()` (`leitor.rs`) e vira um aviso "N objetos de fora".

**A ideia que resolve de vez: preservar o que não sabemos mostrar.** Hoje o desconhecido é jogado fora; passa a ser guardado como
`Geometria::Bruto { tipo, tags }` (os códigos de grupo crus, com camada e cor), regravado igual no DXF e guardado no `.fcdz`.
O aviso muda de "perdido" para "mantido, não exibido". Assim abrir e salvar um arquivo de outro programa **nunca destrói nada**, mesmo
sem entender o objeto. É o que faz "ler todos os DXF" ser verdade.

| Ainda cai fora hoje | Plano |
|---|---|
| LEADER (antigo), XLINE/RAY, WIPEOUT, MLINE, TOLERANCE, SHAPE | Entidades próprias, na ordem de uso: LEADER, XLINE/RAY, WIPEOUT (máscara), MLINE |
| IMAGE/UNDERLAY (PDF, DWF, DGN) | Referência por caminho + moldura; PDF vetorial depende do FeatherPDF |
| VIEWPORT (espaço do papel) | Junto com a Fase 3 |
| POLYLINE 3D/malha, LINE com Z, REGION, 3DSOLID, MESH, SURFACE | Z preservado em `Bruto`; exibir a projeção 2D como cinza |
| ATTDEF/ATTRIB invisíveis | Atributos de verdade (`ATTDEF`, `ATTEDIT`), hoje só texto solto |
| XDATA e dicionários (OBJECTS: GROUP, LAYOUT, MLEADERSTYLE, DIMSTYLE, STYLE, LTYPE complexo) | Ler os que têm efeito visível (DIMSTYLE, LTYPE com texto/forma, STYLE/fonte); o resto em `Bruto` |
| DXF binário, `$DWGCODEPAGE` | Já na fila 3 |

**Teste de aceite:** o corpus de 46 amostras do ezdxf e de terceiros abre, **regrava e relê com o mesmo número de objetos** (hoje só
conferimos que "abre sem pânico"), e o `ezdxf.audit()` não acusa erro no que gravamos. Depois **gravador R2000+** (handles, `BLOCK_RECORD`,
`LAYOUT`) com o aviso do que a versão antiga perde.

**DWG** (adiado): quando voltar ao assunto, ver `acadrust` (pureza em Rust, licença, cobertura) como crate à parte `feathercad-dwg`. Não entra antes
de o DXF fechar.

## 5. Fase 3 — Espaço do papel (pedido do usuário de 29/09, anotado em memória e agora aqui)

Pedido original (29/09): "todo projeto tem que estar numa **folha** (moldura, carimbo, escala)" e o app deve ter **layout/espaço do papel como o
AutoCAD**, com exportar PDF e imagem. Isto estava só na memória; vira item de plano.

Ordem: (1) `quadro` desenha a viewport (recorta o modelo pela moldura e aplica `escala_modelo`/`centro_modelo`), (2) `.fcdz` guarda layouts e viewports,
(3) PDF exporta um layout por página, (4) viewport editável (mover, redimensionar, escala travada, camadas congeladas por viewport), (5) MSPACE/PSPACE,
(6) carimbo e campos ligados ao layout (a "folha" atual vira o bloco do layout), (7) DXF grava `LAYOUT` + `VIEWPORT` (depende do R2000+), (8) PNG/SVG.
A "folha" de hoje (bloco `FOLHA-n` no modelo) continua funcionando até lá, e depois migra para um layout.

## 6. Fase 4 — MCP e engenharia: **com calma** (decisão do usuário, 07/10)

É o nosso diferencial e não é para correr. Nada novo aqui até as fases 0–3 andarem. Quando voltar: um item por vez, conferindo o
resultado ao vivo no app antes do próximo, como foi combinado no modelo `ferrovia`. A lista de componentes que faltam (seção 8) é
inventário, não fila.

## 7. Ordem e esforço

| # | Fase | Esforço (sessões) | Observação |
|---|---|---|---|
| 0 | Seleção: mapa de ids + R-tree + benchmark | 1–2 | Primeiro: é a dor presente. |
| 1a | Pegas, polar/rastreio, chanfro/quebrar/unir/explodir | 3–4 | Fecha o 2D diário. |
| 1b | Spline, MTEXT, elipse real, cotas restantes, PEDIT | 3–4 | Já alinhado com "últimas entidades". |
| 2 | DXF: ler tudo (`Bruto`), ida e volta no corpus, gravador R2000+ | 4–5 | DWG adiado. |
| 3 | Espaço do papel: viewport no quadro, `.fcdz`, PDF, edição | 3–4 | Esqueleto já existe. |
| 4 | MCP e engenharia | devagar | Só depois das fases 0–3. |

Cada fase fecha com `cargo test` + `cargo clippy -D warnings` limpos e o número do benchmark anotado no `CLAUDE.md`.

## 8. Componentes que faltam (inventário; não é fila)

**Já existem:** elétrica (`biblioteca/eletrica`): disjuntor modular 1–4P, DR 2P/4P, DPS, trilho DIN (100/200/300), barras de terra e neutro,
pentes 4/8/12, caixa de quadro (340×340, 460×400), transformador (lateral e frontal). Símbolos NBR 5444 no MCP: tomadas (baixa/média/alta/piso),
pontos de luz (teto, embutido, vapor de mercúrio), interruptores (simples, duas seções, paralelo, intermediário), luminárias (3 células, LED),
fotocélula, arandela, quadros (geral/parcial, aparente/embutido), caixas (medidor, passagem de piso, telefone), nó de ligação, emergência.
Arquitetura: pia, porta 80, vaso, lavatório, tanque, fogão, geladeira. Ferrovia: trilho TR-57, dormente de madeira e de concreto.

**Faltam, por área:**
- **Elétrica (já anotado no `CLAUDE.md`):** unifilares de fusível, seccionadora, TC, TP, pára-raios; cubículo de média tensão; QGBT; atributos (tag, kVA) nos
  componentes; lista de materiais automática. Do que se vê na NBR 5444 e não está: campainha, ventilador/exaustor, ar-condicionado, TV/antena, interfone,
  tomada de dados/telefone, eletrocalha e curvas de eletroduto, haste de aterramento e caixa de inspeção, captor de SPDA.
- **Arquitetura (só há 7 peças):** janelas (correr, maxim-ar), portas de 60/70/90/duplas/correr, cama, sofá, mesa, cadeira, armário, chuveiro/box, bidê,
  escada, pilar, viga, elevador, vagas de garagem, norte, cotas de nível, símbolos de corte/fachada.
- **Ferrovia:** aparelho de mudança de via (AMV), placa de apoio e fixação, seção de lastro, passagem em nível, sinalização, poste de catenária.
- **Viário/civil:** placas, bueiro, meio-fio, guia, poço de visita, poste, bueiro de linha.
- **Carimbo e folha como componente** (hoje o carimbo é o bloco `FOLHA-n` gerado no código).

Ordem sugerida quando a Fase 4 voltar: carimbo como componente e atributos (destrava lista de materiais) → unifilares → arquitetura → o resto.

## 9. Ferramentas de edição que faltam (AutoCAD tem, nós ainda não)

Já temos: apagar, mover, copiar, girar, escala, espelhar, esticar, matriz (retangular), aparar, estender, concordância (fillet), deslocar (offset),
hachura, bloco, grupo, igualar propriedades/camada, medir, contar iguais (MCP), cotas (linear, alinhada, raio, diâmetro), chamada, tabela.

| Grupo | Faltam |
|---|---|
| **Modificar** | chanfro, quebrar, unir, explodir, alongar (LENGTHEN), alinhar (ALIGN), PEDIT (juntar/afinar/bulge), matriz polar e por trajeto, overkill, dividir/medir (DIVIDE/MEASURE), escala de texto, ajustar texto, ordem de desenho (trazer/enviar), alterar espaço |
| **Desenhar** | polígono, spline, MTEXT (entrada), XLINE/RAY, donut, MLINE, contorno (BOUNDARY), região, elipse real, hachura com gradiente, cobertura (WIPEOUT), nuvem de revisão |
| **Anotar** | cota angular, ordenada, contínua, linha de base, estilo de cota, estilos de texto e de chamada, campos, quebra de cota |
| **Blocos** | editor de bloco, atributos (ATTDEF/ATTEDIT), blocos dinâmicos, XREF, WBLOCK |
| **Camadas** | isolar/desisolar, apagar/mesclar, estados de camada, filtros por propriedade, "andar" pelas camadas |
| **Seleção** | QSELECT, selecionar semelhantes, filtro, isolar objetos, ciclar sobreposição |
| **Arquivo** | PURGE, AUDIT/RECOVER, autosave e recuperação, plot preview, publicar vários layouts em um PDF, importar PDF |
| **Consulta** | lista de propriedades (LIST), área/perímetro de vários objetos, MASSPROP, localizar e substituir texto, corretor ortográfico |

## 10. Ferramentas novas — o que nem o AutoCAD nem o CADCraft têm (ideias)

Cada uma vem de algo que já temos e o AutoCAD não tem como ter. **Ressalva:** a lista de comandos do AutoCAD 2026 não foi reconferida nesta
sessão; "não tem" aqui é pelo que conheço, e o que for encontrado já existindo lá cai da lista. Em ordem de valor para o custo:

1. **Verificação viva** (como o corretor ortográfico do Word). O `cad_verificar` do MCP roda hoje sob pedido; no app passa a rodar em
   segundo plano e marcar na tela: texto sobre texto, linha duplicada, contorno quase fechado (falha de 0,01), objeto na camada errada, eletroduto sem circuito.
   Já achamos 11 textos sobre texto nas plantas da ferrovia; uma ferramenta que mostra isso na hora vale ouro. O AutoCAD só tem o CAD Standards manual.
2. **Seleção por expressão.** `camada = "EL-TOMADA" e comprimento > 3 e dentro(folha 2)` numa caixinha, em Rust, igual ao motor de expressões do
   `cad_lote`. O QSELECT do AutoCAD é um diálogo com poucos operadores. Seleção salva com nome vira um "filtro vivo" reutilizável.
3. **Histórico de projeto com ramos (Git do `.fcdz`).** O formato já prevê Git embutido. Dá para voltar a qualquer versão, testar uma alternativa num
   ramo e **mesclar** (merge de três vias) o trabalho de duas pessoas **sem servidor**, fiel à regra de privacidade. O AutoCAD compara dois DWG (2D), mas não tem ramo nem merge.
4. **Nuvem de revisão automática.** Compara a versão atual com a revisão anterior (do histórico acima) e desenha sozinho a nuvem em volta do que
   mudou e preenche o quadro de revisões. No AutoCAD a nuvem é manual.
5. **Quantitativos vivos e tabela ligada ao FeatherSheet.** Tabela no desenho que conta blocos, soma comprimentos por camada e se atualiza quando o
   desenho muda; e a opção de a tabela ser uma planilha `.fshz` da suíte (fórmulas, OpenQuery), em vez do assistente de extração de dados do AutoCAD, que é manual e exporta uma vez.
6. **Rótulos que se desviam sozinhos.** Texto de cota, estaca e curva ganha "colisão": se bater em outro, o motor o desloca para o espaço livre mais
   perto e põe uma linha-guia. Nenhum CAD faz isso por padrão, e a plantas A1 da ferrovia pedem.
7. **Desfazer seletivo.** Desfazer só as mudanças dos objetos selecionados (ou de uma camada), sem desfazer o resto. O histórico em passos
   (`Mudanca`) já guarda quais objetos cada passo tocou. O AutoCAD desfaz só em ordem cronológica.
8. **Cotas ligadas ao significado.** Em vez de só medir, a cota sabe o que mede (parede, vão, distância mínima de norma) e avisa quando uma edição quebra a regra,
   usando a verificação viva (1). É o passo natural depois das cotas paramétricas.
9. **Macro da suíte.** Os mesmos módulos de macro do editor Alt+F11 do FeatherSheet no CAD, em vez de AutoLISP. Uma linguagem para os cinco apps.

**Escolhas do usuário (07/10):**
- **1 Verificação viva:** boa, mas **entra primeiro no FeatherDocs** e depois nos outros apps (registrado no `README.md` da raiz). No CAD, o `cad_verificar` do MCP segue como está até lá.
- **2 Seleção por expressão:** boa. Ainda **não existe** "selecionar tudo de uma camada" no CAD (só há igualar camada, tornar atual e contar por camada); isso vira o caso mais simples da expressão (`camada = "X"`) e entra junto com o QSELECT.
- **3 Histórico com ramos:** muito bom. **4 Nuvem de revisão automática:** boa (depende do 3).
- **6, 7, 8, 9:** explicados em conversa; **8 "pode ser"** (baixa prioridade). **9 Macros:** o mesmo editor e o mesmo motor de macros do FeatherSheet, o que pede um pacote comum (regra 3), porque hoje ele mora dentro do `feathersheet/`.

Fora da fila: ver a lista como menu para o usuário escolher; nenhuma delas começa antes das fases 0–1.

## 11. Riscos e dúvidas em aberto

- **"10×" é meta de desempenho e de fluxo de trabalho, não de contagem de comandos.** Se a ideia for 10× em quantidade de comandos, isso vira ~2 900 e não é realista; vale confirmar.
- **DWG**: adiado por decisão do usuário (07/10). `acadrust` fica anotado, sem compromisso.
- **O que fica no `.fct` e o que vira JSON**: já decidido no MCP; só manter.
- Código do CADCraft é MIT/Apache: pode ser estudado e reaproveitado com atribuição (`ATTRIBUTION.md`), mas pela regra 3 não entra por caminho relativo; se algo for copiado, vira crate nosso.
- Nada deste plano está commitado ainda; o FeatherCAD tem muita coisa não commitada (ver memória de 29/09 a 04/10). Commitar por partes antes de mexer em `documento.rs`.
