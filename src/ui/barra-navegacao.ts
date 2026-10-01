/** A barra de navegação flutuante no canto da área de desenho: zoom, pan 3D, órbita e alternância de projeção. */

import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const BARRA_NAVEGACAO_CSS = `
.navegacao {
  position: absolute;
  top: 180px;
  right: 12px;
  z-index: ${Z.paleta};
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 2px;
  background: var(--fundo-painel);
  border: 1px solid var(--borda);
  border-radius: 5px;
  box-shadow: var(--sombra);
  cursor: default;
}
.navegacao button {
  width: 30px;
  height: 28px;
  background: none;
  border: 0;
  border-radius: 3px;
  color: var(--texto);
  cursor: pointer;
  font-size: 15px;
  line-height: 1;
  display: grid;
  place-items: center;
  transition: background 0.12s, color 0.12s;
}
.navegacao button:hover {
  background: var(--fundo-faixa);
  color: var(--destaque);
}
.navegacao button.ativo {
  background: color-mix(in srgb, var(--destaque) 25%, var(--fundo-painel));
  color: var(--destaque);
  border: 1px solid var(--destaque);
}
.navegacao .divisor {
  height: 1px;
  background: var(--borda);
  margin: 2px 0;
}
`;

export function criarBarraNavegacao(
  t: Textos,
  aoAproximar: () => void,
  aoAfastar: () => void,
  aoEnquadrar: () => void,
  aoAlternarOrbita?: () => void,
  aoAlternarPan?: () => void,
  aoAlternarProjecao?: () => void
): HTMLElement {
  const el = document.createElement('div');
  el.className = 'navegacao';

  const botoesBase = [
    { simbolo: '＋', dica: t.navegacao.maisZoom, acao: aoAproximar },
    { simbolo: '－', dica: t.navegacao.menosZoom, acao: aoAfastar },
    { simbolo: '⤢', dica: t.navegacao.ajustar, acao: aoEnquadrar },
  ];

  for (const { simbolo, dica, acao } of botoesBase) {
    const b = document.createElement('button');
    b.textContent = simbolo;
    b.title = dica;
    b.addEventListener('click', acao);
    el.append(b);
  }

  if (aoAlternarOrbita || aoAlternarPan || aoAlternarProjecao) {
    const div = document.createElement('div');
    div.className = 'divisor';
    el.append(div);

    if (aoAlternarOrbita) {
      const orb = document.createElement('button');
      orb.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M3.6 9h16.8"/><path d="M3.6 15h16.8"/><path d="M11.5 3a17 17 0 0 0 0 18"/><path d="M12.5 3a17 17 0 0 1 0 18"/></svg>`;
      orb.title = 'Órbita 3D (Girar vista em 3D)';
      orb.addEventListener('click', aoAlternarOrbita);
      el.append(orb);
    }

    if (aoAlternarPan) {
      const pan = document.createElement('button');
      pan.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 11V6a2 2 0 0 0-4 0v5"/><path d="M14 10V4a2 2 0 0 0-4 0v6"/><path d="M10 10.5V6a2 2 0 0 0-4 0v9.5a7.5 7.5 0 0 0 15 0V13a2 2 0 0 0-4 0v-2"/></svg>`;
      pan.title = 'Pan 3D (Deslocar vista)';
      pan.addEventListener('click', aoAlternarPan);
      el.append(pan);
    }

    if (aoAlternarProjecao) {
      const proj = document.createElement('button');
      proj.innerHTML = `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M23 7l-7 5 7 5V7z"/><rect x="1" y="5" width="15" height="14" rx="2" ry="2"/></svg>`;
      proj.title = 'Alternar Ortográfica / Perspectiva';
      proj.addEventListener('click', aoAlternarProjecao);
      el.append(proj);
    }
  }

  // O mouse sobre a barra não desenha nem mexe o cursor do desenho.
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick'] as const) {
    el.addEventListener(ev, (e) => e.stopPropagation());
  }

  return el;
}
