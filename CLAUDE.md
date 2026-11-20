# FeatherCAD — instruções de trabalho

**Repositório próprio desde 16/11/2026:** `feather-works/feathercad` (pasta `GitHub/feathercad`). Saiu do FeatherOffice; a pasta `FeatherOffice/feathercad` não existe mais. Não há `Cargo.toml` na raiz: cada crate em `crates/` e o `src-tauri/` compilam em separado.

**Direção atual — 17/11/2026: interface em Rust no pelicano** (o Iced foi apagado da suíte em 02/11/2026).

## ⚠ EM ANDAMENTO (17/11/2026): interface no pelicano — `crates/feathercad` — RETOMAR DAQUI

Pedido do usuário: "passa tudo para Rust usando pelicano". O app Tauri (`src/` TypeScript + `src-tauri/`) **continua
existindo até o novo ter tudo**; só então apagar os dois (e mover `src/idiomas/*.json` para dentro do crate, hoje lidos
por `include_str!("../../../src/idiomas/…")`).
- Arquitetura: a casca fala com o motor **só pela `Ponte`** (`motor.rs`, mesmo contrato JSON do `src/ponte/index.ts`);
  o desenho vem do `quadro` FCQ1 (`quadro.rs`, decodificador com teste contra o motor) e é pintado em `area.rs` (um
  `pelicano::Program`: caminhos montados uma vez por quadro, um por cor/espessura; contornos dos textos guardados na
  1ª pintura). MCP ao vivo em `mcp_vivo.rs` (pedido chega ao `update` por subscription, documento troca de lugar com o
  do `Agente`); segundo FeatherCAD entrega os arquivos ao primeiro (`main.rs`, `pedir_foco`).
- **Feito (17/11/2026, 10 testes, clippy limpo, compila para Windows; conferido por vídeo com `PELICANO_ROTEIRO`+`PELICANO_GRAVAR`):**
  todas as janelas do TS. Abas de desenho (`doc.rs`: **cada aba tem o próprio motor e o próprio Ctrl+Z**; no TS trocar de
  aba regravava o desenho e perdia o histórico), "Alterações não salvas" ao fechar aba e ao sair (um desenho por vez),
  página inicial (`inicio.rs`: Recentes/Projetos, lista/miniaturas em segundo plano), unidade, camadas (`camadas.rs`),
  paleta de propriedades (`propriedades.rs`, Ctrl+1; só os campos que o motor tem — os 3D que o TS listava davam erro),
  Configurações (`configuracoes.rs`: tema/destaque, fundo, Snap/Orto/OSNAP, cota, ângulo do Orto, formato, idioma,
  apelidos, MCP com Testar, Sobre/FPS), hachura, folha (inserir/escala/carimbo) e exportar PDF (`caixas.rs`), faixa
  completa com camada atual e cor/espessura/tipo (`faixa.rs`; ícones SVG do TS em `icones.rs`), cubo de vista de verdade (`cubo.rs`: faces, arestas,
  quinas, arrastar = órbita, bússola, ⌂) e barra de navegação, grade 3D girada, imagens do quadro, abas de layout,
  menu de tipos de snap, avisos flutuantes, Monitor F12 (`feather_ui::monitor`, janela à parte), recarregar quando o
  arquivo muda no disco, MCP ao vivo (testado ponta a ponta com o `feathercad-mcp` real). Preferências em
  `<dados>/br.feathercad.app/config.json` (`config.rs`).
- Consertos de JSON junto: `estender.clique/semLimite/naoSuporta` estavam dentro de `ribbon.botoes` (prompt do Estender
  saía como chave crua); textos fixos do TS ("FRENTE", "Modelo", "Novo layout", cobrir seleção) viraram chaves.
- **Ainda não feito / diferente do TS:** fonte e corpo da interface, "pintar em segundo plano", "texto no movimento" e
  zoom suave (eram da webview); data da página inicial em UTC (sem fuso). **Defeito visto 2×, intermitente:** o pelicano
  desenhou texto errado num campo ("drag", "> e L") com DXF cheio de textos — o valor real estava certo; suspeita no
  cache/atlas de glifos do pelicano; não reproduziu ao repetir.
- **Para trocar de vez:** testar no uso real (Linux e Windows), depois apagar `src/`, `src-tauri/`, `index.html`,
  `package*.json`, `vite.config.ts`, `test/` e mover `src/idiomas/*.json` para o crate.
- Rodar: `cargo run --release --manifest-path crates/feathercad/Cargo.toml [desenho.dxf]`.

Inventário anterior (abaixo) registra funcionalidades do app Tauri; não presumir que já existem no pelicano.

