# Funcionalidades do AutoCAD 2D — o que o FeatherCAD implementa

Avaliação de 29/09/2026 da lista enviada pelo usuário (13 áreas). **É
proposta, não decisão:** a coluna "Veredito" sugere; o usuário confirma antes
de cada fase. Critérios, em ordem: (1) cabe nas regras da suíte — Rust puro,
sem biblioteca C, **sem servidor, conta ou telemetria**; (2) o quanto o
desenhista 2D usa por dia; (3) custo (o que precisa existir antes).

Legenda: **F1** primeira versão útil · **F2** logo depois · **F3** fase
madura · **Talvez** só com pedido · **Não** fere uma regra da suíte ou não
compensa.

## 1. Geometria e primitivas

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Linha, círculo, arco, elipse, retângulo, polígono, ponto | **F1** | base do DXF; o motor já nasce com `Geometria` enumerável |
| Polilinha (`PLINHA`, `LWPOLYLINE`) e edição (`PE`: unir, fechar, inverter, espessura) | **F1** | é a entidade mais usada em planta; arcos por *bulge* como no DXF |
| Spline (ajuste e pontos de controle, NURBS) | **F2** | o DXF tem `SPLINE`; precisa avaliar B-spline no Rust |
| `XLINE` / `RAY` (linhas infinitas) | **F2** | fácil; o quadro precisa recortar na janela |
| `REGIAO` / `LIMITE` (contorno fechado → região) | **F2** | base de hachura, área e `MASSPROP` |
| Booleanas 2D em regiões (união, subtração, interseção) | **F3** | geometria computacional cuidadosa; crate próprio, testar bastante |
| `DIVIDIR` / `MEDIDA` | **F2** | depende de ponto e bloco |

## 2. Modificação

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Mover, copiar, girar, escala, espelhar, apagar | **F1** | o mínimo para desenhar |
| Aparar, estender, concordância, chanfro | **F1** | dependem de interseção robusta (`feathercad-core::geom`) |
| Deslocamento (`O`), esticar (`S`), alongar, quebrar, unir, explodir | **F1/F2** | deslocamento e esticar já em F1 |
| `BLEND` (spline tangente entre extremidades) | **F3** | depende de spline |
| Matrizes retangular/polar/caminho **associativas** | **F2** | matriz simples (copiar N vezes) em F1; a associativa (parâmetros vivos) exige entidade-derivada no documento, F2 |
| `OVERKILL` (fundir colineares, apagar duplicadas) | **F2** | algoritmo puro no Rust; ótimo para limpar DXF de terceiros |

## 3. Restrições paramétricas

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Restrições geométricas (coincidente, paralela, perpendicular, tangente, fixa…) | **F3** | precisa de **solver** de restrições no Rust (tarefa grande e de risco); desenho tem que ter grafo de restrições persistido no `.fcdz` |
| Restrições dimensionais e tabela de parâmetros com fórmulas | **F3** | reaproveita o motor de fórmulas do FeatherSheet? **Não importar por caminho** (regra 3): se for útil, vira crate comum com nome e dono |

## 4. Blocos

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Bloco simples (criar, inserir, editar, explodir), `GBLOCO`, atributos (`ATTDEF`, edição) | **F1/F2** | bloco e inserir em F1; atributos em F2 |
| Blocos dinâmicos: estados de visibilidade, tabela de propriedades | **F3** | no DXF ficam em objetos proprietários; ler/preservar sem quebrar já em F2, **editar** só em F3 |
| Blocos dinâmicos: estiramento, alinhamento, flip | **F3** | idem |

## 5. Documentação: hachura e anotação

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Hachura (padrões, sólido, gradiente) e associatividade | **F2** | os DXF de `testes/amostras` já têm casos difíceis (`hatches_*`, `hatch_issues`) |
| Texto simples e múltiplo, estilos de texto | **F1** | fonte: começar com o que o sistema tem; SHX/TrueType do DXF é ponto de atenção |
| Cotas (linear, alinhada, angular, raio, diâmetro, arco, ordenada, base, contínua) e estilos | **F1/F2** | linear/raio/diâmetro/angular em F1; o resto em F2 |
| Linhas de chamada (`LE`, `MLINCH`) e tabelas | **F2** | |
| Escala anotativa | **F2** | a conta do tamanho final é do Rust; o quadro já leva o valor resolvido |

## 6. Gestão do desenho

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Camadas (criar, cor, tipo/espessura de linha, ligar, congelar, bloquear) | **F1** | painel + lista na Ribbon |
| Filtros de camada e **estados de camada** | **F2** | estados = instantâneo salvo no documento |
| Layouts/pranchas e viewports | **F2** | |
| **Substituições por viewport** (cor/espessura/tipo de linha por folha) | **F2/F3** | depende de viewport |

## 7. Coordenadas e precisão

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Entrada de coordenadas (absoluta, relativa `@`, polar `dist<ângulo`) | **F1** | absoluta e relativa já no motor; polar em F1 |
| OSNAP: extremidade, ponto médio, centro, interseção, perpendicular, tangente, próximo, quadrante | **F1** | extremidade já feita; o resto entra por comando |
| OSNAP: interseção aparente, centro geométrico, `M2P` | **F2** | |
| Rastreio (`OSTRACK`, F11) e polar (F10) | **F1** | polar e rastreio são cálculo de cursor: Rust (`cursor.rs`) |
| Filtros `.X` / `.Y` | **F2** | pedido de coordenada parcial; interpretador da linha de comando |

