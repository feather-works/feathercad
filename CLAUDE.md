# FeatherCAD — instruções de trabalho

CAD da suíte (sétimo aplicativo). **Estado (29/09/2026): esqueleto funcionando** —
motor, ponte, `src-tauri` e uma casca de tela com a aba Início e a ferramenta
Linha de ponta a ponta, fundo (nada/pontos/grade), zoom, página inicial e abrir/salvar
DXF e `.fcdz`. Regras da suíte: `../CLAUDE.md`. Receita de app
novo, que este segue inteira: `../RECEITA.md`.

## Regra de ouro (dita pelo usuário, 29/09/2026)

**O TypeScript é frontend: só desenha, não calcula nada.** Tudo que é conta,
geometria, snap, medida, formatação de número/coordenada, interpretação de
comando e estado do documento é **Rust**. Se uma função TS tem um `if` sobre o
conteúdo do desenho, está no lado errado. Estado de interface (zoom, pan,
ferramenta ativa, foco, paleta aberta) fica no TS.

## Onde está cada coisa

- `README.md` — o que é, formatos (`.fcdz`, `.dxf`), versões de DXF.
- `docs/INTERFACE.md` — composição da tela (referência AutoCAD LT 2026),
  Ribbon com várias sub-abas, quem é dono de quê, ordem sugerida.
- `docs/DESDE-O-DIA-1.md` — o que o FeatherSheet fez tarde e aqui nasce
- `docs/ATALHOS.md` — as teclas e os apelidos do AutoCAD, e onde cada um mora.
- `docs/FUNCIONALIDADES.md` — o que do AutoCAD 2D entra, em que fase, e o que não entra.
  certo: língua em JSON, motor só Rust, lints ligados, fonte única da Ribbon.
- `docs/DXF-REFERENCIA.md` — estrutura do DXF (ezdxf).
- `testes/amostras/` (ezdxf, ver `LEIA-ME.md`) e `testes/amostras-terceiros/`
  — corpus para o teste de ida e volta.

## Comandos (de dentro de `feathercad/`)

```bash
npm run app:dev        # o app com janela (Tauri); usa a porta 5174
npm run dev            # só o navegador (o motor roda como .wasm)
npm test               # testes TS contra o motor Rust em wasm
npm run portoes        # tsc + npm test + cargo test/clippy nos 2 crates + build
cd src-tauri && cargo clippy
```

## Arquitetura (o que existe)

- `crates/feathercad-core` — motor: `documento` (camadas, entidades, desfazer),
  `cursor` (snap/orto/grade), `vista` (zoom/pan), `sessao` (ferramenta, linha
  de comando, interruptores), `atalhos` (teclas do AutoCAD), `comandos`
  (apelidos por língua), `formato` (número na língua), `quadro` (binário).
- `crates/feathercad-io` — `dxf::leitor` (só `LINE` e camadas por ora; o resto vira
  aviso), `dxf::escritor` (**R12/AC1009**), `fcdz` (ZIP + JSON), `aci`, `inicio`
  (recentes, pastas de projeto, listagem — tudo local). Testado contra as 46
  amostras de `testes/` (todas abrem; o que não é `LINE` é avisado).
- `crates/feathercad-ponte` — `Ponte::chamar` (JSON) + `wasm.rs` para os testes.
  `COMANDOS_DA_PONTE` lista o que atende; um teste exige que o `Motor` do TS
  chame todos. Comando novo na ponte: acrescentar também em `COMANDOS_DA_PONTE_ESPERADOS`
  (`src/ponte/index.ts`; um teste trava as duas listas). Na largada a interface pergunta
  `comandos_da_ponte` ao motor e, se faltar algum, mostra "motor desatualizado" (fim dos erros soltos).
- `src-tauri` — janela + protocolo síncrono `ponte://` (XHR síncrono, como o
  `pasta://` do FeatherSheet). **Sem socket:** motor no mesmo processo.
- `src/ponte` — `Motor` (chamadas tipadas), `canal.ts`, `metricas.ts`
  (`[motor]`/`[ação]`/`[quadro]`). `src/idiomas` — `<código>.json` (textos) +
  `comandos.<código>.json` (apelidos digitados, vão para o Rust). `src/ui` —
  um componente por arquivo com o seu CSS; `ribbon-dados.ts` é a Ribbon como
  dado; `z.ts` é a escala de `z-index`.
- **Monitor (F12)**, copiado do FeatherSheet em 29/09/2026 a pedido do usuário:
  página `monitor.html` + `src/monitor/` (abas Memória, Console, Métricas), janela
  própria pelo comando `abrir_janela_monitor`; captura do console em
  `src/ui/console-depuracao.ts` e conversa entre janelas por `BroadcastChannel`
  (`monitor-ponte.ts`); Rust: `src-tauri/src/metricas.rs` (`[motor]`) e
  `memoria_arvore.rs`. Textos em `monitor.*` dos JSON de língua. F12 é do Monitor,
  não da entrada dinâmica do AutoCAD (`docs/ATALHOS.md`).
