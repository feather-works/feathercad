# Plano de trabalho — tirar o Python do MCP

> **Estado em 04/10/2026 (NÃO commitado):** feitos 0.1–0.5, 1.1–1.5, **1.7**, 2.1–2.6, **2.7**, 3.1–3.7, 4.1–4.3 (+4.4 pelo nome guardado na sessão), **4.2 `bordos`** e modelos do usuário.
> **Faltam:** 1.6 (HATCH no DXF, bloqueado: exige R2000+) e 4.2 `meio-fio`. Conferir `bordos` com um levantamento real (testado só com seções sintéticas).
> **Fechado nesta rodada (04/10):** `cad_pdf papel:"auto"` (cada página no papel da própria folha) e `pdf_indice` em `cad_folha listar`;
> `.fct` com `camera …` + `L3`/`P3` (projetados ao importar); `cad_via` lê `trilho_*`/`dormente_*` de `biblioteca/ferrovia/*.fct` (`trilho:"TR-57"`, `dormente:"concreto"`);
> `cad_nuvem_feicoes {tipo:"bordos"}` (crista da plataforma + pé do talude; `desenhar:true`); `cad_modelo` roda `~/.config/feathercad/modelos/*.json`
> (`{descricao, esquema, dados_padrao?, passos, retornar?}` = um `cad_lote`).
> **Leiaute do modelo `ferrovia` com o CSV real:** rótulos de km à direita e TS/SC/CS/ST sempre à direita do eixo (estacas à esquerda), letras da rosa dos ventos
> fora da ponta, perspectiva da prancha 3 na escala certa. `cad_verificar {leiaute}`: 24 → **8** `texto_sobre_texto` (todos entre rótulos de curvas coladas em 1:10000 —
> inerente à escala, sem linha de chamada) e 0 `texto_sobre_linha`. Folhas 1, 3 e 5 conferidas por imagem; 2 e 4 ainda não.
> Ajuste automático: RMS 0,237 / máx 0,785 (ref. manual 0,182 / 0,643), 13 curvas (ref. 12).
> Código novo: core `viario/{alinhamento,superelevacao,greide,ajuste}.rs`, `nuvem.rs`, `trajeto.rs`; mcp `viario*.rs`, `via.rs`, `modelos.rs`, `nuvem_ferramentas.rs`,
> `tabela/grafico/texto_bloco/detalhe/ao_longo/malha_norte/projecao/leiaute/dados/regras.rs`. Depois de `npm run mcp:build`, reconectar com `/mcp`.

**Origem (01/10/2026).** Projeto ferroviário de teste: nuvem de pontos
`levantamento_topografico.csv` (1 237 pontos, 6,2 km, bitola métrica, V = 65 km/h) virou 4 pranchas A0.
O MCP fez ~20 % do trabalho (abrir, ver, folha, DXF/PDF); os outros 80 % foram scripts Python
improvisados (numpy): eixo existente, traçado com clotoides, superelevação, greide, tabelas,
diagramas, perspectivas, trilhos e dormentes. **Meta: o mesmo projeto sai só de chamadas MCP,
sem nenhuma linha de Python**, e com bem menos tokens.

## Regras do plano

1. **Toda conta é Rust.** Geometria e engenharia moram no `feathercad-core` (como `topografia.rs`
   já faz), em módulo próprio; a ferramenta MCP é só a casca (lê JSON, chama o core, desenha,
   devolve números). Assim o app ganha o mesmo recurso depois, sem reescrever.
2. **Toda ferramenta nova devolve os números, não só desenha.** A IA precisa dos resultados
   (TS/SC/CS/ST, he, I, levante…) para escrever o relatório sem recalcular.
3. **Toda ferramenta nova funciona dentro do `cad_lote`** e é um passo de desfazer.
4. **Limites normativos em tabela, não no código.** Um JSON de regras (embutido no binário + o do
   usuário em `~/.config/feathercad/regras/`), como a biblioteca de componentes. A norma muda,
   o binário não.
5. **Teste com valores de referência** (tabela no fim deste arquivo): cada item só fecha quando
   reproduz os números do projeto de 01/10.
