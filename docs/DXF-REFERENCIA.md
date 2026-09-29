# Referência DXF (extraída do ezdxf, "DXF Internals")

Fonte: https://ezdxf.readthedocs.io/en/stable/dxfinternals/index.html

**Aviso sobre a fidelidade.** As seis primeiras páginas (marcadas 📋) vieram coladas
pelo usuário, na íntegra. As demais vieram de um leitor de páginas que devolve
**resumo**, não texto literal. Onde o resumo cortou os exemplos de código DXF, isso
está indicado com ⚠. Para implementar, abrir a URL da página marcada ⚠ e copiar o
exemplo original.

---

## 1. Estruturas básicas

### 📋 fileencoding.html — Codificação
- R2004 (AC1018) e anteriores: ASCII, codificação dada por `$DWGCODEPAGE` (padrão `ANSI_1252`).
- R2007 (AC1021) e posteriores: UTF-8.
- Caracteres fora da codificação vão como `\U+nnnn` (vale nas duas gerações).
- Mapa `$DWGCODEPAGE` → codec: ANSI_874 Tailandês, ANSI_932 Japonês, ANSI_936 Chinês simplificado (gbk),
  ANSI_949 Coreano, ANSI_950 Chinês tradicional, ANSI_1250 Europa Central, 1251 Cirílico,
  1252 Europa Ocidental, 1253 Grego, 1254 Turco, 1255 Hebraico, 1256 Árabe, 1257 Báltico, 1258 Vietnamita.

### 📋 dxftags.html — Tags
- Arquivo = pares de linhas: código de grupo (inteiro) e valor.
- Tipo do valor pelo código: 0–9 string; 10–39 ponto 3D (double); 40–59 double; 60–79 int16;
  90–99 int32; 100/102 string; 105 handle hex; 110–149 double; 160–169 int64; 170–179 int16;
  210–239 double; 270–289 int16; 290–299 bool; 300–309 texto; 310–319 binário em hex;
  320–369 ponteiros/handles; 370–389 int16; 390–399 handle; 400–409 int16; 410–419 string;
  420–429 int32; 430–439 string; 440–449 int32; 450–459 long; 460–469 double; 470–479 string;
  480–481 hard-pointer; 999 comentário; 1000–1009 string; 1010–1059 double; 1060–1070 int16; 1071 int32.
- Significados fixos: 0 início de entidade/tabela; 1 texto principal; 2 nome; 5 handle
  (DIMSTYLE usa 105); 6 tipo de linha; 7 estilo de texto; 8 layer; 9 nome de variável (só HEADER);
  10/20/30 X/Y/Z (Y e Z vêm logo após o X); 39 espessura; 40–48 floats; 49 valor repetido
  (precedido por um 7x com o tamanho); 50–58 ângulos em graus; 62 cor (ACI); 66 "entidades seguem"
  (INSERT/POLYLINE); 67 espaço (0 modelo, 1 papel); 70–78 inteiros/flags; 210/220/230 extrusão;
  330 handle do dono; 370 espessura de linha (mm×100); 420 cor verdadeira 0x00RRGGBB;
  430 nome de cor; 440 transparência.
- Strings: até 255 caracteres por segurança (R2000+ admite 2049). MTEXT e binário vêm em pedaços < 255.
- **XDATA** (códigos ≥ 1000): começa com `1001` (nome do app), segue a entidade. 1002 = `{`/`}`,
  1003 layer, 1004 binário (até 127 bytes por pedaço), 1005 handle, 1010–1033 pontos/vetores
  (1010 fixo; 1011 move/escala/gira; 1012 só desloca; 1013 só direção), 1040 real, 1041 distância,
  1042 fator de escala, 1070 int16, 1071 int32. Aqui a ordem e a repetição de códigos importam.