- **Snap ao objeto** (29/09/2026, não commitado): `crates/feathercad-core/src/snap.rs` — 9 tipos configuráveis
  (extremidade, meio, centro, quadrante, interseção, perpendicular, tangente, nó, mais próximo); cada entidade vira
  primitivas (segmento/arco, bulge exato) e ganha o candidato mais perto do cursor (o "mais próximo" só sem outro).
  Perpendicular/tangente usam o ponto anterior. Ponte: `osnap_modos_definir {modos:{nome:bool}}`, estado `osnap_modos`,
  cursor `snap` (tipo da marca). Interface: seta ▾ ao lado de OSNAP (ou botão direito) abre a lista; escolha guardada
  em `localStorage` (`snapModos`); marca desenhada por tipo em `area-desenho.ts`. De fábrica ligados: extremidade,
  interseção, meio, centro, quadrante e nó. **Não entram (ainda):** snap em geometria dentro de blocos, extensão e paralelo.
- **Arquivos:** o disco e as caixas de diálogo são do `src-tauri/src/arquivos.rs`
  (abrir, salvar, recentes, projetos); o documento mora na ponte
  (`documento_abrir` recebe os bytes crus, `documento_gravar` devolve os bytes).
  `Ctrl+N/O/S/Shift+S` chegam ao motor, que responde `janela` (a interface executa).
  No navegador o mesmo fluxo roda com `<input type=file>`/download.
- **Página inicial** (aba Início, estilo AutoCAD): Abrir, Novo, Recentes e Projetos.
  Projeto = pasta do disco (sem nuvem, regra 5). Datas formatadas pelo `Intl` do
  navegador (o Rust sem dependência não sabe o fuso) — única conta de apresentação no TS.
- **Unidade do desenho** (mm, cm, m — 29/09/2026): todo Novo pergunta (`caixa-unidade.ts`,
  já marcada a última usada; Esc desiste). Mora no `Documento` (`Unidade`), vai no `.fcdz`
  (`unidade`) e no DXF como `$INSUNITS` (4/5/6; o R12 não define a variável, mas quem a
  conhece a lê). Arquivo sem unidade abre com `unidade: null`; o rodapé mostra
  `MODELO · mm` e Configurações → Desenho troca (conta como alteração, sem desfazer). Só
  metadado: nenhum objeto é redimensionado. Polegada/pé ainda não.
- **Distância digitada** (29/09/2026, `sessao.rs::ler_ponto`): com uma ferramenta que tem ponto
  anterior (linha, polilinha, mover/copiar, retângulo…), um número solto vira ponto **a essa
  distância, na direção do cursor** (já com orto/snap, guardado pela ponte em `ultimo_cursor`);
  no círculo, com o centro marcado, é o raio. O número vale na unidade do desenho; aceita
  sufixo (`10cm`, `2,5m`, `300mm`) e converte. Sem direção (cursor parado na base) a linha de
  comando avisa (`comando.semDirecao`).
- **Tamanho da cota**: Configurações → Desenho; `cota_tamanho_definir` (Rust, `Sessao::altura_cota`,
  `None` = o do texto). A preferência fica no `localStorage` (`feathercad.cota`, na unidade do
  desenho) e é reenviada ao motor na largada — não vai dentro do arquivo.
- **Grab** (29/09/2026): sem comando ativo, apertar o mouse sobre um objeto o "pega"; arrastar
  (≥ 4 px) move a seleção inteira (prévia ao vivo, com orto/snap; um Ctrl+Z desfaz), soltar sem
  andar é um clique de seleção. Shift+clique continua sendo seleção. Rust: `Sessao::arrasto_*`
  (`edicao_sessao.rs`); ponte: `arrasto_iniciar|fim|cancelar`; TS só repassa (`area-desenho.ts`).
  **Ao vivo:** durante o arrasto o `quadro` esconde os selecionados (versão com o bit 62) e
  `quadro_arrasto` (`quadro::montar_deslocado`, raiz `Transf::mover`) devolve só eles, já
  deslocados, com blocos, texto e hachura; o TS pinta isso no canvas de cima a cada movimento.
- **Seleção por arrasto** (29/09/2026): apertar o mouse no vazio abre a janela (1º canto); se o mouse anda
  ≥ 4 px até soltar, soltar fecha a janela; se não anda, segue valendo o 2º clique. **Esquerda→direita =
  janela, azul, cheia: só pega o que cabe inteiro; direita→esquerda = cruzando, verde, tracejada: basta
  tocar** (Esticar é sempre cruzando). Quem decide é o Rust (`tipo_da_janela` → `RespostaCursor.janela`,
  `edicao::na_janela`; círculo/arco só pegam se o **traço** toca); o TS só pinta (`--janela-dentro/-toca`).
- **Ângulo do Orto** (29/09/2026): o Orto trava em múltiplos de um passo (90° = só eixos; 60, 45, 30, 15 ou
  qualquer 0–180). Rust: `Sessao::angulo_orto` → `Ajudas.angulo` (`cursor.rs::orto_e_grade`, o ponto é a
  projeção do mouse na direção travada); ponte `orto_angulo_definir` + `Estado.orto_angulo`. TS: lista ao lado
  do botão Orto no rodapé e linha em Configurações → Desenho (passos prontos + campo livre); a escolha fica em
  `feathercad.ortoAngulo` e é reenviada na largada. Vale para linha, mover, arrasto etc. (tudo que usa o cursor).