6. Depois de `npm run mcp:build`, reconectar o MCP no Claude Code (`/mcp`).

Legenda: ☐ a fazer · ◐ em andamento · ☑ feito.

---

## Nível 0 — economia de contexto (só no **modo projeto**)

**Por quê (medido em 01/10, `/usage`):** 85 requisições, 98 % do input vindo do cache, mas **76 % do
uso com contexto > 150 mil** e **24 % do uso vindo de resultados do MCP `feathercad`** (sobretudo as
imagens do `cad_ver`, que ficam no contexto e são relidas em toda requisição seguinte). O custo real é
≈ Σ (tamanho do contexto por requisição): o projeto de referência processou ~12 milhões de tokens.
Meta com o plano todo: **~1 milhão** (ver "Metas").

**Quando vale:** só ao **gerar um projeto** (pranchas a partir de dados, modelo, `cad_lote` grande). No
uso normal — o usuário pede para ver, mexer, perguntar — nada muda: imagem cheia, resposta completa.

☑ **0.1 Modo projeto** (04/10: `cad_modo` e `Agente.projeto` feitos; só `cad_ver` lê o estado por enquanto; a volta automática do `cad_modelo` fica para quando ele existir) — `cad_modo {projeto: true|false}` (padrão `false`); liga sozinho dentro de
`cad_modelo` e volta ao estado anterior no fim. É só uma troca de **padrões** das ferramentas abaixo;
qualquer argumento explícito na chamada continua valendo sobre o padrão.
- Onde: estado no `Agente` (`agente.rs`), lido por `cad_ver`, `cad_tabela_estacas`, `cad_alinhamento*`,
  `cad_greide`, `cad_nuvem_*`.

☑ **0.2 `cad_ver` leve no modo projeto** (04/10: 800 px, sem grade/rótulos no modo projeto; `arquivo: x.png` grava no disco sem imagem na conversa; falta o "grade desligada só quando a janela é uma folha inteira" — hoje é tudo ou nada) — largura padrão 800 px (fora do modo segue 1000);
`arquivo: caminho` grava o PNG no disco e devolve só o caminho e a janela (sem imagem na conversa);
`grade`/`rotulos` desligados por padrão quando a janela é uma folha inteira.

☐ **0.3 Resultado grande vai para arquivo** — no modo projeto, listas longas (tabela por estaca,
ripagem por ponto, seções da nuvem, pontos do eixo) são gravadas em JSON ao lado do desenho
(`<desenho>.dados/<ferramenta>.json`) e a resposta traz **resumo + caminho**. As ferramentas seguintes
aceitam esse caminho como entrada (`{"de_arquivo": "…"}`), então os números não passam pela conversa.
Fora do modo projeto, limite de linhas na resposta com aviso "N linhas a mais em <arquivo>".

☐ **0.4 `cad_verificar {leiaute: true}`** — confere em texto o que hoje só se via na imagem:
texto sobreposto a texto/linha, texto fora da área útil da folha, texto de cabeça para baixo
(giro entre 90° e 270°), tabela/gráfico que estoura a caixa, elemento entre duas folhas. Devolve
lista curta com id e posição; a imagem só é pedida onde houver problema.

☐ **0.5 Guia de sessão no prompt do modelo** — o texto do `cad_modelo`/prompt de projeto lembra a IA
de: conferir por `cad_verificar {leiaute}` antes de pedir imagem; uma imagem por folha, depois só
recortes; e sugerir ao usuário `/compact` ao fechar cada etapa (nuvem → traçado → pranchas).

### Metas (projeto de referência)

| | Hoje (01/10) | Meta |
| --- | --- | --- |
| Requisições | ~85 | 15–18 |
| Conteúdo novo na sessão | ~270 mil tokens | ~50 mil |
| Contexto no fim | ~260 mil | ≤ 80 mil |
| Total processado (Σ contexto) | ~12 milhões | ~1 milhão |
| Cota da sessão usada (`/usage`) | ~20 % | ~2–3 % |
| Python | ~30 scripts | 0 |

Medir de novo, numa sessão nova, ao fechar o item 3.7.

---

## Nível 1 — formato e encanamento (pequeno, destrava o resto)