- **MTEXT multi-tag**: texto > 250 caracteres vem em pedaços de 250 em tags `3`; o último é tag `1`.
- **R13+**: marcadores de subclasse `100`; código repetido em subclasses diferentes (ordem importa, ex. ATTDEF 280);
  em LWPOLYLINE a tag de contagem deve ser a primeira da subclasse `AcDbPolyline` (AutoCAD exige).
- **Dicionário de extensão**: `102 {ACAD_XDICTIONARY` / `360 handle` / `102 }`.
- **Reatores persistentes**: `102 {ACAD_REACTORS` / `330 handle`… / `102 }`.
- **Dados de aplicação**: `102 {NOMEAPP` … `102 }` (ou `NOMEAPP}`); exceção: XRECORD trata 102 como código comum.
- **Objetos embutidos** (R2018; só ATTRIB, ATTDEF e MTEXT com colunas): começam com `101 Embedded Object`,
  vão no fim do objeto hospedeiro, aceitam qualquer código exceto 0.

### 📋 handles.html — Handles
- Handle = string hex única no arquivo (até 16 dígitos, em maiúsculas). Definição é sempre o código 5
  (DIMSTYLE: 105).
- R10–R12: opcional (`$HANDLING` = 1). R13+: obrigatório (e `$HANDLING` some).
- `$HANDSEED` deve ser maior que o maior handle usado, mas o AutoCAD ignora o valor. Não confiar.
- Ponteiros: soft-pointer (330–339), hard-pointer (340–349), soft-owner (350–359), hard-owner (360–369),
  arbitrário (320–329, não traduzido). Referência "hard" protege do PURGE. Objeto tem no máximo um dono.
- Dono típico: BLOCK e entidades complexas são hard-owners dos elementos; tabelas e dicionários são soft-owners;
  POLYLINE é dono de VERTEX/SEQEND; INSERT é dono de ATTRIB/SEQEND.
- XDATA `1005` = handle com semântica de soft-pointer; handle inválido no AUDIT vira `0`.

### 📋 filestructure.html — Estrutura do arquivo
- Seções entre `0 SECTION` / `2 NOME` e `0 ENDSEC`; arquivo termina em `0 EOF`.
- Ordem: **HEADER** (deve ser a primeira) → **CLASSES** (R13+) → **TABLES** → **BLOCKS** → **ENTITIES** →
  **OBJECTS** (R13+) → THUMBNAILIMAGE (opcional, ignorável) → ACDSDATA (R2013+, sem documentação).
- TABLES: LTYPE, LAYER, STYLE, VIEW, VPORT (várias entradas com o mesmo nome; `*ACTIVE` é a configuração
  atual), DIMSTYLE, UCS, APPID, BLOCK_RECORD (R13+).
- BLOCKS: `*Model_Space` e `*Paper_Space` são vazios; o conteúdo do modelo e do papel ativo fica em ENTITIES.
  Layouts de papel inativos ficam em blocos `*Paper_Spacennn`.
- **Mínimo R12 (AC1009)**: só `SECTION / ENTITIES / ENDSEC / EOF`.
- **Mínimo R13+**: HEADER (`$ACADVER`, `$HANDSEED`), CLASSES (pode ser vazia), TABLES (LTYPE com
  BYBLOCK/BYLAYER/CONTINUOUS; LAYER com `0`; STYLE com `STANDARD`; APPID com `ACAD`; DIMSTYLE com
  `STANDARD`; BLOCK_RECORD com `*MODEL_SPACE` e `*PAPER_SPACE`; VPORT/VIEW/UCS podem ser vazias),
  BLOCKS (`*MODEL_SPACE`, `*PAPER_SPACE`), ENTITIES (pode ser vazia), OBJECTS (DICTIONARY raiz com
  `ACAD_GROUP`).

### 📋 datamodel.html — Modelo de dados
- Todo objeto de banco tem handle; opcionais: XDATA, reatores, dicionário de extensão.
- **R12**: modelo = estrutura do arquivo; referências por **nome**; espaço por tag `67` (0/ausente = modelo,
  1 = papel); bloco = `BLOCK … ENDBLK`. R12 ainda é lido e gravado pelo AutoCAD.