- **Ângulo na linha de comando** (29/09/2026, como no AutoCAD; `sessao.rs::ler_ponto` e `digitar`): polar
  `@10<45` (a partir do ponto anterior) e `10<45` (a partir da origem); ângulo em graus, anti-horário do
  eixo X, distância com unidade opcional (`@50cm<30`, decimal na língua: `@2,5<30`). `<45` sozinho, com um
  ponto anterior, **trava a direção** (`Sessao::angulo_travado` → `Ajudas.direcao`: o cursor passa a andar
  só nessa reta) e a próxima distância digitada sai nela; o lado (frente/trás) é o que o cursor indica.
  A trava some no próximo ponto, ao cancelar ou ao trocar de comando.
- **Servidor MCP** (29/09/2026, `crates/feathercad-mcp`, binário `feathercad-mcp`; `npm run mcp:build`): deixa uma IA (Claude
  Code etc.) **ver, medir e desenhar**, por stdio, 100% local. Usa o mesmo motor (`feathercad-core`/`-io`), não a ponte.
  **Noção de espaço:** `cad_ver` devolve um PNG com grade numerada, eixos, escala e o `#id` de cada entidade (o traço vem do
  `quadro` do motor: blocos, hachuras, textos); `cad_entidades` (medidas exatas + filtro por janela/proximidade), `cad_medir`,
  `cad_regioes` (áreas fechadas por traços soltos, com área útil e textos dentro), `cad_verificar` (folgas, duplicadas, comprimento
  zero). **Economia de tokens:** o **formato compacto `.fct`** (`compacto.rs`, uma linha por objeto: `L x1 y1 x2 y2`, `R`, `C`, `A`,
  `P` com `~bulge`, `T`, `H`) é lido/escrito por `cad_exportar`/`cad_importar` e por `cad_abrir`/`cad_salvar`/`cad_converter`
  (DXF/`.fcdz` ⇄ `.fct` no disco, sem passar pela conversa; devolve os tamanhos antes e depois). Criar/editar valida tudo antes e
  vira **um** passo de desfazer. Só grava `.dxf/.fcdz/.fct` e não sobrescreve sem `sobrescrever: true`. Registrar no Claude Code:
  `claude mcp add feathercad -- <caminho>/feathercad/crates/feathercad-mcp/target/release/feathercad-mcp`.
- **MCP: folha, PDF, contagem e hachura (29-30/09/2026, `crates/feathercad-mcp/src/folhas.rs`, feito, testado, não commitado):**
  `cad_folha` (`listar`, `inserir` com papel/escala/campos/origem, `escala`, `ajustar`, `campos`, `campos_definir` — o mesmo
  `Documento::folha_*` do botão do app), `cad_pdf` (PDF vetorial por folha, papel A4…16A0/FOLHA, cores, páginas escolhidas),
  `cad_contar` (`igual_a` um id, ou `tipo`+`medida`+`tolerancia`), `cad_hachurar` com `padrao` e `escala` (biblioteca de
  `padroes.rs`), forma `hachura` e `raio` de cantos em `cad_criar`, e `cad_editar` com `operacao: texto`. **Depois de
  `npm run mcp:build` é preciso reconectar o MCP no Claude Code (`/mcp`)**: o processo antigo continua com o binário velho.
  O servidor MCP tem o **desenho dele na memória, separado do app**: o app só vê o que for gravado no arquivo que ele tem
  aberto (recarrega em ~1 s se não há alteração pendente). Ligar o MCP ao motor do app ao vivo ainda não existe.
- **Botão direito = menu** (`menu-contexto.ts`; a lista é do motor, `menu_contexto.rs`): comando em
  andamento → Enter/Cancelar/Fechar + interruptores; objetos selecionados → Mover, Copiar, Girar…
  Apagar, Propriedades; nada → Repetir último comando, Colar, Desfazer/Refazer, Zoom, interruptores,
  Configurações. O objeto sob o mouse vira a seleção. **O botão direito não é mais Enter** (o Enter é o
  1º item do menu quando há comando; a tecla Enter continua igual). Item novo: rótulo em `menu.item.<id>`
  nos JSON de língua (um teste cobra todo comando do motor).
- **Configurações** (`configuracoes.ts`, Ctrl+, ou ⚙ na barra de título): modal do
  FeatherSheet; apelidos do usuário em `feathercad.apelidos` vão junto à tabela de comandos
  mandada ao Rust. Atalhos de tecla regraváveis: falta (exige tabela editável no Rust).
- **Tema** (igual ao FeatherSheet): automático/claro/escuro, guardado em `office.theme`
  (chave da **suíte**, para os apps acompanharem), aplicado como `data-theme` no `<html>`
  (`src/ui/tema.ts`). Componente **nenhum** escreve cor solta: só as variáveis de
  `base.ts` (teste trava; exceções: console copiado e toast de erro). Quem desenha em
  canvas relê as variáveis quando o tema muda. O "branco" do CAD (ACI 7) vira preto no
  fundo claro — conta do Rust (`quadro::cor_na_tela`). Falta o que o FeatherSheet tem
  além disso: esquemas de cores e cor de destaque à escolha.