☑ **1.1 `cad_importar {caminho}`** (04/10, só `.fct`; teste em `tests/sessao.rs`) — acrescenta um `.fct` do disco ao desenho aberto (sem
substituir; `substituir: true` opcional). Hoje só aceita texto na conversa, e acrescentar algo grande
obriga a reabrir tudo.
- Onde: `crates/feathercad-mcp/src/mutacoes.rs` (ou onde está `cad_importar`), reusa `compacto::importar`.
- Teste: abrir A, importar B do disco → entidades = A + B; um Ctrl+Z desfaz só B.

☑ **1.2 Alinhamento de texto no `.fct`** (04/10; v: `b` base, `m` meio, `t` topo, `f` fundo — o motor tem `Baixo`, que o plano não previa) — sufixo opcional depois do conteúdo:
`T x y h giro "texto" c` (h: `e`/`c`/`d`; v: `b`/`m`/`t`, ex. `cm`). O motor já tem `AlinhaH/AlinhaV`.
- Onde: `compacto.rs` (`forma` → `"T"`, e o `escrever` correspondente).
- Teste: ida e volta `.fct` ⇄ desenho mantém o alinhamento; texto antigo sem sufixo continua esquerda/base.

☑ **1.3 Propriedades de camada no `.fct`** (04/10; `esp=`, `linha=`, `transp=`) — `camada NOME #cor esp=0.35 linha=eixo [off] …`.
- Onde: `compacto.rs::camada_da_linha` + `CamadaLida` (+ escrever).
- Teste: abrir `.fct` com `esp`/`linha` → `cad_camadas listar` mostra os valores.

☐ **1.4 Largura de texto** — `cad_criar` devolve a caixa de cada texto criado; e
`cad_medir {texto, altura}` → largura. Mesma métrica de fonte do `quadro`/PDF (`fonte.rs`).

☐ **1.5 `cad_topografia`: onde vai o quadro** — `quadro_ponto: [x,y]` ou `incluir_quadro:false`
(já existe) + devolver as estatísticas no JSON sem desenhar.

☐ **1.7 `cad_pdf`: ordem e papéis misturados** — hoje `pdf.rs::folhas` ordena as folhas pela posição
(esquerda→direita, depois de cima para baixo) e `folhas:[i]` indexa essa lista ordenada, não a ordem de
inserção nem o número do carimbo. Em 01/10 as A1 (fileira de baixo) saíram intercaladas com as A0.
- ☑ (01/10) Ordem = número do campo `folha` do carimbo (`05/12` → 5; vazio = ordem de criação), depois os
  retângulos da camada `FOLHA` por posição (`pdf.rs::folhas` + `numero_da_folha`; testes em `tests/pdf.rs`).
  Vale para o app (Exportar PDF) e para o `cad_pdf`. Falta: `cad_folha listar` devolver o índice de PDF.
- `papel: "auto"` → cada página no papel da própria folha (A0 e A1 no mesmo arquivo). O `FOLHA` atual
  (1 unidade = 1 mm) gera página errada em desenho em metros: usar o papel e a escala do bloco `FOLHA-n`.

☐ **1.6 Hachura no DXF** — gravar HATCH de verdade (exige DXF R2000+, item 3 da fila do
`CLAUDE.md`). Até lá a decomposição atual segue.

---

## Nível 2 — documento técnico genérico (serve a qualquer disciplina)

☐ **2.1 `cad_tabela`** — `{origem:[x,y], escala?, titulo?, colunas:[{titulo, largura, alinhamento?}],
linhas:[[...]], altura_linha, altura_texto, blocos?: N (quebra em N colunas lado a lado), grade?: "todas"|"5em5"}`.
Medidas em **mm de papel** quando há `escala` (texto e linhas saem do tamanho certo na folha).
- Core: `tabela.rs` (só monta geometria a partir dos dados; quebra de linha por largura usa 1.4).
- Substitui: quadro de curvas, pontos notáveis, tabela de desvios (204 linhas), quadro de layers.
- Teste: 204 linhas em 4 blocos cabem na caixa pedida; texto centralizado na célula.