- **R13+**: handles obrigatórios, tabela BLOCK_RECORD, seção OBJECTS em árvore de DICTIONARY a partir da raiz
  (deve ser o primeiro objeto). Objetos novos por versão: R13 (DICTIONARY, GROUP, IMAGEDEF, MLINESTYLE,
  XRECORD, SORTENTSTABLE…), R2000 (LAYOUT, ACDBPLACEHOLDER), R2004 (MATERIAL, PLOTSETTINGS, DIMASSOC…),
  R2007 (FIELD, VISUALSTYLE, TABLESTYLE, SECTION…), R2013 (GEODATA), R2018 (ACDBNAVISWORKSMODELDEF).
- Dicionários da raiz que valem a pena criar: ACAD_GROUP (obrigatório), ACAD_LAYOUT (obrigatório com
  vários layouts), ACAD_PLOTSTYLENAME (obrigatório, aponta para ACDBDICTIONARYWDFLT com `Normal`);
  ACAD_COLOR, ACAD_MATERIAL, ACAD_MLEADERSTYLE, ACAD_MLINESTYLE, ACAD_PLOTSETTINGS, ACAD_SCALELIST,
  ACAD_TABLESTYLE, ACAD_VISUALSTYLE podem apontar para dicionários vazios.

---

## 2. Seções — https://ezdxf.readthedocs.io/en/stable/dxfinternals/sections/index.html

### 📋 header_section.html
- Obrigatória desde R13, opcional até R12. Cada variável = `9 $NOME` seguido das tags de valor
  (ex.: `9 $ACADVER` / `1 AC1009`).

### 📋 classes_section.html
- Desde AC1015 (R13). Cada `CLASS` (sem handle): `1` nome DXF (pode repetir), `2` nome C++ (único),
  `3` aplicativo, `90` flags de proxy, `91` contador (R2004+, pode estar errado), `280` foi-proxy,
  `281` é-entidade. Chave de armazenamento correta: tupla (nome, nome C++).
- Só importa para produtos Autodesk; algumas entidades exigem a classe para o AutoCAD abrir.

### tables_section.html
- Cada tabela: `0 TABLE` / `2 NOME` / (R13+: `5` handle, `330 0`, `100 AcDbSymbolTable`) / `70` contagem
  (**não confiar**) / entradas / `0 ENDTAB`. R12: sem `330`/`100`; handle só se `$HANDLING` = 1.
- Tabelas: APPID, BLOCK_RECORD (só R13+), DIMSTYLE, LAYER, LTYPE, STYLE, UCS, VIEW, VPORT.

### blocks_section.html
- Contém todos os `BLOCK … ENDBLK`: blocos reutilizáveis (INSERT) e todos os layouts.
- Nomes: `*Model_Space` (`$MODEL_SPACE` no R12), `*Paper_Space` (`$PAPER_SPACE`), inativos `*Paper_Spacennnn`.

### entities_section.html
- Página vazia ("TODO") no ezdxf. Modelo e papel ativo ficam aqui.

### objects_section.html
- Árvore hierárquica a partir do dicionário de nomes (raiz). Nem tudo está na árvore: dicionários de extensão
  e XRECORDs de entidades gráficas também moram aqui.

---

## 3. Tabelas — https://ezdxf.readthedocs.io/en/stable/dxfinternals/tables/index.html

- **appid_table.html** — Nomes de aplicativo usados no XDATA. `ACAD` obrigatório; o AutoCAD rejeita
  arquivo com AppID não definido. Conhecidos: `ACAD`, `AcAecLayerStandard` (descrição de layer),
  `AcCmTransparency` (transparência de layer), `HATCHBACKGROUNDCOLOR`, `EZDXF`.
  R12: `APPID` com `2` nome e `70` flags. R2000+: `5` handle, `330` dono, `100 AcDbSymbolTableRecord`,
  `100 AcDbRegAppTableRecord`, `2`, `70`.