- Botão e atalho usam o **nome interno** do comando (`executar`); o texto
  digitado só vale pela tabela da língua (no en-US `LINHA` não existe).

## Decisões

- **Língua em JSON desde o primeiro texto** (`src/idiomas/<código>.json`,
  como no FeatherSheet), inclusive nome/alias de comando por língua; ver
  `docs/DESDE-O-DIA-1.md`. Nenhum texto de interface no código.
- Sem conta, login, nuvem ou loja (regra 5 da suíte). A aba "Colaborar" do
  AutoCAD, se existir, só faz coisa local (comparar, exportar).
- Ribbon com **sub-abas como dados**: aba nova entra sem tocar nas outras.
  Comando novo entra na Ribbon **e** no menu, a partir da mesma fonte.
- Primeira sub-aba: **Início** (Home). As demais são descritas e confirmadas
  uma por vez antes de codar.

## ✅ Polilinha e texto (29/09/2026) — feito, testado, **não commitado**

Motivo: os avisos ao abrir `VP4.dxf`. Agora entram **LWPOLYLINE, POLYLINE 2D do R12 (com arcos por bulge) e TEXT**
(leitura, gravação R12 como POLYLINE+VERTEX+SEQEND e TEXT, `.fcdz`, desenho na tela, snap nos vértices), e há a
**ferramenta Polilinha** (`PL`/`PLINHA` no pt-BR, `PL`/`PLINE` no en-US; Enter termina, opção Fechar
(`F`/`C`) fecha, Esc mantém o já marcado). Continuam de fora, de propósito: espaço do papel (layouts, fase 2),
MTEXT, POLYLINE 3D/malha, e **a ferramenta de criar texto** (pede altura + conteúdo em passos). Ver `crates/`
e os testes (`cargo test` em core/io, `npm test`). A tabela de língua agora é `{ "comandos", "opcoes" }`.

**Lembrete recorrente:** o `npm run app:dev` **não recompila o Rust quando só mudam os crates em `crates/`**.
Se aparecer "comando desconhecido na tabela de língua" ou campo `undefined`, é o motor antigo: fechar e abrir
o `app:dev` (ou `touch src-tauri/src/lib.rs`). A interface agora mostra uma mensagem clara nesse caso.

## ⚠ EM ANDAMENTO (29/09/2026): blocos (INSERT) — RETOMAR DAQUI

**Estado da árvore (verde: `npm run portoes` passa — 49 testes TS, core 28, io 23; nada commitado).**
Já feito e testado nesta sessão, além do que está nas seções acima: círculo e arco (ferramentas `C`, `A`),
polilinha (`PL`), texto (só ler/mostrar/gravar), fundo, zoom, tema, página inicial, DXF R12 + `.fcdz`.
Também: as caixas de abrir/salvar/escolher pasta abrem em **`Documentos/FeatherCAD`** (criada se faltar; na
1ª vez já aparece em Projetos) — `src-tauri/src/arquivos.rs::pasta_padrao/pasta_inicial`, **não testado no app**.

**Blocos — o que JÁ está pronto (core, 28 testes verdes):**
`Geometria::Insercao{bloco:usize, ponto, escala_x, escala_y, rotacao}`; `Bloco{nome, base, entidades:
Vec<NovaEntidade>}`; `Documento::de_partes_com_blocos(camadas, atual, blocos, entidades)`, `blocos()`,
`bloco_por_nome()`, `adicionar_insercao()`, `caixa_guardada_do_bloco()` (caixa de cada bloco calculada ao
montar; `PROFUNDIDADE_MAXIMA = 8`); `geom::Transf` (matriz afim: `de_insercao`, `depois_de`, `aplicar`);
`quadro.rs` desenha recursivo (`Contexto/Nivel/Peca`; recorte pela caixa do bloco transformada; **camada `0`
dentro do bloco herda a cor do INSERT**); snap no ponto de inserção; ferramenta `inserir` (`Ferramenta::Insercao`:
digita o nome do bloco, depois clica o ponto); comando `inserir` em `CONHECIDOS`. TS: apelidos
`INSERIR/I` (en `INSERT/I`), textos `insercao.*`, painel "Bloco" na Ribbon (⧉).