☐ **2.2 `cad_grafico`** — `{caixa:[x0,y0,x1,y1] (mm de papel), escala, eixo_x:{de, ate, escala, marcas, rotulo}, 
series:[{pontos:[[x,y]...], camada, estilo:"linha"|"cruz"|"degrau", rotulos?}], eixo_y:{de, ate, marcas, rotulo},
grade?, titulo}`. Vários gráficos empilhados com o mesmo eixo X = várias chamadas com a mesma escala.
- Core: `grafico.rs`.
- Substitui: os 5 diagramas da Prancha 2 (alinhamento, superelevação, ripagem, perfil, levante).

☐ **2.3 `cad_texto_bloco`** — parágrafos com quebra automática pela largura:
`{origem, largura, altura_texto, entrelinha, itens:[{tipo:"titulo"|"texto"|"formula"|"nota", texto}], colunas?: N}`.
Devolve onde terminou (y) para encadear.
- Substitui: o memorial (Prancha 4) e as notas das pranchas.

☐ **2.4 `cad_detalhe`** — `{janela (coords do modelo), escala_detalhe, folha|ponto, caixa (mm de papel),
titulo, malha?: passo}`: copia a região ampliada (blocos `DETALHE-n`, como `FOLHA-n`), com moldura,
título e malha de coordenadas **do modelo original** nas bordas. Atualizar = refazer o bloco.
- É o embrião do "viewport" que falta no app (fila do `CLAUDE.md`, item 0).
- Substitui: detalhes A e B da Prancha 1 (transformei ponto a ponto no Python).

☐ **2.5 `cad_malha_coordenadas`** — `{caixa, passo, rotulos:"E/N", cruzetas?}` e **`cad_norte`**
`{ponto, tamanho, tipo:"seta"|"rosa", quadricula?}`.

☐ **2.6 Ao longo de um eixo** — `cad_ao_longo {eixo: id | pontos, de?, ate?, ...}` com modos:
- `repetir`: um bloco/forma a cada `passo`, girado perpendicular ao eixo (dormentes, postes, estacas);
- `paralelas`: polilinhas a ±d (trilhos, bordos, meio-fio) — usa `deslocar.rs`;
- `marcas`: traços e rótulos a cada `passo` (estaca 20 m, km), com rótulo sempre legível (nunca de cabeça para baixo).
Distâncias pelo comprimento do eixo, não pela corda.

☐ **2.7 Linhas 3D projetadas** — no `.fct`: `L3 x y z x y z`, `P3 …` e um cabeçalho
`camera iso|obliqua|{azimute, elevacao, escala, origem}`; o motor projeta para 2D ao importar
(o DXF continua 2D). Alternativa mínima: `cad_projetar {pontos3d, camera}` → pontos 2D.
- Substitui: bloco 3D e perspectiva da curva (Prancha 3).

---

## Nível 3 — domínio viário/ferroviário (o maior peso)

Core novo: `crates/feathercad-core/src/viario/` (`alinhamento.rs`, `clotoide.rs`, `estaca.rs`,
`superelevacao.rs`, `greide.rs`, `via.rs`). Regras em `regras/ferrovia_metrica.json` (e
`ferrovia_larga.json`, `rodovia_*.json` depois).

☐ **3.1 `cad_alinhamento`** — `{pis:[[E,N]...], curvas:[{R, Ls}] (uma por PI interno),
estaca_inicial?, passo_estaca?:20, desenhar?:{eixo, estacas, km, notaveis, rotulos}, camada_base?}`.
- Conta: clotoide simétrica (θs, xs, ys pelas séries, p, k, Ts, E, Lc, Δ), integração do eixo,
  validação (Lc ≥ mín, tangente entre curvas ≥ 0), estaqueamento `n+xx,xx` e `km+mmm`.
- Devolve: por curva `{Δ, lado, R, Ls, Lc, Ts, E, θs, A, TS/SC/CS/ST:{estaca, km, E, N}}`, comprimento total,
  e um **id de alinhamento** guardado no desenho (para as próximas ferramentas consultarem por estaca).
- Consulta: `cad_alinhamento_ponto {id, estaca}` → E, N, azimute, curvatura, elemento (T/S/C).
- Teste: os números da C6 na tabela de referência.

