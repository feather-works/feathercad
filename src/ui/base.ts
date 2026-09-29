/** As cores e o esqueleto da janela. Componentes só usam as variáveis daqui. */

export const BASE_CSS = `
:root {
  /* Claro: o padrão. As cores são só estas; componente nenhum escreve cor solta. */
  --fundo: #dde2e9;
  --fundo-area: #ffffff;
  --fundo-faixa: #eef1f5;
  --fundo-painel: #f7f8fa;
  --borda: #c3cad4;
  --texto: #1d2733;
  --texto-suave: #5b6776;
  --destaque: #1a6fd1;
  --destaque-suave: #cfe2fb;
  --fonte: system-ui, "Segoe UI", Roboto, sans-serif;
  /* O desenho */
  --grade-linha: rgba(0, 0, 0, 0.07);
  --grade-principal: rgba(0, 0, 0, 0.17);
  --grade-ponto: rgba(0, 0, 0, 0.38);
  --cursor: #000000;
  --previa: #1a6fd1;
  --janela-dentro: #1a6fd1;
  --janela-toca: #1f9d3a;
  --guia: #1f9d3a;
  --osnap: #c98a00;
  --caixa-dinamica: #ffffff;
  --veu: rgba(20, 28, 40, 0.35);
  --sombra: 0 8px 30px rgba(20, 28, 40, 0.28);
  /* Ícones do gerenciador de camadas */
  --icone-ativo: #d99a00;
  --icone-frio: #2f86c9;
  --icone-bloqueio: #b8860b;
  --icone-ok: #2a9d4b;
  color-scheme: light;
}
[data-theme='dark'] {
  --fundo: #1f262e;
  --fundo-area: #212830;
  --fundo-faixa: #2b333d;
  --fundo-painel: #323b46;
  --borda: #444f5c;
  --texto: #dfe5ec;
  --texto-suave: #98a4b3;
  --destaque: #3b8fe0;
  --destaque-suave: #2a4e75;
  --grade-linha: rgba(255, 255, 255, 0.055);
  --grade-principal: rgba(255, 255, 255, 0.13);
  --grade-ponto: rgba(255, 255, 255, 0.28);
  --cursor: #ffffff;
  --previa: #9bd1ff;
  --janela-dentro: #4d9cf0;
  --janela-toca: #5fd07e;
  --guia: #66d17a;
  --osnap: #ffd84a;
  --caixa-dinamica: #0d1520;
  --veu: rgba(0, 0, 0, 0.5);
  --sombra: 0 8px 30px rgba(0, 0, 0, 0.55);
  --icone-ativo: #ffcf3a;
  --icone-frio: #62b4ee;
  --icone-bloqueio: #e0b040;
  --icone-ok: #5fd07e;
  color-scheme: dark;
}
/* Cor de destaque escolhida em Configurações: só troca estas duas variáveis. */
[data-acento='verde'] { --destaque: #0f8c57; --destaque-suave: #cfeedd; }
[data-theme='dark'] [data-acento='verde'], [data-theme='dark'][data-acento='verde'] { --destaque: #3fbf85; --destaque-suave: #1d4d38; }
[data-acento='roxo'] { --destaque: #7a4bb8; --destaque-suave: #e5d9f5; }
[data-theme='dark'] [data-acento='roxo'], [data-theme='dark'][data-acento='roxo'] { --destaque: #a77fe0; --destaque-suave: #3f2d63; }
[data-acento='laranja'] { --destaque: #c25e0a; --destaque-suave: #fbe0c8; }
[data-theme='dark'] [data-acento='laranja'], [data-theme='dark'][data-acento='laranja'] { --destaque: #f0913f; --destaque-suave: #5e3612; }
[data-acento='rosa'] { --destaque: #c02a6b; --destaque-suave: #f8d5e4; }
[data-theme='dark'] [data-acento='rosa'], [data-theme='dark'][data-acento='rosa'] { --destaque: #ee6fa3; --destaque-suave: #5d2140; }
[data-acento='grafite'] { --destaque: #4a5058; --destaque-suave: #dde0e4; }
[data-theme='dark'] [data-acento='grafite'], [data-theme='dark'][data-acento='grafite'] { --destaque: #a9b1bb; --destaque-suave: #3d444d; }
[data-fonte='serif'] { --fonte: Georgia, 'Times New Roman', serif; }
[data-fonte='mono'] { --fonte: ui-monospace, Consolas, monospace; }
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; background: var(--fundo); color: var(--texto); font: var(--corpo, 12px)/1.3 var(--fonte); overflow: hidden; }
[hidden] { display: none !important; }
.motor-antigo { margin: 40px; padding: 18px; white-space: pre-wrap; border: 1px solid var(--destaque); background: var(--fundo-painel); }
/* Coluna flexível, não grade: com a Ribbon escondida (página inicial) uma grade de
   linhas fixas empurraria o corpo para uma linha de altura automática. */
#app { display: flex; flex-direction: column; height: 100%; }
#app > * { flex: none; }
#app > .corpo { flex: 1 1 0; min-height: 0; display: grid; grid-template-rows: minmax(0, 1fr); grid-template-columns: minmax(0, 1fr); }
.desenho-wrap { position: relative; display: grid; grid-template-rows: minmax(0, 1fr) auto auto; min-height: 0; }
.inicio { height: 100%; }
button { font: inherit; color: inherit; }
`;