**FEITO em 29/09 (a, b, c):** leitor de BLOCK/INSERT/DIMENSION (`leitor.rs`: `Leitura`, `numerar_blocos`, aviso
`aviso.dentroDeBlocos`), escritor da seção BLOCKS + INSERT, `.fcdz` com `blocos`, testes em `tests/arquivos.rs`
("blocos (INSERT)"). Não há biblioteca de blocos: só existem os do arquivo aberto (o comando INSERIR pede o nome;
a lista para escolher entrou: `Estado.blocos` enquanto o Inserir espera o nome, botões na linha de comando). `ATTRIB` visível vira `TEXT` solto
(sem ligação com o bloco); `ATTDEF` e `ATTRIB` invisível são pulados sem aviso. **MTEXT e HATCH (e) FEITOS em 29/09:** `MTEXT` vira um `Texto` por linha (`limpar_mtext`, ancoragem `71`,
entrelinha 5/3 da altura; sem quebra automática pela largura `41`). `HATCH` = `Geometria::Hachura{contornos, padrao,
nome}` (contorno de polilinha ou arestas linha/arco/elipse/spline-pelo-polígono-de-controle; `padrao` vazio = sólida).
O quadro ganhou a seção de **preenchimentos** (`FCQ1`: contador após os textos, par-ímpar) — sólida vira área cheia,
de padrão vira linhas recortadas pelo contorno (linha contínua, tracejado não; se as linhas ficam a < 3 px, vira
preenchimento). O deslocamento do padrão no DXF está em coordenadas do **mundo**. **Limites:** o DXF R12 gravado hoje
**não tem HATCH** — avisa `aviso.hachuraNaoGravada` e a hachura só sobrevive no `.fcdz`; gradiente vira sólida da cor
da entidade; ilha/detecção de contorno pelo desenho e criar hachura (comando) não existem. `serde_json` ganhou
`float_roundtrip` (o `.fcdz` perdia a última casa dos decimais). **Bateria (29/09):** `corpus_de_amostras_abre_sem_panico` exige que **todos os 46 DXF** de `testes/` abram, gravem em
DXF e `.fcdz` e reabram; relatório por arquivo: `cargo test --manifest-path crates/feathercad-io/Cargo.toml bateria
-- --nocapture --ignored`. Lidos desde 29/09 (tarde): ELLIPSE e SPLINE (viram `Polilinha` amostrada; spline = B-spline racional, de Boor),
SOLID/TRACE (viram `Hachura` sólida, cantos na ordem 1-2-4-3), 3DFACE (contorno fechado, Z some), POINT
(`Geometria::Ponto`, cruz de 3 px), MINSERT (matriz expandida, máx. 5000), ACAD_TABLE / ARC_DIMENSION /
LARGE_RADIAL_DIMENSION (INSERT do bloco anônimo). MULTILEADER (linhas até o pouso + dogleg + texto; sem ponta de seta). Ainda de fora (aviso): WIPEOUT,
RAY/XLINE (linhas infinitas), 3DSOLID, MESH. **DXF R12 gravado:** elipse/spline saem como POLYLINE; hachura de padrão
sai como `LINE`s (traço contínuo) e sólida sem ilha como triângulos `SOLID` (`geom::triangular`, corte de orelhas) —
avisa `aviso.hachuraDecomposta`; sólida **com ilha** (ou padrão com > 20 mil traços) não sai: `aviso.hachuraNaoGravada`
(falta triangular com furo). O `.fcdz` guarda a hachura de verdade. Não há `.dwg` (formato fechado).
**Falta d) reiniciar o app.** O texto abaixo é o plano original.

**Falta (nesta ordem):**
a) **io leitor** (`crates/feathercad-io/src/dxf/leitor.rs`, hoje só LINE/LWPOLYLINE/POLYLINE/TEXT/CIRCLE/ARC):
   - 1ª passada: percorrer os objetos e numerar cada `BLOCK` da seção BLOCKS (tag 2 = nome, 10/20 = base),
     **exceto** `*Model_Space`, `*Paper_Space*`, `$MODEL_SPACE`, `$PAPER_SPACE`; mapa nome(minúsc.)→índice
     (nome repetido: o índice segue a contagem, o mapa guarda o 1º). 2ª passada: entidades entre `BLOCK` e
     `ENDBLK` vão para o bloco (mesma função `entidade()`, sem a checagem de espaço do papel) — refatorar
     `entidade()` para receber uma struct `Leitura{camadas, nomes, ignorados, ignorados_de_blocos}` (clippy
     acusa mais de 7 argumentos).
   - `INSERT`: tag 2 nome → `Geometria::Insercao` (10/20 ponto, 41 escala X padrão 1, 42 escala Y padrão 1,
     50 giro); nome inexistente → ignorar "INSERT (bloco inexistente)". `ATTRIB`/MINSERT (70/71) seguem de fora.
   - **`DIMENSION` → INSERT do bloco anônimo `*D..` (tag 2) em (0,0), escala 1** — mostra as cotas de graça.
   - o que for ignorado DENTRO de blocos vira aviso à parte: `aviso.dentroDeBlocos` `{n, tipos}` (novo texto pt/en
     + entrar na lista do teste de idiomas).
   - usar `Documento::de_partes_com_blocos`.
b) **io escritor/fcdz**: `escritor.rs` tem um braço vazio `Geometria::Insercao {..} => {}` marcado TODO — gravar a
   seção `BLOCKS` (`0 SECTION 2 BLOCKS`; por bloco: `0 BLOCK 8 0 2 nome 70 0 10 bx 20 by 30 0 3 nome`,
   entidades, `0 ENDBLK 8 0`; `0 ENDSEC`) **antes** de ENTITIES, e `0 INSERT 8 camada [62] 2 nome 10/20/30 41 sx 42 sy
   43 1 50 giro`. Para isso generalizar `cabeca()`/`entidade()` (hoje recebem `&Entidade`) para receber
   camada+cor+geometria (blocos guardam `NovaEntidade`). `fcdz.rs`: acrescentar `blocos: Vec<{nome, base,
   entidades}>` com `#[serde(default)]` (`GeometriaJson::Insercao` já existe, mas os blocos ainda não são gravados).