- **block_record_table.html** ⚠ — Cada layout e cada bloco tem uma entrada; é o hard-owner das entidades.
  Campos vistos em block_management: `2` nome, `70` unidades de inserção, `280` explodível, `281` escalável.
- **dimstyle_table.html** ⚠ — ~40 variáveis R12 (DIMASZ, DIMCLRD, DIMTXT…), ~20 acrescentadas em R13/14
  (precisão angular, unidades alternativas, estilo de texto), ~10 em R2000+ (movimento do texto, espessuras,
  arranjo do texto sem espaço). Antes do R2007 configurações estendidas iam em XDATA. Handle em `105`.
  Consultar a URL para a tabela completa das variáveis.
- **layer_table.html** ⚠ — `2` nome, `70` flags (congelada/bloqueada), `62` cor (**negativa = layer desligada**),
  `6` tipo de linha, `370` espessura, `390` handle de estilo de plotagem. Descrição e transparência ficam no
  XDATA. Sobrescrita por viewport: XRECORDs ligados ao dicionário de extensão da LAYER (transparência, cor,
  tipo de linha, espessura).
- **linetype_table.html** ⚠ — Todo tipo de linha usado precisa de entrada. Simples (padrão de traços; positivo =
  traço, negativo = vazio; ex. `CENTER` = 2.0, 1.25, -0.25, 0.25, -0.25); complexos com TEXTO (ex. `GASLEITUNG`)
  e com FORMA/SHAPE (ex. `GRENZE2`), estes só de R2000 em diante.
- **style_table.html** — `STANDARD` obrigatório. `40` altura fixa, `41` fator de largura, `50` ângulo oblíquo,
  `71` flags de geração (espelhamento), `3` arquivo de fonte (SHX/TTF), `4` big font. Metadados de fonte no
  XDATA de `ACAD`: `1000` família, `1071` flags (itálico = 0x1000000, negrito = 0x2000000).
- **ucs_table.html** — Vazia ("TODO") no ezdxf.
- **view_table.html** ⚠ — Vistas nomeadas; não afetam o desenho. `2` nome, `70` flags (bit 1: papel), `40`/`41`
  largura/altura, `10` centro (DCS), `11` direção, `12` alvo (WCS), `42` distância focal, `43`/`44` planos de
  corte, `50` giro, `71` modo. R2000+: `281` modo de renderização (0–6), `72` usa UCS.
- **vport_table.html** ⚠ — Configurações de viewport do modelo; várias entradas com o mesmo nome formam
  configuração múltipla; ativa = `*ACTIVE`. Coordenadas de tela normalizadas (0,0)–(1,1). R2000+ acrescenta
  `AcDbViewportTableRecord`, `281`, dados de UCS (110–112, 79) e XDATA de navegação.

---

## 4. Entidades — https://ezdxf.readthedocs.io/en/stable/dxfinternals/entities/index.html

- **dimension_entity.html** ⚠ — Internos da DIMENSION (pontos de definição, ligação com DIMSTYLE). O resumo
  não trouxe o conteúdo detalhado; abrir a URL.
- **mesh.html** ⚠ — MESH (R13/14): vértices, faces (n-gons por índice), arestas e "creases" por aresta.
  Crease igual ao nível de subdivisão impede subdividir a aresta. Exemplo de cubo com 8 vértices na URL.
- **mleader.html** ⚠ — MULTILEADER/MLEADER: usa cor "raw" e estruturas de contexto aninhadas (conteúdo MText
  ou bloco, definições de linha-guia). Não trouxe os códigos; abrir a URL.