## Auditoria de 19/11/2026 (corrigido)

- **Arquivos:** HATCH com contagem `79` absurda travava a abertura; handles `330` com letras derrubavam hachura com ilha;
  POLYLINE 3D (as curvas de nível que o próprio app grava) era descartada ao reabrir; `.fcdz` e DXF agora guardam o
  espaço de papel (`espaco`, `67 1`) e os `layouts`; `.fcdz` recusa o que passa de 1 GB descompactado; no máximo
  200 mil inserções por DXF (MINSERT). Gravações: `.parcial` + `sync_all` + rename (app e MCP).
- **Motor:** aparar polilinha fechada no trecho que passa pelo vértice 0; `2,5` em pt-BR é distância (a vírgula só
  separa coordenadas quando o decimal não é vírgula); desfazer em lote numa passada só (`aplicar_lote_simples`) e a
  entidade apagada volta no mesmo lugar da ordem de desenho (a lista fica em ordem de id); cota/bloco/símbolo usam
  `Passo::AdicionouBloco` em vez de copiar o desenho inteiro; trocar de layout não marca o desenho como alterado.
  `viewport_inserir` (sem uso) saiu da ponte e do TS. `geom::dentro_do_laco` e `geom::distancia_ao_segmento` são as
  únicas cópias (o cruzamento de retas segue com 3 versões de tolerâncias diferentes — juntar exige testar cada uso).
- **MCP:** ao vivo, `cad_novo`/`cad_abrir`/`cad_importar {substituir}`/`cad_modelo ferrovia` recusam com alteração não
  salva, a menos de `descartar: true`; `cad_modelo` não cabe em `cad_lote` (modelo que chamava a si mesmo estourava a
  pilha); `arranjo` até 200×200, grade do luminotécnico com teto de ~5·10⁷ contas, `decimais` até 12; `cad_ver
  {arquivo}` pede `sobrescrever`; `cad_topografia {caminho}` não troca mais o arquivo da aba pelo `.csv`; pedido ao vivo
  que venceu os 300 s não roda mais depois.
- **Interface pelicano:** Salvar pela pergunta "Alterações não salvas" grava a aba perguntada (`Doc::id`, não o
  índice); Descartar ao sair fecha a aba; o aviso "mudou no disco" volta a valer após salvar/recarregar; Ctrl+S sobre
  DXF não-R12 (ou que perdeu algo ao abrir) vira "Salvar como" (`Doc::dxf_perderia`); arrastar a vista desenha no
  máximo um quadro a cada 16 ms. O app Tauri não recebeu estes consertos da casca (vai ser apagado).

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
- `docs/PLANO-10X.md` — plano (07/10/2026) para superar o CADCraft (`storytold/cadcraft`): metas medíveis,
  fase 0 = seleção em desenho pesado (mapa de ids + R-tree), depois 2D diário, DXF R2000+/DWG, layout de papel.
- `testes/amostras/` (ezdxf, ver `LEIA-ME.md`) e `testes/amostras-terceiros/`
  — corpus para o teste de ida e volta.

## Arquitetura (o que existe)

- `crates/feathercad-core` — motor: `documento` (camadas, entidades, desfazer),
  `cursor` (snap/orto/grade), `vista` (zoom/pan), `sessao` (ferramenta, linha
  de comando, interruptores), `atalhos` (teclas do AutoCAD), `comandos`
  (apelidos por língua), `formato` (número na língua), `quadro` (binário).
- `crates/feathercad-io` — `dxf::leitor` (cerca de 20 tipos de entidade; o que não lê vira
  aviso), `dxf::escritor` (**R12/AC1009**), `fcdz` (ZIP + JSON), `aci`, `inicio`
  (recentes, pastas de projeto, listagem — tudo local). Testado contra as 46
  amostras de `testes/` (todas abrem; o que não é `LINE` é avisado).
- **Distância digitada** (29/09/2026, `sessao.rs::ler_ponto`): com uma ferramenta que tem ponto
  anterior (linha, polilinha, mover/copiar, retângulo…), um número solto vira ponto **a essa
  distância, na direção do cursor** (já com orto/snap, guardado pela ponte em `ultimo_cursor`);
  no círculo, com o centro marcado, é o raio. O número vale na unidade do desenho; aceita
  sufixo (`10cm`, `2,5m`, `300mm`) e converte. Sem direção (cursor parado na base) a linha de
  comando avisa (`comando.semDirecao`).
- **Tamanho da cota**: Configurações → Desenho; `cota_tamanho_definir` (Rust, `Sessao::altura_cota`,
  `None` = o do texto). A preferência fica no `localStorage` (`feathercad.cota`, na unidade do
  desenho) e é reenviada ao motor na largada — não vai dentro do arquivo.