c) testes: ida e volta DXF/.fcdz com bloco aninhado; corpus (VP4 e o arquivo do usuário com 190 INSERT —
   o aviso de INSERT deve sumir); DIMENSION; bloco que se insere em si mesmo não trava.
d) rodar `npm run portoes`; atualizar esta seção; **avisar o usuário para reiniciar o `npm run app:dev`**.
e) depois: **MTEXT**, depois HATCH (fase 2). O pedido do usuário que originou tudo: os avisos ao abrir arquivo
   real ("214 objetos de fora: 2 ARC, 16 CIRCLE, 2 HATCH, 190 INSERT, 4 MTEXT").

**Lembrete recorrente:** o `npm run app:dev` **não recompila o Rust quando só mudam os crates em `crates/`**
("comando desconhecido na tabela de língua" / campo `undefined` = motor antigo). Fechar e abrir o `app:dev`
(ou `touch src-tauri/src/lib.rs`). A interface mostra uma mensagem clara nesse caso.

**Perguntas ainda sem resposta do usuário** (`docs/FUNCIONALIDADES.md`, fim): hachura na fase 1? linguagem de
macro (só `.SCR` ou a da suíte)? georreferência só UTM/WGS84? importar PDF vetorial pelo FeatherPDF?

## ✅ Edição da aba Início (29/09/2026, noite) — feito, testado, **não commitado**

Comandos novos (todos no Rust; botões da faixa ligados; apelidos pt/en em `comandos.<língua>.json`):
**apagar, mover, copiar, girar, escala, espelhar, esticar, matriz, aparar, concordância, retângulo, elipse, hachura.**
- **Seleção** (`edicao_sessao.rs`): clique no objeto (a menos de 6 px) seleciona; **Shift** alterna; clique no vazio abre uma
  janela e o 2º clique a fecha — esquerda→direita só o que cabe, direita→esquerda também o que toca; Esc limpa; **Delete** apaga;
  **Enter** ou **botão direito** confirma (o TS manda `digitar('')`). Realce azul no quadro (`quadro::montar_com_selecao`); a
  versão do quadro é `versão do desenho + revisão da seleção × 2³²`.
- **Fluxo** (como o AutoCAD): com objetos já selecionados o comando pula a escolha; senão pede objetos + Enter. Mover/copiar/girar/
  escala: ponto base, depois destino (girar e escala também aceitam número digitado; escala clicada = distância). Copiar repete até
  Enter. Espelhar mantém o original (sem a pergunta de apagar). Esticar exige janela cruzando. Matriz: digita `linhas colunas
  espaço-linha espaço-coluna` (máx. 10 mil cópias). Aparar: cada clique apara o trecho entre os dois cortes vizinhos (as arestas
  são todas as outras formas; linha, arco, círculo, polilinha só de trechos retos). Concordância: só entre 2 linhas; raio digitado
  no prompt (0 = só encontrar). Hachura: clique **dentro** de círculo/polilinha fechada → o menor contorno que contém o ponto, com os
  contornos fechados de dentro como ilhas; `S`/`P` alterna sólida/linhas a 45°. Retângulo = polilinha fechada; elipse = polilinha de 72 lados.
- **Histórico:** uma edição = **um** Ctrl+Z (`Documento::aplicar_mudancas` com `Passo::Lote`; `Mudanca::{Novo, Remover, Trocar}`).
- **Módulos:** `edicao.rs` (transformar, esticar, escolher, janela, matriz, retângulo, elipse), `aparar.rs` (aparar, concordância,
  hachura por ponto), `edicao_sessao.rs` (seleção e fluxo dos comandos), `geom::Transf::{mover,girar,escalar,espelhar,inversa}`.
- **Hachura por linhas soltas** (29/09/2026, `regiao.rs`): clicar dentro de uma área delimitada por linhas/arcos/polilinhas
  soltas (um quadrado de quatro linhas) também hachura: as curvas viram segmentos, são cortadas onde se cruzam ou encostam
  (T inclusive), as pontas soltas caem, e a menor face que contém o ponto vence. Abertura no contorno = sem região (como no
  AutoCAD). Círculo/polilinha fechada continuam tendo prioridade (com bulge exato) quando são o menor contorno. Ilhas só
  entre contornos fechados; blocos e texto não delimitam; teto de 20 mil segmentos.
- **Limites conhecidos:** aparar não trata polilinha com arco nem texto/bloco; sem "estender", "deslocar" (offset) nem pegas (grips); o espelho de bloco
  usa escala Y negativa; a seleção não mostra "N selecionados" na tela (só no histórico). Janela é por 2 cliques (arrastar ainda não).
- **Desempenho:** o desenho no canvas agora agrupa os segmentos por cor (um `stroke` por cor, não por segmento) e o cursor consulta o
  motor 1× por quadro de tela (antes: a cada evento do mouse) — corrigia a lentidão com muitos elementos sem gastar CPU/RAM.

## ✅ Camadas, propriedades e o resto da aba Início (29/09/2026, noite) — feito, testado, **não commitado**

