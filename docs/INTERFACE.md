# Interface do FeatherCAD — composição da tela

Registro do exemplo de referência enviado pelo usuário em 29/09/2026: a tela
do **AutoCAD LT 2026** com a aba **Home** ativa. Vai haver **várias sub-abas**
na Ribbon (Home é só a primeira). Este arquivo só anota; nada aqui está
implementado ainda.

## 1. Composição (de cima para baixo)

1. **Barra de título / acesso rápido**
   - Menu do aplicativo (Novo, Abrir, Salvar, Salvar como, Exportar, Imprimir).
   - Acesso rápido: novo, abrir, salvar, salvar como, imprimir, desfazer/refazer.
   - Título: produto + nome do arquivo (`*` = alteração não salva).
   - Busca de comando/ajuda. **Sem** conta, login, loja ou nuvem (regra 5 da
     suíte: privacidade por padrão).
2. **Ribbon** — abas principais (§2), cada uma com painéis (§3).
3. **Abas de arquivo** — aba `Início` (recentes, modelos), uma aba por
   desenho aberto (com `*` se sujo), botão `+` (desenho novo).
4. **Área de desenho (espaço Modelo)**
   - Cursor em cruz; entrada dinâmica junto ao cursor (prompt do comando
     ativo + coordenadas); ícone do sistema de coordenadas no canto.
   - Paletas laterais **acopláveis/flutuantes**: Gerenciador de Camadas
     (esquerda) e Paleta de Ferramentas (direita, com abas verticais).
5. **Linha de comando** — histórico + prompt do comando ativo, embaixo da
   área de desenho.
6. **Rodapé**
   - Esquerda: abas `Modelo` e as pranchas (layouts) + `+`.
   - Direita: interruptores `MODELO`, Grade (F7), Snap (F9), Orto (F8),
     Polar (F10), OSNAP (F3), Rastreio de OSNAP (F11), Entrada dinâmica
     (F12), Espessura de linha, escala de anotação, tela cheia.

## 2. Abas principais da Ribbon

| Aba | Conteúdo | Observação para o FeatherCAD |
| --- | --- | --- |
| **Início** (Home) | o dia a dia: Desenhar, Modificar, Anotação, Camadas, Bloco, Propriedades, Grupos, Utilitários, Área de transferência | **primeira a fazer** |
| **Inserir** | blocos, referências externas, imagem, PDF/DXF subjacente | só arquivo local |
| **Anotar** | cotas, texto múltiplo, chamadas múltiplas, tabelas, marcas de centro | |
| **Paramétrico** | restrições geométricas e dimensionais | fase tardia; solver é Rust |
| **Exibir** | viewports, ViewCube, barra de navegação, painéis, estilos visuais | |
| **Gerenciar** | personalizar interface, gravar macro de comando, limpar não usados (purge) | |
| **Saída** | plotar/PDF, plotagem em lote, configuração de página | |
| **Colaborar** | compartilhar/comparar | **só o que roda local**: comparar dois desenhos, exportar. Nada de nuvem |

A lista de sub-abas é aberta: entra aba nova sem mexer nas outras (§5).

## 3. Aba Início — painéis e botões

- **Desenhar:** Linha, Polilinha, Círculo, Arco; abaixo retângulo, elipse,
  hachura.
- **Modificar:** Mover, Girar, Aparar, Copiar, Espelhar, Concordância
  (fillet), Esticar, Escala, Matriz (array).
- **Anotação:** Texto, Cota, Linear, Chamada (leader), Tabela.
- **Camadas:** Propriedades de camada; lista da camada ativa; ligar/desligar,
  congelar, bloquear; "tornar atual" (pelo objeto), "igualar camada".
- **Bloco:** Inserir, Criar, Editar, Editar atributos.
- **Propriedades:** Cor, Espessura de linha, Tipo de linha (padrão
  `PorCamada`), Igualar propriedades.
- **Grupos:** Agrupar (sem criar bloco). **Utilitários:** Medir (distância,
  área, ângulo). **Área de transferência:** Colar, Copiar, Recortar.

## 4. Paletas

| --- | --- | --- |
| Botão da Ribbon | executa o comando; diz se está habilitado/ativo e o valor do dropdown (camada atual, cor…) | desenha o botão, manda o **nome** do comando |
| Cursor / entrada dinâmica | recebe o ponto do mouse, devolve o **ponto já resolvido** (snap/ortho/polar/rastreio), as guias a desenhar e o **texto pronto** (`[10931.976 , -37074.202]`) | desenha a mira, as guias e as caixinhas com o texto recebido |
| Interruptores do rodapé (Grade, Snap, Orto…) | guarda o estado e aplica no cálculo | botão liga/desliga = um comando; mostra o estado devolvido |
| Linha de comando | interpreta o texto, mantém o comando ativo, devolve o prompt e o histórico | caixa de texto e lista; manda a linha digitada |
| Seleção (janela, cruzamento, clique) | acha os ids sob o retângulo/ponto | traça o retângulo, manda os dois cantos |
| Camadas, blocos, propriedades | tudo | mostra as linhas do quadro |
| Abas de arquivo / `*` sujo | sabe se está sujo | desenha a aba |
| Abas Modelo/Pranchas | layouts e viewports | desenha as abas |
| Cores, espessuras, tipos de linha na tela | resolve `PorCamada`/`PorBloco` para o valor final | pinta com o valor final |

## 6. Ordem sugerida (proposta; confirmar antes de codar)

1. Esqueleto da janela com a composição da §1 vazia, Ribbon com a aba
   Início e os botões desabilitados (só a casca).
2. Motor: documento com camadas + entidade `LINE`; `linha_criar` e quadro.
3. Ferramenta Linha ponta a ponta (cursor resolvido → entrada dinâmica →
   linha de comando), com os três tráfegos da ponte e as métricas
   `[motor]`/`[ação]`/`[quadro]`.
4. Camadas (painel + lista da Ribbon), depois os demais painéis do Início.
5. Uma sub-aba nova por vez, cada uma descrita e confirmada antes.

## 7. Em aberto

- Nomes finais das sub-abas em pt-BR (acima estão como no AutoCAD).
- Quais sub-abas entram na primeira versão.
- Paletas acopláveis: só flutuantes no início, ou acoplar já?
- Ribbon recolhível (só abas) e barra de ferramentas clássica: fica para
  depois?