- **Ângulo na linha de comando** (29/09/2026, como no AutoCAD; `sessao.rs::ler_ponto` e `digitar`): polar
  `@10<45` (a partir do ponto anterior) e `10<45` (a partir da origem); ângulo em graus, anti-horário do
  eixo X, distância com unidade opcional (`@50cm<30`, decimal na língua: `@2,5<30`). `<45` sozinho, com um
  ponto anterior, **trava a direção** (`Sessao::angulo_travado` → `Ajudas.direcao`: o cursor passa a andar
  só nessa reta) e a próxima distância digitada sai nela; o lado (frente/trás) é o que o cursor indica.
  A trava some no próximo ponto, ao cancelar ou ao trocar de comando.
- **Projeto ferroviário completo (04/10/2026, feito e testado, commitado):** `exemplos/ferrovia_completa.py nuvem.csv saida_base [--vivo] [--so C6,T7]` (módulos
  `ferrovia_lib/_elementos/_nuvem/_resumo.py`) dirige o MCP: `cad_modelo ferrovia` (18 folhas-base, agora com **perfil longitudinal** em A1 e greide compensado) +
  **uma folha A3 por curva e por tangente** (criticidade, aterro/escavação, dormentes) + **folha da nuvem de pontos** com detalhe geográfico + **resumo/quantitativos**;
  renumera o carimbo e grava `.fcdz/.dxf/.pdf` (47 folhas no caso de teste, 41 s). **Por pedido do usuário o cálculo auxiliar é em Python** (a meta "sem Python" do
  plano ficou de lado aqui); o que já estava em Rust ficou: `viario/criticidade.rs` (Criterios ajustáveis, `analisar`), `geodesia.rs` (UTM→graus; a zona 23 S é
  premissa), `perfil.rs`, e as ferramentas `cad_analise`, `cad_geografico`, `cad_alinhamento_ponto {de,ate,passo}`, `cad_tabela_estacas {de,ate}`, `cad_verificar {limite}`.
  Achados do caso de teste: ripagem de 0,8 m deixa C6 e T1 CRÍTICOS; 17 em ATENÇÃO; **282 de 299 pares têm os 2 trilhos na mesma cota** (cota copiada, a superelevação
  medida não é confiável); aterro ≈ escavação ≈ 420 m³ (estimativa, plataforma de 4 m). Limites: limiares da criticidade são premissas (não norma); volumes sem seção do terreno.
- **MCP e cores/tema (04/10/2026, feito e testado):** o prompt inicial (`rpc.rs::INSTRUCOES`) agora manda escolher cores que contrastem com o fundo do tema
  (claro/escuro): #FFFFFF e #000000 são automáticos (`quadro::cor_na_tela` troca), nada quase branco no tema claro nem quase preto no escuro. O app informa o
  tema ao motor pelos pedidos de `quadro` (`Ponte::tema_claro`), o `mcp_vivo` o passa ao `Agente.tema_claro`, `cad_resumo` devolve `tema`, e `cad_criar` devolve
  `aviso` quando a cor (ou a da camada) tem contraste < 2 com o fundo (`cores.rs`). Cores do modelo `ferrovia` e do carimbo (`CARIMBO` era #E0E0E0, sumia no tema
  claro) foram escurecidas. Falta: avisar também nas outras ferramentas que aceitam `cor` (via, tabela, gráfico…) e nas cores por camada de `cad_camadas`.
- **MCP: plano de 01/10 (04/10/2026, feito, testado, não commitado):** ver o topo de `docs/PLANO-MCP.md`. Em resumo: `cad_modelo ferrovia` (e modelos do
  usuário em `~/.config/feathercad/modelos/*.json`, que rodam como `cad_lote`), `cad_alinhamento/_ajustar`, `cad_superelevacao`, `cad_greide`, `cad_via`
  (trilho/dormente vêm de `biblioteca/ferrovia/*.fct`), `cad_nuvem_secoes|feicoes (trilhos, bordos)|qualidade`, `cad_tabela_estacas`, `cad_projetar`,
  `.fct` com `camera`+`L3`/`P3`, `cad_pdf papel:"auto"`, `cad_verificar {leiaute}`. Falta HATCH no DXF (1.6) e `meio-fio`.