**Camadas (prioridade do usuário: "tem que ficar perfeito")** — `camadas.rs` (motor), `camadas-painel.ts` (janela), `ribbon-widgets.ts`:
- `Camada{nome, cor, ligada, congelada, bloqueada, plotar, tipo_linha, espessura, transparencia}`. Congelada some do desenho e das escolhas;
  bloqueada aparece mas **não se escolhe nem se edita**; transparência mistura a cor com o fundo (`quadro::misturar_com_o_fundo`);
  espessura e tipo de linha da camada valem para o objeto "por camada". Plotar só é guardado (não há impressão).
- O gerenciador (botão "Propriedades de camada") tem o layout do AutoCAD: Status, Nome (duplo clique renomeia), Ativada, Congelar, Bloquear,
  Plotar, Cor (seletor do sistema), Tipo de linha, Espessura, Transparência; árvore de filtros (Todas / Todas as usadas); busca com `*` e `?`;
  clicar no cabeçalho ordena; várias linhas (Ctrl/Shift) recebem a mesma mudança; contagem "N camadas exibidas de M". **Filtrar, buscar, ordenar,
  validar nome e aplicar são do Rust** (`camadas` e `camada_acao` na ponte). Regras: `0` não se renomeia/exclui; a atual não se congela/exclui;
  só se excluem camadas sem objetos (índices se remapeiam; um Ctrl+Z restaura); nome único sem diferenciar maiúscula; sem `< > / \ " : ; ? * | = , \``.
- Histórico: mudança de camada = captura da tabela (`Passo::Camadas`); excluir = captura de tudo (`Passo::Tudo`). Um Ctrl+Z por ação.
- Faixa: o painel Camadas ganhou a lista de camada atual com as chaves rápidas (lâmpada, sol/floco, cadeado) — `ribbon-dados.ts` aceita `widget`
  e `acao` (ação da interface) além de `comando`.
- DXF: lê/grava congelada (bit 1), bloqueada (bit 4) e tipo de linha; lê espessura (370), plotar (290) e transparência (440); o R12 gravado não tem
  espessura/plotar/transparência — só o `.fcdz` guarda.
**Propriedades dos objetos** — `propriedades.rs`, `propriedades-controles.ts`, `propriedades-painel.ts` (Ctrl+1 ou o ↘ do painel): camada, cor,
tipo de linha e espessura da seleção ("variado" quando diferem); as três listas também estão na faixa. `Entidade.props: Propriedades{espessura,
tipo_linha, grupo}`; tipo de linha vira traço e vão **em pixels no Rust** (`quadro::escrever_tracos`); espessura vai no segmento do quadro.
**Botões novos:** texto, cota (linear), chamada, tabela (só a grade), medir (distância / `A` área), colar/copiar/recortar (prancheta **interna**:
Ctrl+C/X/V, nada vai para o sistema), tornar atual, igualar camada, igualar propriedades, criar bloco (converte a seleção em bloco + uma inserção),
agrupar (clicar num membro seleciona o grupo; agrupar de novo desagrupa). **Falta:** editar bloco (o único botão ainda apagado), cota alinhada/
raio/ângulo, texto com várias linhas na criação, propriedades por entidade "escala do tipo de linha" e transparência do objeto.
**Desempenho:** suspeita — o `npm run app:dev` compila o Rust em debug (10–50× mais lento nas contas). Se o mouse continuar lerdo no dev e bom no
build, otimizar os crates no perfil dev (`[profile.dev.package.feathercad-core] opt-level = 3` etc. em `src-tauri/Cargo.toml`).
(Um teste de tempo do cursor chegou a ser esboçado e foi removido sem terminar: medir antes de mexer no perfil.)

## ✅ Folhas (pranchetas) e escala (29/09/2026) — feito, testado, **não commitado**

`crates/feathercad-core/src/folha.rs`: a folha é **um bloco** `FOLHA-n` desenhado em mm de papel (moldura, marcas de centro, faixa
de notas/legenda/revisões e carimbo — a prancheta do escritório, base A1, proporcional nos outros papéis A4…16A0) e inserido com a
escala do desenho (`escala_x = 1:S ÷ mm da unidade`). Trocar a escala = trocar a escala da inserção (e o texto `1:S` do carimbo).
`Documento::{folha_inserir, folha_escala, folha_ajustar, folhas_inseridas}`; sem escala pedida o motor escolhe a menor de
`ESCALAS` em que o desenho cabe; a 1ª folha se centra no desenho, as seguintes vão à direita. Um Ctrl+Z por operação
(`Passo::Tudo`). O PDF (`pdf.rs::folhas`) exporta uma página por folha inserida (além dos retângulos da camada `FOLHA`).
Ponte: `folha_listar|folha_inserir|folha_escala|folha_ajustar`. Interface: aba **Inserir → painel Folha** (`caixa-folha.ts`:
"Inserir folha" e "Escala da folha"/"Ajustar ao desenho"). **Limites:** só deitada; os campos do carimbo são texto fixo do bloco
(editar depois = refazer a folha); a linha divisória do meio é opcional; sem "janela de vista" (viewport) com escala própria por
vista dentro da mesma folha.

## Fila de trabalho

