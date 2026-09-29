# FeatherCAD: o que fazer certo desde o dia 1

O FeatherSheet "começou mal" (palavras do usuário, 29/09/2026): nasceu com o
motor em TypeScript e teve que migrar tudo para Rust depois; nasceu com o texto
de interface fixo em pt-BR no código e teve que migrar para arquivos de
língua; ligou as regras de tamanho de função só no papel. Cada item abaixo
é uma dor dessas, já resolvida no desenho do FeatherCAD. A receita geral está
em `../../RECEITA.md`; aqui é o que muda ou pesa mais num CAD.

## 1. Língua em JSON desde o primeiro commit

- `src/idiomas/pt-BR.json` (referência) + `en-US.json`, mesmo formato do
  FeatherSheet (`../feathersheet/src/idiomas/`): bloco `idioma` (configuração
  regional) + todos os textos. Língua nova = um arquivo + uma linha no
  `registro.ts`.
- **Zero texto de interface no código**, nem "provisório": botão, dica, prompt
  da linha de comando, mensagem de erro. Variável com `{nome}` + `formatar()`.
- Teste que acusa chave faltando/digitada errado, desde o primeiro texto.
- **Novidade do CAD — nome de comando por língua.** No AutoCAD o comando
  digitado muda com a língua (`LINE` / `LINHA`, alias `L`). O arquivo de língua
  traz `comandos: { linha: ["LINHA", "L", "LINE"] }`; **o Rust** recebe a tabela
  e interpreta a linha de comando (o TS só manda o texto digitado). O prompt
  do comando ("Especifique o primeiro ponto:") também sai do arquivo, mas
  quem decide *qual* prompt é o Rust, devolvendo a chave + variáveis.
- **O Rust nunca formata em pt-BR fixo.** Separador decimal, unidade
  (mm, pol, ft-pol), ângulo (graus, grados) e formato de data vêm da
  configuração regional do arquivo de língua (o FeatherSheet só descobriu
  isso depois: o Contábil escrevia `R$` fixo em qualquer língua).
- Nomes de camada, bloco e estilo **do usuário** ficam como ele digitou; nunca
  são traduzidos.

## 2. Motor 100% Rust desde a primeira linha

- Nenhuma cópia do desenho no TS; ele só guarda a janela de vista (zoom/pan) e
  pede o **quadro** dessa janela. Sem "dublê em TS" nos testes: teste passa pelo
  `.wasm` da ponte.
- **Nada por quadro pela ponte** (regra crítica). Num CAD isso pesa mais:
  arrastar o mouse dispara snap, guias e prévia do desenho **a cada
  movimento**. Isso é **um** comando (`cursor_mover`) que devolve ponto
  resolvido + guias + texto da entrada dinâmica + prévia, tudo junto. Nunca
  "qual entidade está aqui?" seguido de "qual o ponto médio dela?".
- Desenho grande (dezenas de milhares de entidades, como no exemplo com 148
  camadas): índice espacial no Rust (para seleção, snap e recorte da vista),
  e o quadro leva só o que cabe na janela, já em coordenadas de tela e com
  cor/espessura resolvidas (`PorCamada` → valor final).
- Desfazer/refazer é do Rust (histórico por comando, um gesto = um passo).
  O formato `.fcdz` já com Git embutido dá o histórico de versões do arquivo;
  o desfazer de sessão é outra coisa e não se confunde com ele.
- DXF: lê sem confiar na ordem das tags; grava na versão escolhida e avisa o
  que simplificou (ver `README.md`); teste de ida e volta com o corpus de
  `testes/`.

## 3. Qualidade ligada no dia 1

- `[lints.clippy] too_many_lines = "deny"` e `cognitive_complexity = "deny"`
  em cada crate no primeiro `Cargo.toml` (no FeatherSheet está só escrito).
- Os cinco portões (`tsc`, `npm test`, `cargo test`, `cargo clippy`,
  `npm run build`) antes de todo commit.
- Métricas `[motor]`/`[ação]`/`[quadro]` desde o primeiro comando; o
  `cursor_mover` em especial precisa de orçamento por movimento (meta a
  medir com arquivo real, não com desenho sintético).
- Todo bug vira teste que falha no código antigo. Corpus real (`testes/`)
  em vez de dado sintético.

## 4. Interface

- Ribbon, menu e barra de acesso rápido saem **da mesma fonte de dados**
  (no FeatherSheet foi preciso registrar em `menus.ts` E `ribbon.ts`, e o
  botão de macro sumiu por esquecer um dos dois). Sub-abas como dados.
- Nada de item de menu morto: comando não implementado não aparece.
- Um componente por arquivo com o seu CSS; `z-index` de uma escala com teste;
  sem CSS global. Detalhes da tela: `INTERFACE.md`.
- Configurações (formato de salvamento padrão, versão do DXF, unidades,
  língua) já nascem num só lugar, com as chaves de `localStorage` com prefixo
  `feathercad.` desde o início (sem migração futura).

## 5. Formato e nome

- Nome, extensão (`.fcdz`), `identifier` (`br.feathercad.app`), mime e
  associação de arquivo corretos no primeiro commit — o FeatherSheet
  precisou de uma renomeação inteira depois.
- Sem formato "de teste" gravado em disco: se o `.fcdz` mudar antes da
  primeira versão, muda; sem compatibilidade com o rascunho.

## 6. Em aberto (para decidir antes de codar)

- ~~Tabela de comandos por língua: no JSON da língua ou em arquivo próprio?~~
  **Decidido e feito (29/09):** arquivo próprio `src/idiomas/comandos.<código>.json`.
- Unidades do desenho (mm, m, pol) vivem no arquivo `.fcdz`/`$INSUNITS` e
  **não** na língua; só o *formato de exibição* vem da língua.
- Orçamento de tempo do `cursor_mover` (ms por movimento).