- **MCP guia a IA: `cad_projeto` (04/10/2026, `receita.rs`, feito e testado, não commitado):** funil pedido pelo usuário. Pedido SIMPLES (desenhar um círculo) vai direto
  às ferramentas; pedido de PROJETO começa por `cad_projeto {acao:iniciar}` → pergunta a disciplina (elétrica/civil/ferroviário) → `escolher {caminho:[…]}` até a
  RECEITA (perguntar antes, regras, fases e itens obrigatórios com a ferramenta de cada um) → `status {caminho}` confere cada item contra o desenho aberto
  (camadas/textos esperados; `feito|pendente|manual`), dá o `proximo` passo e **o equilíbrio por folha** (ocupação da área útil, caixas NOTAS/LEGENDA vazias; limiar 60 %,
  calibrado nos A1 de referência: 89–98 %). Receitas: `eletrica/luminotecnico` (completa), `eletrica/eletrica_geral` (básica), `civil/planta_baixa` (esboço),
  `ferroviario/tracado_nuvem` (aponta a cadeia viária existente); `outra` = projeto sem receita. Receita nova = entrada em `receita.rs` (itens com `Detecta`).
  As `INSTRUCOES` do MCP (`rpc.rs`) mandam usar o funil só em pedido de projeto. O v3 do galpão passa nos itens (23/26) mas **reprova no equilíbrio** (31–50 % de ocupação).
- **MCP elétrico: lições do apartamento (04/11/2026, feito e testado, não commitado):** `cad_parede`, `cad_ambiente`,
  `cad_ramais`, `cad_simbolo {encostar, bloco}`, `cad_contar {simbolos}`, leiaute ligado no `cad_salvar`; detalhes na
  "Fase elétrica" de `docs/PLANO-MCP.md`. **Para testar uma correção do MCP**: `cargo build --release` no
  `crates/feathercad-mcp` e reiniciar o MCP; a sessão em curso continua com o binário antigo.
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

## ⚠ EM ANDAMENTO (29/09/2026): blocos (INSERT) — RETOMAR DAQUI

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

**Perguntas ainda sem resposta do usuário** (`docs/FUNCIONALIDADES.md`, fim): hachura na fase 1? linguagem de
macro (só `.SCR` ou a da suíte)? georreferência só UTM/WGS84? importar PDF vetorial pelo FeatherPDF?

## ✅ Edição da aba Início (29/09/2026, noite) — feito, testado, **não commitado**

Comandos novos (todos no Rust; botões da faixa ligados; apelidos pt/en em `comandos.<língua>.json`):
**apagar, mover, copiar, girar, escala, espelhar, esticar, matriz, aparar, concordância, retângulo, elipse, hachura.**
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

- `Camada{nome, cor, ligada, congelada, bloqueada, plotar, tipo_linha, espessura, transparencia}`. Congelada some do desenho e das escolhas;
  bloqueada aparece mas **não se escolhe nem se edita**; transparência mistura a cor com o fundo (`quadro::misturar_com_o_fundo`);
  espessura e tipo de linha da camada valem para o objeto "por camada". Plotar só é guardado (não há impressão).
- O gerenciador (botão "Propriedades de camada") tem o layout do AutoCAD: Status, Nome (duplo clique renomeia), Ativada, Congelar, Bloquear,
  Plotar, Cor (seletor do sistema), Tipo de linha, Espessura, Transparência; árvore de filtros (Todas / Todas as usadas); busca com `*` e `?`;
  clicar no cabeçalho ordena; várias linhas (Ctrl/Shift) recebem a mesma mudança; contagem "N camadas exibidas de M". **Filtrar, buscar, ordenar,
  validar nome e aplicar são do Rust** (`camadas` e `camada_acao` na ponte). Regras: `0` não se renomeia/exclui; a atual não se congela/exclui;
  só se excluem camadas sem objetos (índices se remapeiam; um Ctrl+Z restaura); nome único sem diferenciar maiúscula; sem `< > / \ " : ; ? * | = , \``.
- Histórico: mudança de camada = captura da tabela (`Passo::Camadas`); excluir = captura de tudo (`Passo::Tudo`). Um Ctrl+Z por ação.

## Fila de trabalho

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
6. `.fcdz` com `.git/` embutido (`FORMATO.md`, decisão de quando commitar pendente).
7. Miniaturas de verdade na página inicial (hoje um símbolo); menus "Abrir ▾/Novo ▾"
   do AutoCAD (sem itens ainda: modelos por unidade, etc.).
8. Mais entidades no motor (círculo, arco, polilinha, texto…) — cada uma leva junto a
   leitura/gravação de DXF, o apelido do AutoCAD (`docs/ATALHOS.md`) e a linha na Ribbon.
9. ✅ Edição básica (ver acima). Falta: estender, deslocar, pegas, seleção por arrasto, polar e rastreio, cotas, camadas (painel).
