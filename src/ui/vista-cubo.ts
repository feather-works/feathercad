/** O marcador de vista no canto da área de desenho (estilo ViewCube do AutoCAD). Em 2D é só a bússola
 * com a vista "de cima" e o sistema de coordenadas do mundo; ainda não responde a clique. */

import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const VISTA_CUBO_CSS = `
.vista-cubo { position: absolute; top: 12px; right: 12px; z-index: ${Z.paleta}; width: 92px; display: flex; flex-direction: column; align-items: center; gap: 4px; color: var(--texto-suave); font-size: 12px; user-select: none; cursor: default; }
.vista-cubo .bussola { position: relative; width: 92px; height: 92px; border-radius: 50%; background: color-mix(in srgb, var(--fundo-painel) 55%, transparent); }
.vista-cubo .ponto { position: absolute; font-weight: 600; }
.vista-cubo .n { top: 2px; left: 50%; transform: translateX(-50%); }
.vista-cubo .s { bottom: 2px; left: 50%; transform: translateX(-50%); }
.vista-cubo .w { left: 4px; top: 50%; transform: translateY(-50%); }
.vista-cubo .e { right: 4px; top: 50%; transform: translateY(-50%); }
.vista-cubo .face { position: absolute; inset: 26px; display: grid; place-items: center; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 3px; font-size: 11px; color: var(--texto); }
.vista-cubo .wcs { padding: 1px 8px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 3px; font-size: 11px; }
`;

export function criarVistaCubo(t: Textos): HTMLElement {
  const el = document.createElement('div');
  el.className = 'vista-cubo';
  el.title = t.vistaCubo.dica;
  const bussola = document.createElement('div');
  bussola.className = 'bussola';
  for (const [classe, texto] of [['n', t.vistaCubo.norte], ['s', t.vistaCubo.sul], ['w', t.vistaCubo.oeste], ['e', t.vistaCubo.leste]] as const) {
    const p = document.createElement('span');
    p.className = `ponto ${classe}`;
    p.textContent = texto;
    bussola.append(p);
  }
  const face = document.createElement('div');
  face.className = 'face';
  face.textContent = t.vistaCubo.topo;
  bussola.append(face);
  const wcs = document.createElement('div');
  wcs.className = 'wcs';
  wcs.textContent = 'WCS';
  el.append(bussola, wcs);
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick'] as const) el.addEventListener(ev, (e) => e.stopPropagation());
  return el;
}