- **mtext.html** — Sem OCS: usa `text_direction` (eixo x local) e `extrusion`; `rotation` existe mas o eixo x
  tem prioridade. Texto em tags `3`+`1` (255 bytes por linha, na prática ~249). Códigos inline: `^I` tab,
  `^J` quebra, `^M` ignorado; `%%c` Ø, `%%d` °, `%%p` ±; `\A` alinhamento 0–2; `\C` cor ACI, `\c` RGB;
  `\H` altura, `\W` largura; `\S` fração (divisores `^`, `/`, `#`); `\f`/`\F` fonte separada por `|`;
  `\p` parágrafo. Altura total do MTEXT não é calculável só pelo DXF (`rect_height` é raro). Fundo (R2007+):
  `90` tipo (0 off, 1 cor, 2 janela, 3 canvas, 16 moldura), `45` escala, `63` ACI, `421` cor verdadeira,
  `431` nome. Colunas: R2000–R2013 = MTEXTs encadeados com dados no XDATA; R2018+ = um MTEXT com objeto
  embutido. Tipos: estática, dinâmica auto-altura, dinâmica altura manual, sem colunas.

---

## 5. Objetos — https://ezdxf.readthedocs.io/en/stable/dxfinternals/objects/index.html
Página vazia ("TODO").

---

## 6. Estruturas de gerenciamento

### block_management.html
- Bloco = contêiner de entidades referenciado por INSERT; identificado por `2 nome` e ponto base `10`.
- Nome único e **sem distinção de maiúsculas**. Evitar bloco que contém INSERT de si mesmo (ciclos
  travam programas CAD). Nome duplicado: o AutoCAD funde com resultado imprevisível.
- **R12**: `BLOCK` com `8` layer, `2` nome, `70` flags, `10/20/30` base, `3` nome 2, `1` nome do xref
  (normalmente vazio); entidades; `ENDBLK`. Sem handle originalmente.
- **R13+**: BLOCK e ENDBLK têm handle, `330` dono = entrada da BLOCK_RECORD, marcadores `100`.
  BLOCK_RECORD é a estrutura primária de todos os layouts.

### layout_management.html ⚠
- Três tipos de espaço: modelspace, layouts de papel, blocos. Todos têm BLOCK e (R13+) BLOCK_RECORD.
- Nomes: `*Model_Space` (`$MODEL_SPACE` no R12), `*Paper_Space`, inativos `*Paper_Space0`, `*Paper_Space1`…
- Desde R2000: cada espaço precisa de um objeto **LAYOUT** em OBJECTS (subclasse PlotSettings: impressora,
  papel, margens, escala, rotação; subclasse `AcDbLayout`: nome, ordem da aba, limites, base, extensão, UCS;
  `330` BLOCK_RECORD; `331` viewport ativa).
- Layout de papel exige a VIEWPORT "principal" (ID=1) no seu bloco (centro e tamanho em unidades de papel,
  alvo/direção/lente, snap/grid, UCS, renderização). Exemplos completos de LAYOUT e VIEWPORT na URL.

### rendering_of_dxf_content.html — Notas de renderização
- **Cor**: a maioria das entidades herda do layer ou do bloco; ACI depende de tabela de cores por aplicativo.
  Entidade em layer `B` dentro de bloco usa a cor de `B` mesmo com o INSERT no layer `A`; **layer `0`** é
  especial (adota o layer do INSERT). BYBLOCK fora de bloco = branco (modelo) ou preto (papel).
- **Layers/ordem de desenho**: nomes sem distinção de maiúsculas (chaves em minúsculas); layer decide
  visibilidade, não ordem; um INSERT ocupa uma posição só na ordem (blocos não se intercalam).
- **TEXT**: âncora sempre à esquerda da linha de base (ATTRIB idem); ATTRIB aceita formatação.
- **MTEXT**: altura do caractere = altura de maiúscula; entrelinha padrão 5/3 da altura; quebra só em espaços;
  espaços repetidos entre linhas contam como um; palavra maior que a coluna transborda.
- **POINT**: estilo global por `$PDMODE`; pode escalar com a vista ou ter tamanho absoluto.