0. **(29/09/2026, pedido do usuário) Folha no app.** ✅ **Exportar PDF feito** (não commitado): `crates/feathercad-io/src/pdf.rs` (PDF vetorial em Rust puro, Helvetica/WinAnsi, uma página por **folha** = retângulo fechado na camada `FOLHA`; sem folhas, o desenho inteiro; modo papel: cores claras → preto, preenchimento escuro clareia). Usa o `quadro` do motor com 1 px = 1 pt, então herda blocos, hachuras, tracejado e espessura. Ponte: `documento_exportar_pdf {papel, cores}`; Tauri: `arquivos::desenho_exportar_pdf` (caixa "Salvar"); TS: botão 📄 na barra de título (`exportarPdf` em `arquivos.ts`; no navegador baixa o arquivo). Sem interface: `cargo run --release --example exportar_pdf -- entrada.fcdz saida.pdf [A4|A3|A2|A1] [--cores]` (dentro de `crates/feathercad-io`). **Diálogo de exportação feito (29/09/2026, não commitado):** `src/ui/caixa-exportar.ts` pergunta papel, orientação (`Orientacao` no `pdf.rs`), folhas (`documento_folhas` lista; `folhas` = índices no pedido) e cores; testes em `tests/pdf.rs`. **Falta:** exportar **imagem** (PNG) e o **modo de vista em folha** (layout/espaço do papel com janela e escala, como no AutoCAD): a "folha" segue sendo só uma moldura desenhada no modelo (ver `exemplos/gerar_projeto.py`).
   **`cad_lote` (29/09/2026, feito, testado, não commitado):** muitas ferramentas numa chamada, com `dados`, laços declarativos
   (`para_cada`, `repetir`, `se`, `exigir`, `definir`, `guardar`) e expressões (`=[X + l[0], l[1]]`, `${…}`), tudo em Rust
   (`crates/feathercad-mcp/src/{lote,expressao}.rs`). Tudo ou nada via `Documento::marca_do_historico/juntar_historico/desfazer_ate`
   (um lote = um Ctrl+Z). Objetivo: **substituir os `exemplos/*.py`** — a IA passa os dados e o servidor processa. Próximos degraus
   combinados com o usuário: ferramentas de domínio (`cad_folha`, `cad_tabela`, `cad_cota`, `cad_paredes`, `cad_circuitos`,
   `cad_exportar` PDF) e depois **modelos** (`cad_modelos`/`cad_modelo`: arquivo JSON com `esquema` dos dados + `passos`, de fábrica
   no binário e do usuário em `~/.config/feathercad/modelos/`; o primeiro, `eletrico_residencial`, fica pronto quando reproduzir
   `casa_planta_eletrica`). Corrigido junto: `cad_criar` círculo com `centro` inválido virava (0,0) em silêncio.
   **MCP elétrico (29/09/2026, feito, testado, não commitado):** `cad_simbolos`, `cad_simbolo`, `cad_eletroduto` e o prompt `projeto_eletrico` (`crates/feathercad-mcp/src/{simbolos,eletroduto,guias}.rs`). Exige `npm run mcp:build` e reconectar o MCP. **Recarga automática:** `src-tauri/src/arquivos.rs::desenho_verificar_disco` (a cada 1 s a interface pergunta; recarrega se o arquivo mudou e não há alteração pendente).

1. ✅ (29/09) Esqueleto, casca, ferramenta Linha, Monitor (F12), fundo, zoom, página
   inicial, DXF/`.fcdz`. **Nada commitado ainda.**
2. **Perguntas em aberto ao usuário** (`docs/FUNCIONALIDADES.md`, fim): hachura na
   fase 1? linguagem de macro (só `.SCR` ou a da suíte)? georreferência? importar PDF
   pelo FeatherPDF?
3. **DXF R2000+ na gravação** (handles, BLOCK_RECORD, LAYOUT): só depois de haver como
   validar contra um leitor real (AutoCAD/ezdxf); hoje grava R12. O `README` decidiu
   R2018 como padrão — **desvio provisório**. Leitura de `$DWGCODEPAGE` de verdade
   (hoje UTF-8 ou Latin-1) e DXF binário também faltam.
4. **Vários desenhos abertos** (uma aba por desenho): o motor tem um documento só; a
   aba Início já existe. Exige `Ponte` com N documentos.
5. **Configurações**: formato padrão de salvamento (`.dxf` de fábrica) e versão do DXF
   ainda só por `localStorage` (`preferencias.ts`), sem tela.
6. `.fcdz` com `.git/` embutido (`FORMATO.md`, decisão de quando commitar pendente).
7. Miniaturas de verdade na página inicial (hoje um símbolo); menus "Abrir ▾/Novo ▾"
   do AutoCAD (sem itens ainda: modelos por unidade, etc.).
8. Mais entidades no motor (círculo, arco, polilinha, texto…) — cada uma leva junto a
   leitura/gravação de DXF, o apelido do AutoCAD (`docs/ATALHOS.md`) e a linha na Ribbon.
9. ✅ Edição básica (ver acima). Falta: estender, deslocar, pegas, seleção por arrasto, polar e rastreio, cotas, camadas (painel).
10. Pendências de casca: ícones do FeatherSheet, associação `.fcdz`/`.dxf` no Linux
   (mime + `.desktop`), `tauri build` não testado.
