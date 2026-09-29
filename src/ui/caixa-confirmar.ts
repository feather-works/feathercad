/**
 * A caixa "Alterações não salvas": Salvar / Não salvar / Cancelar. É uma peça da
 * página (não a caixa nativa) para valer igual no aplicativo e no navegador, e
 * para os textos virem do arquivo de língua.
 */

import { formatar, type Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const CAIXA_CONFIRMAR_CSS = `
.confirmar-fundo { position: fixed; inset: 0; z-index: ${Z.menu}; background: var(--veu); display: flex; align-items: center; justify-content: center; }
.confirmar { width: 380px; max-width: 90vw; padding: 18px 20px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 6px; box-shadow: var(--sombra); }
.confirmar h2 { margin: 0 0 8px; font-size: 14px; }
.confirmar p { margin: 0 0 16px; color: var(--texto-suave); }
.confirmar .botoes { display: flex; justify-content: flex-end; gap: 8px; }
.confirmar button { padding: 5px 14px; background: var(--fundo-faixa); border: 1px solid var(--borda); border-radius: 3px; cursor: pointer; }
.confirmar button.principal { background: var(--destaque-suave); border-color: var(--destaque); }
`;

export type Resposta = 'salvar' | 'descartar' | 'cancelar';

/** Pergunta o que fazer com as alterações do desenho `nome`. Esc cancela. */
export function confirmarAlteracoes(t: Textos, nome: string): Promise<Resposta> {
  return new Promise((resolve) => {
    const fundo = document.createElement('div');
    fundo.className = 'confirmar-fundo';
    const caixa = document.createElement('div');
    caixa.className = 'confirmar';
    caixa.setAttribute('role', 'dialog');
    const titulo = document.createElement('h2');
    titulo.textContent = t.confirmar.titulo;
    const corpo = document.createElement('p');
    corpo.textContent = formatar(t.confirmar.corpo, { nome });
    const botoes = document.createElement('div');
    botoes.className = 'botoes';
    const fechar = (r: Resposta): void => {
      window.removeEventListener('keydown', aoTeclar, true);
      fundo.remove();
      resolve(r);
    };
    const aoTeclar = (e: KeyboardEvent): void => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      fechar('cancelar');
    };
    for (const [r, rotulo] of [['salvar', t.confirmar.salvar], ['descartar', t.confirmar.descartar], ['cancelar', t.confirmar.cancelar]] as const) {
      const b = document.createElement('button');
      b.textContent = rotulo;
      if (r === 'salvar') b.className = 'principal';
      b.addEventListener('click', () => fechar(r));
      botoes.append(b);
    }
    caixa.append(titulo, corpo, botoes);
    fundo.append(caixa);
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    (botoes.firstElementChild as HTMLElement | null)?.focus();
  });
}