## 8. Referências externas e importação

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| `XREF` (Anexo e Sobreposição), recorte, desvanecimento | **F2/F3** | **só arquivo local** (`.dxf`/`.fcdz`); mesmo motor de leitura |
| **Importar PDF vetorial** (`PDFIMPORT`) | **Talvez** | a suíte tem o `openpdf` para ler PDF; em vez de duplicar, o FeatherCAD pediria ao FeatherPDF as primitivas — **via crate comum**, não por caminho relativo |
| **Comparar dois desenhos** (`DWGCOMPARE`) | **F3** | é local e útil (revisões); o motor diz o que foi adicionado/removido, a interface só pinta |

## 9. Dados, atributos e planilha

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Extração de quantitativos (`DX`) para tabela/CSV | **F2** | CSV local; `.xlsx` só se a suíte tiver crate de escrita de planilha |
| **Vínculo dinâmico com planilha** (`DATALINK`) | **Talvez** | a suíte já tem o FeatherSheet; o vínculo bidirecional exigiria contrato entre apps (formato `.fshz`, protocolo). Antes disso: **exportar/atualizar por pedido**, sem "tempo real" |
| Campos dinâmicos (`Fields`: área, escala, autor) | **F2** | o texto vira "fórmula" resolvida pelo Rust |
| Atributos de bloco (`ATTDEF`/`BATTMAN`) | **F2** | |

## 10. Análise geométrica

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| `DIST`, `ID`, `AREA`, `LISTA`, `MEDIRGEOM` | **F1** | comandos de consulta; **resultado é texto pronto do Rust** |
| `MASSPROP` (centroide, inércia, raios de giração) | **F2** | depois de `REGIAO`; fórmula fechada, muito testável |
| Medição contínua ("Quick") ao passar o mouse | **F3** | é o `cursor_mover` devolvendo mais um texto; cuidado com o orçamento do movimento |

## 11. Publicação e plotagem

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Plotar/exportar **PDF** e imagem | **F2** | via o mesmo caminho de exportação da suíte |
| Tabelas de estilos de plotagem (`CTB`/`STB`) | **F2** | ler CTB (formato simples) para espessura por cor |
| Conjuntos de folhas (`SSM`) e publicação em lote (`PUBLISH`) | **F3** | |

## 12. Georreferência

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Sistema de coordenadas geográficas e conversão (UTM/WGS84, `GEOGRAPHICLOCATION`) | **Talvez** | conta de projeção é do Rust; sem dependência C (`proj` é C) → implementar só UTM/WGS84 |
| **Imagem de satélite em tempo real** | **Não** | vai à internet por conta própria e manda a localização do desenho para um serviço externo. Fere a regra 5 (privacidade, conector só de mão única e por pedido). Se algum dia: **só por pedido explícito**, com aviso do que sai da máquina |

## 13. Automação

| Funcionalidade | Veredito | Nota |
| --- | --- | --- |
| Arquivos de script `.SCR` (sequência de comandos, em lote) | **F2** | é a linha de comando gravada em arquivo; quase de graça |
| Gravador de ações/macros | **F3** | |
| **AutoLISP/Visual LISP** | **Talvez (F3+)** | interpretador de LISP no Rust puro é viável, mas é um produto; o FeatherSheet já decidiu Python para macros ([[python-no-opensheet]]) — **decidir a linguagem de macro da suíte** antes de escolher |
| DIESEL | **Não** | linguagem de string do AutoCAD, sem uso fora dele |

## Resumo da proposta

- **F1:** linha/círculo/arco/polilinha/retângulo/texto, editar (mover, copiar,
  girar, escala, espelhar, aparar, estender, concordância, deslocar), camadas,
  consultas (dist, área, lista), cotas básicas, OSNAP/polar/rastreio, bloco
  simples, DXF ida e volta, atalhos.
- **F2:** hachura, spline, região, matriz associativa, atributos, `OVERKILL`,
  layouts/viewports, plotar PDF, scripts `.SCR`, campos, extração para CSV,
  `UCS`, `MASSPROP`.
- **F3:** restrições paramétricas (solver), blocos dinâmicos completos,
  booleanas, `SSM`/lote, comparar desenhos, macros/LISP.
- **Fora, por regra:** imagem de satélite em tempo real; nuvem/colaboração.

Ordem que vale a pena confirmar: **F1 inteira antes de qualquer F2**; e a
tabela de comandos por língua ganha as linhas conforme cada comando entra
(`ATALHOS.md`).

## Perguntas para o usuário

1. F1 assim mesmo, ou algo de F2 sobe (hachura? vem em quase toda planta)?
2. Macro/script: manter só `.SCR` (comandos), ou a suíte inteira usa a mesma
   linguagem de macro (Python, como no FeatherSheet)?
3. Georreferência: só UTM/WGS84 sem dependência, ou fora do escopo?
4. Importar PDF vetorial: pedir ao FeatherPDF (crate comum) ou fora?
