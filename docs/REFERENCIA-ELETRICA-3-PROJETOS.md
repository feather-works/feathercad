# Referência elétrica — método de 3 projetos (Bombeiros-GO, Escola-RS, IPESAÚDE-SE)

Estudado em 04/10/2026 a pedido do usuário. **O conteúdo dos projetos não interessa; interessa como são
organizados e montados**, para repetir em outros projetos. Complementa `REFERENCIA-ELETRICA-CREAS.md` (marcas de
condutor no duto, rótulos). Fontes (PDF de plotagem do AutoCAD), copiadas em `~/Documentos/FeatherCAD/referencias/`:
`bombeiros_go.pdf` (2 A1), `escola_rs.pdf` (2 A3 em pé), `ipesaude_se.pdf` (1 A1), mais um `.dxf` por página
(conversão Inkscape: tudo vira polilinha, texto vira contorno; **só serve para olhar e medir, não para editar**).
Olhado de verdade: página 1 e 2 dos dois primeiros, página única do terceiro; camadas, cores e alturas de texto
extraídas dos arquivos.

## 1. Como cada um divide as folhas

| | Bombeiros-GO | Escola-RS | IPESAÚDE-SE |
| --- | --- | --- | --- |
| Papel | 2 × A1 (841×594) | 2 × A3 em pé | 1 × A1 |
| Escala | 1:50 (planta) | 1:75 (planta) | 1:50 |
| Folha 1 | Planta + notas (25) + legenda da fiação + código de cores + legenda de símbolos + tabela de fiação por circuito (26 quadrinhos) + 3 detalhes de vala/aterramento + revisões + carimbo | Só a planta e o carimbo | Tudo junto: planta, quadros, unifilar, detalhes, notas, advertência, legenda, carimbo |
| Folha 2 | Unifilar em cascata + 3 quadros de cargas + **lista de materiais** (52 itens, 3 colunas) | Quadro de cargas + unifilar + simbologia + observações + carimbo | — |

**Regra que se repete:** *planta sozinha numa folha quando o projeto é pequeno; documentação (quadros, unifilar,
lista, detalhes) na folha seguinte; em obra institucional grande a folha 1 já leva notas e legendas*. O quadro de
cargas **sempre** vem perto do unifilar. O carimbo fica no canto inferior direito (A1) ou na faixa de baixo (A3 em pé).
O carimbo diz "Folha n/N" e o **conteúdo da folha** ("Projeto Elétrico; Notas; Detalhes").

## 2. Carimbo (campos, na ordem em que aparecem)

Órgão/logo (brasão) → obra/projeto → local/endereço → proprietário (+ CNPJ) → projetista/responsável técnico (CREA,
ART, assinatura digital) → área construída → tamanho da folha → escala → data → revisão → nº da folha. IPESAÚDE
acrescenta cliente / obra / local / título / fase ("EXECUTIVO") / nome do arquivo. Tabela de **revisões** (nº, descrição,
data, visto) acima do carimbo (Bombeiros e IPESAÚDE). Rodapé com direitos autorais (Bombeiros).

## 3. Camadas (o que o AutoCAD deles tinha)

O PDF do Bombeiros veio **achatado** (sem camadas); os outros dois guardavam camadas e elas aparecem no DXF.

**IPESAÚDE — o esquema mais organizado (prefixo por disciplina):**
- `ARQ` arquitetura (cinza claro, ACI 8), `Base`, `Hachuras`, `DETALHES`.
- **`EL-`** elétrica: `EL-Tomadas`, `EL-Interruptor`, `EL-Lâmpada`, `EL-Quadro de distribuição`,
  `EL-Condutos (Teto)`, `EL-Condutos (Piso)`, `EL-Fios (Fase 1)`, `(Fase 2)`, `(Neutro)`, `(Terra)`,
  `EL-Textos fios`, `EL-Haste de aterramento`.
- **`DI-`** diagrama/documentação: `DI-Tabelas`, `DI-Linhas do diagrama`, `DI-Disjuntor`, `DI-Fios`,
  `DI-Textos do diagrama`, `DI-Pontilhado`.
- **`IFC-`** objetos vindos de BIM/fabricante (`IFC-Luminária-Contorno`, `IFC-Caixa de passagem-Contorno`).
- Símbolos: `SIMBOLO`, `P-Simbol`; textos: `TEXTO_R140`, `TXT060CW`.
- Cores (ACI da primeira aparição): terra verde (90), tomadas azul (150), interruptor azul-violeta (170), lâmpada
  vermelho-laranja (22), conduto de teto laranja (30), arquitetura cinza (8), tabelas/texto 7.

**Escola-RS — esquema mínimo:** `PAREDE`, `Esquadrias` (verde-água), `FO-Pilares`, `ELETRODUTO`, `Simbolos Elétrico`,
`COTAS`, `layer cortes`, `LOTES`, e as da folha com prefixo **`F-`**: `F-MOLDURA`, `F-SELO` (carimbo), `F-Vista 0.30`.

**Padrão a adotar** (junta os dois): `ARQ-*` (paredes, esquadrias, pilares, cotas) | `EL-*` (um por tipo de ponto,
conduto teto/piso e **um por condutor**: fase, neutro, retorno, terra) | `DI-*` (unifilar e tabelas) | `F-*`
(moldura, selo/carimbo, legenda, notas, revisões). Arquitetura **sempre cinza fino atrás**, elétrica colorida por cima.

## 4. Como a planta é desenhada

- Arquitetura em traço fino e claro; elétrica **por cima, mais grossa/colorida** (IPESAÚDE: eletroduto de teto laranja,
  de piso tracejado, símbolos azuis, luminárias em círculo vermelho).