☐ **3.2 `cad_alinhamento_ajustar`** — `{pontos_existentes, regras, raio_min?, arredondar_R?:5,
tolerancia_dp?, quebrar_compostas?:true}` → PIs e raios minimizando a ripagem (mínimos quadrados de
retas nas tangentes + círculo nas curvas, depois descida por coordenadas nos PIs com R arredondado).
- Devolve: o mesmo de 3.1 + ripagem por ponto, RMS e máxima.
- Detalhes que o Python precisou: juntar vértices de mesmo sentido ligados por segmento curto; dividir curva
  composta (quebra com tangente curta); excluir pontos marcados como suspeitos (4.3).
- Teste: ripagem RMS ≤ 0,19 m e máx ≤ 0,65 m no CSV de referência.

☐ **3.3 `cad_superelevacao`** — `{alinhamento: id, V, regras}` (regras: B, h_max, I_max, dh_dt,
rampa, psi, Ls_min, arredondamento de h).
- Conta por curva: `he = B·V²/(127R)`, h adotado, `I`, `anc = g·I/B`, `R_min = B·V²/(127(h_max+I_max))`,
  `V_max`, `L_min = max(h·v/dh_dt, h/rampa, anc·v/psi, Ls_min)`, rampa efetiva, flecha 20 m, excesso a V lento.
- Devolve: tabela por curva com ✔/✘ e o **memorial em texto** (os passos com os números),
  pronto para o `cad_texto_bloco`. Opção `Ls_auto: true` → realimenta o 3.1 com L_min.
- Teste: C6 → he 102,2 mm; h 100; I 2,2; anc 0,020; R_min 238,4; L_min 60,2 → Ls 70; V_max 78,8.

☐ **3.4 `cad_greide`** — `{alinhamento: id, perfil_existente:[[estaca, cota]...], regra:"so_levante"|"minimos_quadrados",
rebaixo_max?:0.02, tolerancia?:0.08, Rv?:5000, rampa_max?}` → PIVs, rampas, curvas verticais (Lv), e por
estaca: cota de projeto e levante/rebaixo.
- Teste: levante médio 0,087 m, máx 0,344 m, rebaixo ≤ 0,023 m.

☐ **3.5 `cad_via`** — `{alinhamento: id, bitola, B?, trilho:"TR-57"|…, dormente:{comprimento, largura, espaco},
de?, ate?, desenho:"planta"|"secao", superelevacao?: id}`.
- Planta: trilhos (bordas do boleto) e dormentes ao longo do eixo (usa 2.6).
- Seção tipo: trilho, dormente girado por h, lastro (espessura, ombro, talude), sublastro, cotas — em escala de papel.
- Perfis de trilho e dormente como **componentes** (`biblioteca/ferrovia/*.fct`), não no código.

☐ **3.6 `cad_tabela_estacas`** — `{alinhamento, superelevacao?, greide?, existente?, passo:20, so_curvas?,
colunas:[...]}` → as linhas prontas (estaca, km, elemento, h atual/corrigida, flecha medida/projetada,
ripagem, cotas, levante) para o `cad_tabela`, sem a IA copiar números.

☐ **3.7 Modelo `ferrovia` em `cad_modelos`** — o `cad_lote` das 4 pranchas (planta + detalhes, pontos
críticos, 3D, memorial) com `esquema` dos dados: `{nuvem, V, bitola, regras, folha, carimbo}`. Fecha o nível 3
quando **uma chamada de modelo** reproduz o projeto de 01/10.

---

## Nível 4 — nuvem de pontos mais inteligente

Core: `crates/feathercad-core/src/nuvem.rs` (ao lado de `topografia.rs`).

☐ **4.1 `cad_nuvem_secoes`** — `{caminho, eixo?: id | "auto", agrupar_m?:3}` → seções transversais
ao longo do eixo: estaca, afastamento e cota de cada ponto. Com `"auto"`: rastreia o eixo pela própria
nuvem (curva principal: avança, recentra pelo centroide, salta lacunas por cone).

