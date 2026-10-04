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
- `docs/PLANO-MCP.md` — plano (01/10/2026) para o MCP fazer sozinho o que hoje exige Python:
  formato `.fct`, tabela/gráfico/detalhe, alinhamento/superelevação/greide/via, nuvem de pontos;
  com os valores de referência do projeto ferroviário de teste.
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
  página `monitor.html` + `src/monitor/` (abas Memória, Console, Métricas; **a aba Memória mostra também CPU e GPU** desde 30/09/2026: `memoria_arvore.rs` lê CPU por processo em `/proc/<pid>/stat` e GPU por processo nos `fdinfo` do DRM (`drm-engine-*`, `drm-resident-vram`, cliente único por `pdev/client-id`), com % calculada no Rust pela diferença entre amostras; a GPU do sistema vem do sysfs (`gpu_busy_percent`, `mem_info_vram_*`; AMD testado, NVIDIA via `nvidia-smi` e Intel só por nome, **não testados**); só Linux — no Windows o Monitor de memória ainda devolve `None`), janela
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
- **MCP ao vivo** (04/10/2026, `crates/feathercad-mcp/src/ao_vivo.rs`, `src-tauri/src/mcp_vivo.rs`; feito, testado, não commitado antes deste commit):
  o aplicativo escuta num socket Unix (`$XDG_RUNTIME_DIR/feathercad/mcp.sock`, pasta 0700, só local) e roda as ferramentas do MCP **sobre o
  documento aberto na tela** (o `Documento` da `Ponte` é trocado de lugar com o do `Agente` durante a chamada; o `Agente` do app guarda
  alinhamentos/nuvens/greides). O `feathercad-mcp` do Claude Code virou proxy: com o app aberto repassa os `tools/call`; sem ele, trabalha
  sozinho como antes (`FEATHERCAD_VIVO=0` força isolado — os testes de integração usam). Cada ferramenta = um Ctrl+Z; a interface pergunta
  `mcp_vivo_verificar` a cada 300 ms, aplica o `estado` (abre aba sozinha se estava na página inicial) e mostra "IA desenhando…" na barra
  de título. `cad_abrir`/`cad_salvar` acertam o arquivo da `Ponte` (`arquivos.rs::depois_da_ia`) para o recarregar-do-disco não apagar o
  histórico. `cad_ver` vê a tela do usuário. **Um aplicativo só** (as várias telas são abas): quem abre o FeatherCAD de novo entrega os
  arquivos da linha de comando ao que já está aberto (`ao_vivo::pedir_foco`, método `feathercad/instancia`), que vem para a frente e os abre
  em abas (`mcp_vivo_verificar` devolve `abrir`); o segundo processo sai. **Transporte:** socket Unix no Linux/macOS; no Windows, TCP em
  `127.0.0.1` com token num arquivo de `%LOCALAPPDATA%\feathercad\mcp.porta` (testado no Linux e compila para `x86_64-pc-windows-gnu`, **não rodado
  no Windows**). **Pânico:** `rpc.rs::chamar_ferramenta` captura (`catch_unwind`), desfaz o que a ferramenta mudou e devolve erro; por isso o
  `profile.release` do `src-tauri` **não** tem mais `panic = "abort"`. **Limites:** a ferramenta segura a trava de escrita (o mouse espera nas
  lentas, como o modelo `ferrovia`); a vista (zoom) do app não acompanha a IA (decisão do usuário: não precisa). Depois de mexer no MCP,
  **fechar e abrir o `app:dev`** (o `tauri dev` reinicia sozinho quando muda `src-tauri/`) e rodar `npm run mcp:build`.
- **MCP: plano de 01/10 (04/10/2026, feito, testado, não commitado):** ver o topo de `docs/PLANO-MCP.md`. Em resumo: `cad_modelo ferrovia` (e modelos do
  usuário em `~/.config/feathercad/modelos/*.json`, que rodam como `cad_lote`), `cad_alinhamento/_ajustar`, `cad_superelevacao`, `cad_greide`, `cad_via`
  (trilho/dormente vêm de `biblioteca/ferrovia/*.fct`), `cad_nuvem_secoes|feicoes (trilhos, bordos)|qualidade`, `cad_tabela_estacas`, `cad_projetar`,
  `.fct` com `camera`+`L3`/`P3`, `cad_pdf papel:"auto"`, `cad_verificar {leiaute}`. Falta HATCH no DXF (1.6) e `meio-fio`.
