# feathercad-mcp — o FeatherCAD para uma IA

Servidor [MCP](https://modelcontextprotocol.io) do FeatherCAD. Conversa por **stdin/stdout** (uma mensagem JSON por linha),
**sem rede**: nada do desenho sai da máquina. Usa o mesmo motor do aplicativo.

## Instalar

Em outro cliente MCP, o comando é só o caminho do binário (sem argumentos).

## Como a IA ganha "noção de espaço e geometria"

Uma IA não enxerga coordenadas; então o servidor dá **olhos, réguas e mapa**:

| Ferramenta | O que dá |
| --- | --- |
| `cad_ver` | uma **imagem** (PNG): grade com números nas bordas, eixos x/y, barra de escala e o `#id` de cada entidade. Com `janela`, dá zoom |
| `cad_entidades` | medidas **exatas** (comprimento, área, centro, raio, ângulo, caixa) e busca por janela ou por proximidade (`perto`) |
| `cad_medir` | distância/ângulo entre pontos ou entidades; área e perímetro de um polígono |
| `cad_regioes` | as **áreas fechadas** que os traços formam (salas, peças), com área útil e os textos dentro |
| `cad_verificar` | folgas (extremidade que quase toca), duplicadas, comprimento zero |
| `cad_resumo` | unidade, extensão, contagens por tipo e camada — comece por aqui |

Convenção (repetida no `initialize`): X para a direita, Y para cima, ângulos em graus anti-horário a partir do eixo X,
medidas na unidade do desenho.

## Economizando tokens: o formato compacto `.fct`

DXF repete códigos de grupo em cada número; o `.fct` é uma linha por objeto:

```text
unidade mm
camada Paredes #FF0000
R 0 0 400 300              retângulo: canto, largura, altura [giro]
L 0 0 100 50               linha
P 0 0 100 0 ~0.5 100 50 f  polilinha (~b = arco até o próximo ponto; f = fechada)
C 50 25 10                 círculo: centro, raio
A 50 25 10 0 90            arco: centro, raio, ângulo inicial e final
T 10 10 2.5 0 "Sala"       texto: ponto, altura, giro, conteúdo
H solida 0 0 10 0 10 10    hachura
```

`cad_exportar` / `cad_importar` trocam esse texto com a IA; `cad_converter` converte arquivos no disco (DXF ⇄ `.fct`)
**sem** passar pela conversa. Blocos são abertos em formas soltas ao exportar.

## Desenhar e editar

`cad_criar` (linha por 2 pontos ou por ponto+distância+ângulo, retângulo, polígono, polilinha, círculo, arco, texto),
`cad_editar` (mover, copiar, girar, escala, espelhar, apagar, propriedades), `cad_camadas`, `cad_hachurar` (região fechada
por linhas soltas também), `cad_desfazer`. Cada chamada valida tudo antes de tocar no desenho e vira **um** passo de desfazer.

## Lote: dados + laços feitos pelo servidor (`cad_lote`)

Em vez de centenas de chamadas (ou de um script Python), a IA manda os **dados** uma vez e o **molde** uma vez; o servidor faz o laço.
Tudo ou nada (falhou = nada aplicado, e o erro diz `passo [2].faca(volta 4)[0]`) e, se dá certo, **um** passo de desfazer.

```json
{ "dados": { "X": 300, "luzes": [[250,200,"a"],[175,600,"b"]] },
  "passos": [
    { "exigir": "=tamanho(luzes) > 0", "mensagem": "sem luzes" },
    { "para_cada": "luzes", "como": "l", "indice": "i", "faca": [
        { "ferramenta": "cad_simbolo",
          "argumentos": { "nome": "ponto_luz_teto", "ponto": "=[X + l[0], l[1]]", "comando": "=l[2]", "circuito": 1 } } ] } ] }
```

Passos: `ferramenta` (+ `argumentos`, `guardar` para usar a resposta depois), `definir`, `para_cada`, `repetir`, `se`, `exigir`.
Em qualquer valor: texto com `=` no começo é **expressão**; `${expr}` dentro de texto é interpolado. Expressões: `+ - * / %`,
comparações, `&& || !`, `c ? a : b`, `l[0]`, `c.campo`, `[x, y]`; listas de números operam elemento a elemento (pontos se somam).
Funções: `abs min max soma media arredondar piso teto raiz potencia sen cos tan atan2 dist angulo ponto_polar ponto_medio comprimento
deslocar intervalo tamanho texto numero virgula se juntar contem inverter soma_campo onde`. Sem `while` (não há laço infinito);
limites: 50 mil chamadas, 200 mil voltas, 16 níveis. Não entram: `cad_ver`, `cad_novo`, `cad_abrir`, `cad_desfazer`, `cad_importar` com `substituir`.

## Projeto elétrico (NBR 5444) — só quando o projeto for elétrico

Símbolos desenhados **uma vez** e chamados pelo nome, no tamanho certo de papel para a escala (`escala` = denominador, 50 = 1:50):

| Ferramenta | O que faz |
| --- | --- |
| `cad_simbolos` | lista a biblioteca: `tomada_baixa/media/alta/piso` (triângulo vazio/meio/cheio; `quantidade` 1-3 = simples/dupla/tripla), `ponto_luz_teto/embutido`, `arandela`, `ponto_luz_vapor_mercurio`, `interruptor_simples/paralelo/intermediario/duas_secoes`, `quadro_geral/parcial_embutido/aparente`, `caixa_telefone`, `caixa_medidor`, `caixa_passagem_piso`, `luz_multifilar`, `disjuntor`, `no_de_ligacao` |
| `cad_simbolo` | coloca um símbolo: `{nome, ponto, escala?, parede?|rotacao?, circuito?, comando?, potencia?, quantidade?}` (texto sempre na horizontal) |
| `cad_eletroduto` | eletroduto (contínuo no teto/parede, tracejado no piso) com os **traços de condutor** (fase cruza, neutro com bandeira, retorno meio traço, terra meio traço com T), números dos circuitos, seção e Ø (automático, ocupação 40 %) |

O guia `projeto_eletrico` (MCP `prompts/list` e `prompts/get`) ensina folha, simbologia, circuitos, fases, queda de tensão e multifilar; **as instruções gerais só apontam para ele quando o projeto é elétrico**.

## Segurança

Só abre e grava `.dxf`, `.fcdz` e `.fct`; não sobrescreve arquivo existente sem `sobrescrever: true`.

## Testes

`cargo test` (unitários + sessões MCP de verdade contra o binário) e `cargo clippy --all-targets`.