☐ **4.2 `cad_nuvem_feicoes`** — `{caminho, tipo:"trilhos", bitola?:1.0}`: em cada seção acha o par a
B ± tolerância com cotas próximas → eixo existente (ponto médio), nivelamento transversal (diferença
de cota), bitola medida; e as estatísticas (mediana de B, lacunas, seções sem par). Depois:
`bordos`, `meio-fio`, `crista/pé de talude`.

☐ **4.3 `cad_nuvem_qualidade`** — cota fora da mediana local (> 0,3 m), lacunas > N m, pares com
cotas **idênticas em série** (suspeita de cota copiada — foi o que pôs em dúvida "via sem superelevação"),
densidade por trecho. Devolve a lista e marca no desenho (camada `INTERVENCAO`).

☐ **4.4 Encadear com o nível 3** — saída do 4.2 vai direto para `cad_alinhamento_ajustar` (eixo existente),
`cad_greide` (perfil), `cad_tabela_estacas` (h existente), sem a IA copiar os pontos.

---

## Ordem sugerida

1. **1.1 → 1.3** (formato) e **0.1 + 0.2** (modo projeto, `cad_ver` leve): tudo fica mais barato.
2. **2.1, 2.2, 2.4** (tabela, gráfico, detalhe): servem também à elétrica.
3. **3.1, 3.3, 3.5** (alinhamento, superelevação, via) + **2.6** (ao longo do eixo).
4. **4.2 + 4.3**, depois **3.2** e **3.4** (ajuste e greide).
5. **2.3, 2.7, 3.6, 3.7** (memorial, 3D, tabela por estaca, modelo).

**Pronto quando:** o projeto de 01/10 sai só por MCP, sem Python, e com os números da tabela abaixo.

---

## Valores de referência (projeto de 01/10/2026)

Entrada: `~/Documentos/FeatherCAD/levantamento_topografico.csv` (E,N,Z). Copiar para
`testes/amostras-terceiros/` antes de usar nos testes (perguntar ao usuário). Resultados completos do
Python: `~/Documentos/FeatherCAD/ferrovia_otimizacao/dados_calculo.json`. Os scripts que fizeram
as contas (para portar ao Rust e depois apagar) estão em `…/ferrovia_otimizacao/scripts_python_referencia/`:
`pares.py` (4.2), `ajuste.py`/`refina.py` (3.2), `geo.py` (3.1, 3.3), `perfil.py`/`dados.py` (3.4, 3.6),
`via.py` (3.5), `prancha1-4.py` (nível 2). Saídas: DXF/`.fcdz`/PDF na
mesma pasta.

| Grandeza | Valor |
| --- | --- |
| Pontos / seções / pares de trilho válidos | 1 237 / 332 / 298 (2 seções descartadas: est. 180+04,4 e 222+19,6) |
| B medido (eixo a eixo dos trilhos) | 1,075 m (mediana) |
| Comprimento do eixo projetado | 6 246,1 m, 12 curvas |
| Ripagem | RMS 0,182 m; máx 0,643 m |
| Levante | médio 0,087 m; máx 0,344 m; rebaixo máx 0,023 m |
| Regras usadas | V 65 km/h; h_max 100 mm; I_max 50 mm; dh/dt 30 mm/s; rampa 1:600; ψ 0,35 m/s³; Ls ≥ 30 m, múltiplo de 10; h arredondado a 5 mm para cima; Rv 5 000 m |
| C6 (crítica) | R 350; Δ 93,92° D; Ls 70; Lc 503,75; Ts 410,44; E 163,68; θs 5,730°; p 0,583; k 34,988; A 156,52 |
| C6 superelevação | he 102,2 mm; h 100; I 2,2 mm; anc 0,020 m/s²; R_min 238,4 m; L_min 60,2 m; V_max 78,8 km/h; flecha 20 m 142,9 mm |
| C6 estacas | TS 117+12,66; SC 121+02,66; CS 146+06,40; ST 149+16,40 |

Raios das 12 curvas: 875, 405, 415, 380, 1100, 350, 510, 500, 725, 720, 750, 1150 m.

**Atenção:** os limites acima são os usuais de bitola métrica, **não** conferidos no texto da
ABNT NBR 16961:2021 — conferir antes de virarem o `regras/ferrovia_metrica.json` de fábrica.
