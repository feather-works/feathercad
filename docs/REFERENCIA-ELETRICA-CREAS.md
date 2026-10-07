# Referência elétrica — projeto CREAS (DEINFRA/SC, 2017)

Estudado em 04/10/2026 a pedido do usuário, para o MCP/Claude **saber como um projeto elétrico profissional
brasileiro é feito** e repetir as convenções. Fonte: `~/Downloads/Projeto_Elétrico_CREAS-Model.pdf` (A1, 1:50,
AutoCAD 2014). Uma cópia vetorial (só para conferência) estava em `projeto_creas_copia.fcdz/.pdf/.dxf/.fct`
na raiz do repositório (removida em 07/10/2026; recuperável no histórico do git). **O objetivo não é copiar vetores: é refazer com `cad_*`, seguindo isto.**

## 1. Composição da prancha (tudo numa A1)

1. Planta baixa (1:50) com arquitetura em linha fina e elétrica por cima.
2. Legenda das indicações (siglas: VTT, ARC10000, ARC8500, VET, RJ45(1)/(2), Projetor) + **Legenda** dos símbolos,
   com a **altura de instalação** em cada um (1,10 m, 0,30 m, 2,20 m, 1,50 m, piso).
3. **Notas** numeradas (medidas das caixas de passagem em mm; eletroduto não cotado = ø3/4"; fiação não cotada =
   #2,5 mm²; pontos sem potência = 100 W; luminárias aterradas, deixar "alça" do PE quando não instaladas).
4. **Cores dos condutores** (texto): neutro azul-claro; fase 1 (R) preto; fase 2 (S) branco; fase 3 (T) vermelho;
   terra verde; retorno amarelo.
5. **Quadro de cargas (QD1)** e **quadro de demanda**.
6. **Unifilar do QD1** (QM1 de medição, DPS, DR, disjuntores).
7. **Entrada de energia em poste** (vistas frontal e lateral, lista de materiais com letras de identificação,
   notas) e **detalhe da armação secundária**.
8. **Carimbo** (órgão, projeto, características, autor, área, escala, data, desenhista, nº da folha).

## 2. Condutores no eletroduto (o que o usuário pediu para lembrar)

O eletroduto é uma **linha** (contínua no teto/parede; tracejada no piso; ao longo de qualquer curva). Em cima dela
vão **marcas perpendiculares curtas**, uma sequência por trecho:

| Condutor | Marca |
| --- | --- |
| **Fase** | traço reto que cruza a linha |
| **Neutro** | traço que cruza a linha **com uma bandeira (gancho) na ponta de cima** |
| **Retorno** (circuitos de luz com interruptor) | **meio traço** (só de um lado da linha); a **letra do comando** (a, b, c…) escrita ao lado |
| **Terra** (tomadas, TUE, emergência) | **meio traço com um travessão em T** no topo |

- Cada circuito que passa no trecho aparece como **par neutro+fase** (+ retorno se for luz comandada, + terra se
  tiver PE). Ex.: circuito de iluminação "1" = neutro, fase e retorno; circuito de tomada "17" = neutro, fase e T.
- **Em cima das marcas, o número do circuito**; **embaixo, a seção em mm²** (`1.5`, `2.5`) de aquele grupo.
  Vários circuitos no mesmo duto = **uma sequência só**, com todos os números em fila (`1 16 17 4 8`).
- Se a seção muda, é outro grupo (outro número embaixo). Seção não cotada = #2,5; duto não cotado = ø3/4".
- Diâmetro do duto escrito ao longo da linha (`ø1"`, `ø1.1/2"`). **Eletroduto de lógica/dados em azul**, de força em preto.
- No MCP isso é o **`cad_eletroduto`**: `condutores` = um item por circuito/tipo (fase, neutro, retorno, terra,
  `circuito`, `secao`); duto compartilhado = **uma chamada com todos os condutores**; `curva` faz o traço curvo;
  `posicao` (0–1) escolhe onde ficam as marcas. Em trechos longos chame de novo com outra `posicao`.

## 3. Pontos e rótulos na planta

- **Símbolos azuis** para todos os pontos (tomadas, luminárias, interruptores, caixas); **eletrodutos de força em
  preto**, **de lógica em azul**; arquitetura em preto fino. (Espessuras no papel: arquitetura 0,08–0,21 mm;
  eletroduto 0,17–0,21; símbolos/lógica 0,25; texto 2,5–2,9 mm; títulos ~7 mm.)
- Ponto de luz: círculo + **`-N-`** (circuito) + **potência** (`20W`, `2x32W`) + **letra do comando** (`a`, `b`…).
- Interruptor: símbolo + letra do ponto que comanda. Tomada: triângulo (cheio/vazio conforme a altura) com
  **número do circuito** e **potência** (`300`, `600W`, `1350W`).
- Ar-condicionado: `ARC8500 -14- 1350W`; ventilador: `VET -9- 130W`; rede lógica: `RJ45(2) -PT TE 3-`;
  chamada de grupo de tomadas lógicas: `4xCSU-4P` com `TE 7 a 8; 20 a 21`.
- Caixas de passagem numeradas e **dimensionadas** (`CX1 200 x 200 mm`, `CX3 150x150x80`); caixa de medição
  `CXA 300 x 300 x 400 mm`; `QM1`; `AT1` (aterramento); `RP` (rack); haste copperweld 5/8" x 2,40 m; cabo de
  cobre nu 35 mm².

## 4. Quadro de cargas (colunas, nesta ordem)

Circuito | Descrição | Esquema (`F+N` luz, `F+N+T` tomadas) | Método de instalação (`B1`) | V (220) |
Iluminação por potência (nº de pontos de 20 / 32 / 300 W) | Tomadas por potência (nº de pontos de 100 / 130 / 300 /
600 / 1350 / 1400 W) | Pot. total (VA) | Pot. total (W) | Fase (R/S/T) | Pot. R | Pot. S | Pot. T | FCT | FCA |
In' (A) | Seção (mm²) | Ic (A) | Disj. (A) | dV (%). Última linha **TOTAL** (contagem por coluna e `R+S+T`).

Conferido na tabela:
- **Pot. (W) = Σ nº de pontos × potência da coluna** (ex.: 5×100 + 2×130 + 4×300 = 1960 W).
- **Ic = capacidade de condução da tabela NBR 5410, método B1**: 1,5 mm² → 17,5 A; 2,5 mm² → 24 A.
- FCT 1,00; **FCA 0,60** (agrupamento) ou 0,70 / 1,00 conforme o circuito; corrente corrigida `Ic × FCT × FCA`
  (24 × 0,6 = 14,4 A).
- Nomes: **TUG** = tomada de uso geral; **TUE** = tomada de uso específico (ar-condicionado).
- Não decifrei com certeza `In'` e a coluna VA (dependem de fator de potência por tipo de carga): ao implementar,
  usar `I = P / (V · FP)` e documentar o FP.
- **Quadro de demanda**: uso específico 10,61 kVA a 100 %; iluminação+TUGs: 12,00 kVA a 100 % + 4,89 kVA a 50 %
  (= 2,45); **total 25,06 kVA**.

## 5. Unifilar do QD1 (como é desenhado)

QM1 (medição, 50 A / 12 kA) → disjuntor geral 50 A / 12 kA → **DPS 275 V – 40 kA** (ligado ao terra) → barramento →
uma linha por circuito: **disjuntor** (símbolo de contato) com a corrente (`10 A`, `16 A`) acima, **(potência em W)**
acima da linha, **fase** (R/S/T) e **nº do circuito + descrição** à direita, **seção** (`1.5`/`2.5`) embaixo.
**DR** (25 A) agrupa os circuitos de tomada/ar-condicionado (caixinha "DR" no ramal).

## 6. Arquitetura que aparece (e que o MCP não tinha)

Paredes, portas com arco, bancadas, **vaso sanitário, lavatório, pia de cozinha, tanque (área de serviço),
geladeira, fogão**. Criei peças em `biblioteca/arquitetura/` (04/10/2026): `vaso_sanitario`, `lavatorio`,
`pia_cozinha`, `tanque`, `geladeira`, `fogao`, `porta_80` — chamar com `cad_componente` (mm reais; `ampliacao`
= 0,02 para 1:50 com desenho em m… conferir a unidade do desenho).

## 7. Ordem de trabalho para refazer um projeto assim

1. Arquitetura (paredes, portas, louças) → 2. pontos (símbolos com circuito, potência, comando) →
3. eletrodutos com condutores (`cad_eletroduto`, um duto compartilhado = uma chamada) → 4. caixas de passagem e
quadro → 5. quadro de cargas e de demanda (`cad_tabela`) → 6. unifilar → 7. legenda, notas, cores dos condutores →
8. carimbo/folha (`cad_folha`) → 9. `cad_verificar {dominio:"eletrica", leiaute:true}` e conferir com `cad_ver`.

## 8. O que falta no MCP para isto ser rápido

Modelo de circuitos (carga, fase, FP, FCT/FCA, seção, disjuntor, dV) que **gere** quadro de cargas, demanda,
unifilar e rótulos; símbolos de tomada por altura, luminária fluorescente/halógena, caixa de passagem, rack, RJ45;
cota, chamada com seta, balão; atributos nos blocos e lista de materiais; ferramenta de parede com espessura e
aberturas (portas/janelas).