- **Bloco de luminária em 3 células** (Escola): retângulo dividido em *potência | circuito | comando* (`30W | 1 | a`),
  com a legenda explicando "1 potência, 2 circuito, 3 interruptor". Evita texto solto ao lado do ponto.
- Ponto de luz do Bombeiros/IPESAÚDE: círculo com X + **potência** e **circuito** escritos; ar-condicionado
  `ARC12000` + potência; tomada = triângulo (cheio = alta, vazio = baixa/média; a legenda diz a altura).
- Circuitos nomeados `C1, C2, C3` (Escola: iluminação, TUG geral, TUG copa) ou numerados `1…26` (os outros).
- Eletroduto não cotado e fiação não cotada vão **na nota**, não no desenho (ø3/4" PVC flexível, #2,5 mm²).
- **Tabela de fiação por circuito** (Bombeiros): um quadrinho por nº de circuito com as marcas de condutor
  (fase/neutro/retorno/terra) e a seção — permite não repetir a conta em cada trecho.
- Detalhes comuns às três: **entrada de energia / poste / caixa de medição** (IPESAÚDE desenha em corte com lista),
  **vala de eletroduto enterrado** (mín. 900 mm de profundidade × 300 mm de largura), **esquema e malha de aterramento**
  (haste 2,40 m, cordoalha #16 mm², caixa de inspeção).

## 5. Quadros e diagramas — colunas

- **Quadro de cargas completo** (Bombeiros, IPESAÚDE): circuito | descrição | esquema (F+N+T…) | método de instalação
  (B1/B2) | tensão | **iluminação (W por classe de lâmpada)** | **tomadas (W por classe: 100/600/815/1085/1630/5400)** |
  pot. total (VA e W) | fases | pot. R / S / T | FCT | FCA | Ib | Ip | seção (mm²) | Ic | disjuntor (A) | dV parcial (%) |
  dV total (%) | **status OK**. Um quadro por quadro de distribuição (QD1, QD2, QM1/AL1) e linha **TOTAL**.
- **Quadro de cargas simples** (Escola): circuito | iluminação por potência (12/24/30) | força por potência (200/600) |
  carga W | bitola mm² | proteção A | fase RST; linha **TOTAL** com demanda em kW.
- **Quadro de demanda** (IPESAÚDE): tipo de carga | potência instalada | fator de demanda | demanda (kVA) | TOTAL.
- **Unifilar:** Escola = 1 quadro, disjuntores e linhas; Bombeiros = **cascata** `AL1 → QM1 (medição, DPS) → QD1 → QD2`
  com a potência do quadro entre parênteses, cada saída = disjuntor (A + kA) **+ DR onde tomada/área molhada** + seção
  (mm²) embaixo + `(W) nº (descrição)` + letra da fase. IPESAÚDE: barramento vertical com `QM1 → QD1`, DPS, 19 saídas;
  coluna de fase R/S/T desbalanceada de propósito para equilibrar a carga.
- **Lista de materiais** (Bombeiros): item | descrição (com **código de referência SINAPI**, ex. `AF_12/2015`) | un | qtd.
- **Notas** numeradas (Bombeiros 25; IPESAÚDE 17; Escola 1 linha de observação) e **código de cores** da fiação em texto
  (neutro azul-claro; fases preto/branco/vermelho; proteção verde/verde-amarelo; retorno amarelo).
- IPESAÚDE: caixa **ADVERTÊNCIA** (não trocar disjuntor/DR por outro de maior corrente) fixada nos quadros.

## 6. Tamanhos (papel, aproximados)

Texto corrente **2,5 mm** (IPESAÚDE, Bombeiros 2,5–2,8); rótulos pequenos 1,4–1,8; títulos de quadro 3,2–4,6; título
da planta/folha 4–7 mm. Escola usa letra maior (A3 em pé). Fontes vistas: Arial, Swiss 721, Technic/Calibri no carimbo.
Espessuras: arquitetura < elétrica < moldura; ver também as do CREAS.

## 7. O que isto cobra do MCP — estado em 04/10/2026

**Feito:** itens 1 (camadas por prefixo no `exemplos/luminotecnico_v3.py`), 2 (`luminaria_3_celulas`), 3 e 4 (`cad_circuitos`: quadro de cargas com queda de tensão, demanda e unifilar), 6 (`cad_carimbo` + `cad_folha`), notas normativas (`cad_notas_padrao`) e o cálculo luminotécnico (`cad_luminotecnico`). **Falta:** 5 (legenda automática dos símbolos usados), lista de materiais contada do desenho (hoje o script calcula), 8 (detalhes prontos na biblioteca), verificador elétrico em `cad_verificar`, unifilar multinível (QD → QDn) e DR/DPS desenhados.

Lista original:

1. **Camadas por prefixo** (`ARQ-/EL-/DI-/F-`) criadas por `cad_camadas` no começo de todo projeto elétrico.
2. **Bloco de luminária em 3 células** como componente (`cad_componente`), com texto parametrizado.
3. **Quadro de cargas completo** gerado pelo Rust a partir da lista de circuitos (potências, FCT/FCA, Ib, seção,
   disjuntor, **queda de tensão**, status) + **quadro de demanda** + totais por fase. Hoje a tabela é só desenho (`cad_tabela`).
4. **Unifilar em cascata** a partir da mesma lista (QM → QD → QDn, DPS, DR, seção, descrição).
5. **Lista de materiais** contada do próprio desenho (`cad_contar`) com a coluna de referência (SINAPI) editável.
6. **Carimbo com todos os campos** + **tabela de revisões** + brasão/logo opcional (`cad_folha` já tem campos).
7. **Legenda automática** só com os símbolos usados no desenho, com altura de instalação.
8. Detalhes prontos: entrada de energia, vala, aterramento (como `.fct` na biblioteca).
