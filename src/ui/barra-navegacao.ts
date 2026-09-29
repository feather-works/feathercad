/** A barra de navegação flutuante no canto da área de desenho: aproximar, afastar e enquadrar. */

import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const BARRA_NAVEGACAO_CSS = `
.navegacao { position: absolute; top: 138px; right: 12px; z-index: ${Z.paleta}; display: flex; flex-direction: column; gap: 1px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 3px; cursor: default; }
.navegacao button { width: 30px; height: 28px; background: none; border: 0; cursor: pointer; font-size: 16px; line-height: 1; }
.navegacao button:hover { background: var(--fundo-faixa); }
`;

/** Cria a barra; cada botão só repassa o pedido (a conta do zoom é do motor). */
export function criarBarraNavegacao(t: Textos, aoAproximar: () => void, aoAfastar: () => void, aoEnquadrar: () => void): HTMLElement {
  const el = document.createElement('div');
  el.className = 'navegacao';
  for (const [simbolo, dica, acao] of [['＋', t.navegacao.maisZoom, aoAproximar], ['－', t.navegacao.menosZoom, aoAfastar], ['⤢', t.navegacao.ajustar, aoEnquadrar]] as const) {
    const b = document.createElement('button');
    b.textContent = simbolo;
    b.title = dica;
    b.addEventListener('click', acao);
    el.append(b);
  }
  // O mouse sobre a barra não desenha nem mexe o cursor do desenho.
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick'] as const) el.addEventListener(ev, (e) => e.stopPropagation());
  return el;
}