- **MCP: biblioteca de componentes em arquivos** (29-30/09/2026, `crates/feathercad-mcp/src/componentes.rs`, `build.rs`, pasta `biblioteca/`):
  cada equipamento é um `.fct` em mm reais (origem no canto inferior esquerdo) com cabeçalho `// descricao|categoria|tamanho|parametros: a=1 b=x`
  e `{{a}}` no corpo. `cad_componentes {busca?}` lista, `cad_componente {nome, ponto, valores?, ampliacao?, rotacao?, camada?}` coloca (um
  passo de desfazer; funciona em `cad_lote`). De fábrica: embutidos no binário pelo `build.rs` (largar um `.fct` em `biblioteca/**` e recompilar);
  do usuário: `~/.config/feathercad/biblioteca/**.fct` (ou `$FEATHERCAD_BIBLIOTECA`), que vale sobre o de fábrica. Os de fábrica atuais
  (disjuntor modular 1-4p, DR, DPS, trilho DIN, barras, pente, caixa de quadro, transformador lateral/frontal) saem de
  `biblioteca/gerar_biblioteca.py` a partir de `exemplos/pack_eletrica.py`. Um quadro inteiro = uma chamada de `cad_lote`. **Depois de
  `npm run mcp:build` é preciso reconectar o MCP (`/mcp`).** Falta: símbolos unifilares (fusível, seccionadora, TC, TP, pára-raios), cubículo MT,
  QGBT, carimbo/folha como componente, atributos (tag/kVA) e lista de materiais automática (estilo AutoCAD Electrical).
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
- **Limites conhecidos:** aparar não trata polilinha com arco nem texto/bloco; sem pegas (grips); **Estender (`EX`, 30/09/2026, `estender.rs`, não commitado)**: clique perto da ponta de linha, arco ou polilinha aberta (trechos retos) leva a ponta até a primeira outra forma visível no caminho (as mesmas arestas do Aparar; sem "borda implícita" nem escolha prévia das arestas); falta polilinha com arco e a opção de estender ao encontro de uma projeção; **Deslocar (`O`, 30/09/2026, `deslocar.rs`, não commitado)**: linha, círculo, arco e polilinha só de trechos retos (quinas em esquadro; recusa se a distância engole o contorno), distância digitada fica guardada e `0` = passar pelo ponto clicado, repete até Enter; falta polilinha com arco; o espelho de bloco
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
**Botões novos:** texto, cota (linear; **desde 30/09/2026 a cota é UM objeto**: `Documento::adicionar_cota` põe linhas, setas e texto num bloco anônimo `*D<n>` e insere uma vez, com a base no 1º ponto medido — seleciona/move/copia/apaga como um só, um Ctrl+Z desfaz, grava como bloco `*D` no DXF/`.fcdz`; **limite:** não é paramétrica, esticar não refaz a medida, e o snap só pega o ponto-base; chamada e tabela ainda saem como formas soltas), chamada, tabela (só a grade), medir (distância / `A` área), colar/copiar/recortar (prancheta **interna**:
Ctrl+C/X/V, nada vai para o sistema), tornar atual, igualar camada, igualar propriedades, criar bloco (converte a seleção em bloco + uma inserção),
agrupar (clicar num membro seleciona o grupo; agrupar de novo desagrupa). **Cotas alinhada, de raio e de diâmetro (30/09/2026, não commitado):** `CAL`/`CRA`/`CDI` (en `DAL`/`DRA`/`DDI`), botões no painel Anotação; raio/diâmetro escolhem círculo ou arco e depois a posição do texto (`R5`, `Ø10`); todas viram um objeto só (`adicionar_cota`). **Falta:** editar bloco (o único botão ainda apagado), cota de ângulo, cota paramétrica, texto com várias linhas na criação, propriedades por entidade "escala do tipo de linha" e transparência do objeto.
**Desempenho (30/09/2026, medido):** `crates/feathercad-ponte/examples/medir_movimento.rs` (`cargo run [--release] --example medir_movimento -- [arquivo]`) mostrou 50 mil linhas = quadro 25 ms / cursor 9 ms em debug contra 3,4 / 2,5 ms em release; corrigido com `[profile.dev.package.feathercad-*] opt-level = 3` em `src-tauri/Cargo.toml` e com pan/zoom agrupados por quadro de tela em `area-desenho.ts` (antes cada `mousemove`/roda pedia um quadro). **Perfil de quadro** (30/09/2026, `src/ponte/perfil.ts`, liga junto com o medidor de FPS): cada etapa de um quadro (`vista_zoom`, `quadro`, `quadro/decodificar`, `ponte/xhr`, `pintar_fundo` e o detalhe `pintar/grade|preenchimentos|tracos|textos`, `cursor`, `pintar_topo`, `ao_cursor`) é cronometrada com `performance.now()` e resumida no console/Monitor a cada 1 s por tipo de quadro (`zoom`, `zoom-seco`, `pan`, `mouse`), com quadros > 33 ms impressos sozinhos. Etapa nova = `etapa('nome', () => …)`; nome com `/` é parte de outra. **Achado (usuário, 30/09):** no zoom o quadro leva ~72 ms e `pintar_fundo` é 68–72 dele (ponte+motor+DOM somam 2–5 ms); mouse parado/andando = 1,8 ms. **Causa do lag do zoom, achada com o perfil (30/09):** `pintar/textos` = 106–135 ms por quadro (traços 1 ms, grade 0). Trocar a altura da letra a cada quadro do zoom faz o WebKit re-rasterizar cada glifo grande (o cache não pega). **Correção (final):** o texto tem **camada de canvas própria** (`.textos`, `Z.textos`); durante zoom/pan ela não é redesenhada, só levada à vista de agora por `transform: matrix()` CSS (`area-desenho.ts::acompanharTextos`, a matriz sai das duas projeções `Vista::para_tela`), e volta nítida `ESPERA_DO_TEXTO_MS` (150 ms) depois do último movimento (`emMovimento`/`pintarCamadaDeTextos`); as linhas continuam repintadas a cada quadro (~1 ms). Efeito colateral: o texto fica levemente borrado durante o zoom e, no zoom-out, o texto que estava fora da tela ainda não aparece até parar. Configurações → Desenho → "Texto completo no zoom e no pan" (`textoNoMovimento`, desligado de fábrica) volta a redesenhar o texto a cada quadro. Erro que já custou uma rodada: `ctx.restore()` devolve fonte/alinhamento ao padrão, então cache de fonte não pode conviver com `save/restore` (teste em `test/ui/pintura.test.ts`). **LOD e snap (30/09/2026, medido com `brasilia-brazil.dxf` e `exemplos/gerar_pesado.py`):** (1) `snap::achar` comparava todas as primitivas do alcance entre si (O(k²)): `cursor_mover` chegava a 2,8–5 s num mapa denso; agora só as `MAX_PARA_INTERSECAO` (48) mais perto entram na conta (máximo da varredura 503 → 5 ms; `crates/feathercad-ponte/examples/varrer_cursor.rs`) e `longe_demais` rejeita por caixa sem decompor a entidade. (2) LOD no `quadro.rs` (só a tela, `Saida.lod`; PDF/`montar_fino` seguem completos): `decimar` (pontos a < 0,8 px do último viram um só), traço que cabe num pixel sai uma vez por pixel e cor (`ocupados`), bloco < 2 px vira ponto (`BLOCO_MINIMO_PX`). Resultado: Brasília 3,3 → 1,2 MB por quadro, projeto pesado 19 → 12,8 MB. **Ainda pesa:** o quadro é ~24 B por segmento e atravessa o XHR síncrono como texto (`ponte/xhr` ≈ 30–190 ms em 3–12 MB); opções: formato mais compacto, ou o motor em wasm na página (sem XHR). `cargo run --release --example medir_movimento -- arquivo`. **Pintura do quadro em Worker (30/09/2026, verificada no WebKitGTK real com `exemplos/webkit_captura.py`):** decodificar e pintar centenas de milhares de traços custava 50–70 ms por quadro na thread principal (`pintar/tracos` + `pintar/agrupar`). Agora `src/ui/pintor-quadro.ts` manda os bytes (transferidos, sem cópia) para `quadro-worker.ts`, que pinta em duas `OffscreenCanvas` (`pintor-nucleo.ts`, mesmo código do fallback na thread principal) e devolve `ImageBitmap`s; a thread principal os instala com `bitmaprenderer` **na mesma tarefa** em que troca a matriz CSS (sem piscar). `Motor.quadroBrutoAsync` entrega os bytes crus; `lerQuadro` mora em `src/ponte/quadro-binario.ts` (roda no Worker). Configurações → Desenho → "Pintar o desenho em segundo plano" (`segundoPlano`, ligado; vale ao reabrir; sem suporte a `OffscreenCanvas` cai sozinho na thread principal). No perfil o Worker aparece como `pintor/decodificar|fundo|textos` dentro de `quadro-chegou`. **Quadro por IPC binário assíncrono (30/09/2026, copiado do padrão do FeatherSheet — `ler_arquivo` devolve `tauri::ipc::Response`):** o quadro (3–12 MB) não atravessa mais o XHR síncrono do `ponte://` (que o entregava como texto: `ponte/xhr` 105–125 ms + `ponte/bytes` 12 ms medidos). Comando Tauri `ponte_binaria` (`src-tauri/src/lib.rs`, `spawn_blocking`, `Response::new(bytes)` → `ArrayBuffer`), `nativo.ts::ponteBinaria`, `Motor.quadroAsync` (no navegador/testes segue sendo o `quadro` síncrono). `pedirQuadro()` em `area-desenho.ts` é assíncrono: **um pedido por vez, o último estado vale**; o quadro que chega é pintado com a vista **do pedido** e as camadas (linhas e textos) são levadas à vista de agora por `transform: matrix()` CSS (`acompanhar`/`acompanharCamadas`) até o próximo chegar — faixas vazias nas bordas durante um pan rápido são o efeito colateral. Comandos pequenos (cursor, vista) seguem no `ponte://` síncrono. **`RwLock<Ponte>`** no `src-tauri` e `Ponte::chamar_leitura(&self)` (quadro, quadro_arrasto, cursor_mover, vista_*, estado) deixam o cursor andar enquanto o quadro é calculado (`ultimo_cursor` virou `Mutex`); o resto usa a trava de escrita. No perfil o quadro aparece como `[perfil] quadro-chegou` (`quadro/latencia(pedido→resposta)` + `pintar_fundo`). **GPU (30/09/2026, compila; efeito na placa AINDA NÃO CONFERIDO no app):** o usuário viu que a placa de vídeo não se mexia no zoom; o WebKitGTK usa `HardwareAccelerationPolicy::ON_DEMAND` por padrão (GPU só com animação 3D/WebGL). `src-tauri/src/lib.rs::ligar_aceleracao_por_gpu` (Linux, no `.setup`) põe `Always` e imprime `[FeatherCAD] aceleração por GPU: X → Y` no terminal; dependência `webkit2gtk = 2.0.2` com `v2_16` em `src-tauri/Cargo.toml`. Conferir se a placa passa a trabalhar e ler a linha no terminal. Se não ajudar, o passo seguinte é WebGL. **Zoom suave** (30/09/2026, `area-desenho.ts::pedirZoom/passoDoZoom`, Configurações → Desenho, ligado de fábrica): a roda e os botões +/− animam o zoom (a cada quadro anda uma fração do que falta, `TEMPO_DO_ZOOM_SUAVE_MS`); cada passo é um `vista_zoom` do motor, a interface só dita o ritmo; desligado = zoom seco, como no AutoCAD. **Medidor de FPS** (`src/ui/medidor-fps.ts`; Configurações → Sobre ou Ctrl+Shift+F; desligado de fábrica; `seg` = segmentos de reta do quadro, `txt` = textos): FPS durante o movimento, pior quadro e tempo médio de motor/cursor/pintura — se as etapas somam pouco e o FPS é baixo, o gargalo é a tela (webview/canvas), não o motor. **Tentativa que piorou (30/09/2026, revertida):** deslocar/escalar a imagem pronta com `drawImage` do canvas nele mesmo durante pan/zoom e repintar a cada 120 ms — no app do usuário (Wayland, AMD RX 7600, WebKitGTK 2.52) ficou pior; não repetir sem medir. Medidas antes: pintura 23 ms/quadro num desenho cheio de texto, motor e cursor ~1 ms. **Ainda linear:** o motor varre todas as entidades por quadro e por movimento do cursor (2–3 ms em 50 mil, release); índice espacial só se isso pesar. Histórico da suspeita: o `npm run app:dev` compilava o Rust em debug (10–50× mais lento nas contas). Se o mouse continuar lerdo no dev e bom no
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
