import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const VISTA_CUBO_CSS = `
.vista-cubo {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: ${Z.paleta};
  width: 155px;
  height: 180px;
  display: flex;
  flex-direction: column;
  align-items: center;
  user-select: none;
  cursor: default;
}
.vista-cubo .topo-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  justify-content: flex-start;
}
.vista-cubo .casa-btn {
  width: 24px;
  height: 24px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--fundo-painel) 88%, transparent);
  border: 1px solid var(--borda);
  color: var(--texto);
  display: grid;
  place-items: center;
  cursor: pointer;
  box-shadow: var(--sombra);
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  z-index: 10;
  flex-shrink: 0;
}
.vista-cubo .casa-btn:hover {
  background: var(--fundo-faixa);
  border-color: var(--destaque);
  color: var(--destaque);
}

.vista-cubo .coords-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px 6px;
  background: color-mix(in srgb, var(--fundo-painel) 92%, transparent);
  border: 1px solid color-mix(in srgb, var(--borda) 85%, var(--destaque) 15%);
  border-radius: 4px;
  color: var(--texto);
  font-family: var(--fonte-mono, monospace);
  font-size: 10px;
  font-weight: 600;
  box-shadow: var(--sombra);
  white-space: nowrap;
  pointer-events: none;
  z-index: 10;
  letter-spacing: -0.2px;
  max-width: 122px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.vista-cubo .bussola-cena {
  position: absolute;
  top: 30px;
  width: 140px;
  height: 120px;
  perspective: 550px;
  display: grid;
  place-items: center;
}

.vista-cubo .palco-3d {
  position: absolute;
  width: 0;
  height: 0;
  transform-style: preserve-3d;
  transition: transform 0.2s cubic-bezier(0.2, 0, 0, 1);
  display: flex;
  align-items: center;
  justify-content: center;
}

.vista-cubo .bussola-anel {
  position: absolute;
  width: 114px;
  height: 114px;
  left: -57px;
  top: -57px;
  border-radius: 50%;
  border: 3px solid color-mix(in srgb, var(--texto-suave) 30%, transparent);
  background: color-mix(in srgb, var(--fundo-painel) 45%, transparent);
  box-shadow: 0 4px 12px var(--sombra);
  transform-style: preserve-3d;
  transform: translateZ(-16px);
}

.vista-cubo .ponto {
  position: absolute;
  font-weight: 800;
  font-size: 10px;
  color: var(--texto-suave);
  cursor: pointer;
  text-shadow: 0 1px 2px var(--sombra);
  transition: color 0.12s, transform 0.12s;
}
.vista-cubo .ponto:hover {
  color: var(--destaque);
}
.vista-cubo .n { top: 2px; left: 50%; transform: translateX(-50%); }
.vista-cubo .s { bottom: 2px; left: 50%; transform: translateX(-50%); }
.vista-cubo .w { left: 4px; top: 50%; transform: translateY(-50%); }
.vista-cubo .e { right: 4px; top: 50%; transform: translateY(-50%); }

.vista-cubo .roll-btn {
  position: absolute;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--fundo-painel) 85%, transparent);
  border: 1px solid var(--borda);
  color: var(--texto-suave);
  display: grid;
  place-items: center;
  cursor: pointer;
  font-size: 11px;
  z-index: 12;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.vista-cubo .roll-btn:hover {
  background: var(--fundo-faixa);
  color: var(--destaque);
  border-color: var(--destaque);
}
.vista-cubo .roll-btn.left { top: 16px; left: 0; }
.vista-cubo .roll-btn.right { top: 16px; right: 0; }

.vista-cubo .cubo-3d {
  position: absolute;
  width: 58px;
  height: 58px;
  left: -29px;
  top: -29px;
  transform-style: preserve-3d;
  cursor: grab;
}
.vista-cubo .cubo-3d:active {
  cursor: grabbing;
}

.vista-cubo .face-3d {
  position: absolute;
  width: 58px;
  height: 58px;
  left: 1px;
  top: 1px;
  background: color-mix(in srgb, var(--fundo-painel) 94%, var(--texto) 6%);
  border: 1px solid color-mix(in srgb, var(--borda) 85%, var(--texto) 15%);
  box-shadow: inset 0 0 6px var(--sombra);
  border-radius: 3px;
  backface-visibility: hidden !important;
  overflow: hidden;
}

.vista-cubo .face-centro {
  position: absolute;
  inset: 10px;
  display: grid;
  place-items: center;
  font-size: 8.5px;
  font-weight: 800;
  letter-spacing: 0.3px;
  color: var(--texto);
  border-radius: 2px;
  cursor: pointer;
  transition: background 0.12s, color 0.12s, box-shadow 0.12s;
  z-index: 2;
}
.vista-cubo .face-centro:hover {
  background: color-mix(in srgb, var(--destaque) 75%, var(--fundo-painel));
  color: var(--fundo);
  box-shadow: 0 0 6px var(--destaque);
}

.vista-cubo .aresta-strip {
  position: absolute;
  background: transparent;
  cursor: pointer;
  transition: background 0.12s;
  z-index: 5;
}
.vista-cubo .aresta-strip:hover {
  background: color-mix(in srgb, var(--destaque) 80%, transparent);
}
.vista-cubo .aresta-t { top: 0; left: 10px; right: 10px; height: 10px; }
.vista-cubo .aresta-b { bottom: 0; left: 10px; right: 10px; height: 10px; }
.vista-cubo .aresta-l { left: 0; top: 10px; bottom: 10px; width: 10px; }
.vista-cubo .aresta-r { right: 0; top: 10px; bottom: 10px; width: 10px; }

.vista-cubo .quina-box {
  position: absolute;
  width: 10px;
  height: 10px;
  background: transparent;
  cursor: pointer;
  transition: background 0.12s;
  z-index: 6;
}
.vista-cubo .quina-box:hover {
  background: color-mix(in srgb, var(--destaque) 90%, transparent);
  box-shadow: 0 0 4px var(--destaque);
}
.vista-cubo .quina-tl { top: 0; left: 0; }
.vista-cubo .quina-tr { top: 0; right: 0; }
.vista-cubo .quina-bl { bottom: 0; left: 0; }
.vista-cubo .quina-br { bottom: 0; right: 0; }

.vista-cubo .face-3d.topo     { transform: translateZ(29px); }
.vista-cubo .face-3d.inferior { transform: rotateX(180deg) translateZ(29px); }
.vista-cubo .face-3d.frente   { transform: rotateX(-90deg) translateZ(29px); }
.vista-cubo .face-3d.tras     { transform: rotateX(90deg) translateZ(29px); }
.vista-cubo .face-3d.direita  { transform: rotateY(90deg) translateZ(29px); }
.vista-cubo .face-3d.esquerda { transform: rotateY(-90deg) translateZ(29px); }

.vista-cubo .wcs-wrap {
  position: absolute;
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  z-index: 10;
}
.vista-cubo .wcs-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  background: var(--fundo-painel);
  border: 1px solid var(--borda);
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  color: var(--texto);
  cursor: pointer;
  box-shadow: var(--sombra);
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}
.vista-cubo .wcs-btn:hover {
  border-color: var(--destaque);
  color: var(--destaque);
}

.vista-cubo .wcs-menu {
  position: absolute;
  bottom: 26px;
  right: -10px;
  width: 175px;
  background: var(--fundo-painel);
  border: 1px solid var(--borda);
  border-radius: 6px;
  box-shadow: 0 6px 16px var(--sombra);
  padding: 4px 0;
  display: flex;
  flex-direction: column;
  z-index: 25;
}
.vista-cubo .wcs-menu-grupo {
  padding: 3px 8px;
  font-size: 9.5px;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--texto-suave);
  border-bottom: 1px solid var(--borda);
  margin-bottom: 2px;
}
.vista-cubo .wcs-menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--texto);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.vista-cubo .wcs-menu-item:hover {
  background: var(--fundo-faixa);
  color: var(--destaque);
}
.vista-cubo .wcs-menu-item.ativo {
  font-weight: 700;
  color: var(--destaque);
}
`;

export interface VistaCuboController {
  el: HTMLElement;
  atualizar(rotacaoX: number, rotacaoZ: number): void;
  atualizarCoordenadas(x: number, y: number, z?: number): void;
  definirModoProjecao(perspectiva: boolean): void;
}

export function criarVistaCubo(
  t: Textos,
  aoMudarVista?: (rx: number, rz: number) => void,
  aoMudarProjecao?: (perspectiva: boolean) => void,
  aoMudarEstiloVisual?: (estilo: string) => void
): VistaCuboController {
  const el = document.createElement('div');
  el.className = 'vista-cubo';
  el.title = t.vistaCubo.dica;

  const topoBar = document.createElement('div');
  topoBar.className = 'topo-bar';

  const casaBtn = document.createElement('button');
  casaBtn.className = 'casa-btn';
  casaBtn.title = 'Vista Isométrica 3D (Home)';
  casaBtn.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9.5L12 3l9 6.5V20a1.5 1.5 0 0 1-1.5 1.5h-4.5v-6h-6v6H4.5A1.5 1.5 0 0 1 3 20V9.5z"/></svg>`;
  casaBtn.addEventListener('click', (ev) => {
    ev.stopPropagation();
    aoMudarVista?.(60, 45);
  });

  const coordsBadge = document.createElement('div');
  coordsBadge.className = 'coords-badge';
  coordsBadge.textContent = '0,000  0,000  0,000';
  coordsBadge.title = 'Coordenadas do Cursor (X, Y, Z)';

  topoBar.append(casaBtn, coordsBadge);

  const cena = document.createElement('div');
  cena.className = 'bussola-cena';

  const bussolaAnel = document.createElement('div');
  bussolaAnel.className = 'bussola-anel';

  // Roll 90° buttons
  const rollLeftBtn = document.createElement('button');
  rollLeftBtn.className = 'roll-btn left';
  rollLeftBtn.innerHTML = '↺';
  rollLeftBtn.title = 'Girar Vista 90° Anti-Horário';
  rollLeftBtn.addEventListener('click', (ev) => {
    ev.stopPropagation();
    aoMudarVista?.(rxAtual, (rzAtual - 90 + 360) % 360);
  });

  const rollRightBtn = document.createElement('button');
  rollRightBtn.className = 'roll-btn right';
  rollRightBtn.innerHTML = '↻';
  rollRightBtn.title = 'Girar Vista 90° Horário';
  rollRightBtn.addEventListener('click', (ev) => {
    ev.stopPropagation();
    aoMudarVista?.(rxAtual, (rzAtual + 90) % 360);
  });

  const pontosMap: Record<string, [number, number]> = {
    n: [90, 0],
    s: [90, 180],
    e: [90, 90],
    w: [90, 270],
  };

  const labels: Record<string, string> = {
    n: t.vistaCubo.norte,
    s: t.vistaCubo.sul,
    w: t.vistaCubo.oeste,
    e: t.vistaCubo.leste,
  };

  for (const [classe, texto] of Object.entries(labels)) {
    const p = document.createElement('span');
    p.className = `ponto ${classe}`;
    p.textContent = texto;
    p.title = `Vista ${texto}`;
    p.addEventListener('click', (ev) => {
      ev.stopPropagation();
      const [rx, rz] = pontosMap[classe]!;
      aoMudarVista?.(rx, rz);
    });
    bussolaAnel.append(p);
  }

  const cubo3d = document.createElement('div');
  cubo3d.className = 'cubo-3d';

  interface FaceDef {
    classe: string;
    rotulo: string;
    rx: number;
    rz: number;
    arestas: { pos: string; rx: number; rz: number; dica: string }[];
    quinas: { pos: string; rx: number; rz: number; dica: string }[];
  }

  const facesConfig: FaceDef[] = [
    {
      classe: 'topo',
      rotulo: t.vistaCubo.topo,
      rx: 0,
      rz: 0,
      arestas: [
        { pos: 'aresta-t', rx: 45, rz: 180, dica: 'Aresta Topo-Trás' },
        { pos: 'aresta-b', rx: 45, rz: 0, dica: 'Aresta Topo-Frente' },
        { pos: 'aresta-l', rx: 45, rz: 270, dica: 'Aresta Topo-Esquerda' },
        { pos: 'aresta-r', rx: 45, rz: 90, dica: 'Aresta Topo-Direita' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: 45, rz: 225, dica: 'Quina Topo-Trás-Esquerda' },
        { pos: 'quina-tr', rx: 45, rz: 135, dica: 'Quina Topo-Trás-Direita' },
        { pos: 'quina-bl', rx: 45, rz: 315, dica: 'Quina Topo-Frente-Esquerda' },
        { pos: 'quina-br', rx: 45, rz: 45, dica: 'Quina Topo-Frente-Direita' },
      ],
    },
    {
      classe: 'frente',
      rotulo: 'FRENTE',
      rx: 90,
      rz: 0,
      arestas: [
        { pos: 'aresta-t', rx: 45, rz: 0, dica: 'Aresta Frente-Topo' },
        { pos: 'aresta-b', rx: -45, rz: 0, dica: 'Aresta Frente-Inferior' },
        { pos: 'aresta-l', rx: 90, rz: 315, dica: 'Aresta Frente-Esquerda' },
        { pos: 'aresta-r', rx: 90, rz: 45, dica: 'Aresta Frente-Direita' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: 45, rz: 315, dica: 'Quina Frente-Topo-Esquerda' },
        { pos: 'quina-tr', rx: 45, rz: 45, dica: 'Quina Frente-Topo-Direita' },
        { pos: 'quina-bl', rx: -45, rz: 315, dica: 'Quina Frente-Inferior-Esquerda' },
        { pos: 'quina-br', rx: -45, rz: 45, dica: 'Quina Frente-Inferior-Direita' },
      ],
    },
    {
      classe: 'direita',
      rotulo: 'DIREITA',
      rx: 90,
      rz: 90,
      arestas: [
        { pos: 'aresta-t', rx: 45, rz: 90, dica: 'Aresta Direita-Topo' },
        { pos: 'aresta-b', rx: -45, rz: 90, dica: 'Aresta Direita-Inferior' },
        { pos: 'aresta-l', rx: 90, rz: 45, dica: 'Aresta Direita-Frente' },
        { pos: 'aresta-r', rx: 90, rz: 135, dica: 'Aresta Direita-Trás' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: 45, rz: 45, dica: 'Quina Direita-Topo-Frente' },
        { pos: 'quina-tr', rx: 45, rz: 135, dica: 'Quina Direita-Topo-Trás' },
        { pos: 'quina-bl', rx: -45, rz: 45, dica: 'Quina Direita-Inferior-Frente' },
        { pos: 'quina-br', rx: -45, rz: 135, dica: 'Quina Direita-Inferior-Trás' },
      ],
    },
    {
      classe: 'tras',
      rotulo: 'TRÁS',
      rx: 90,
      rz: 180,
      arestas: [
        { pos: 'aresta-t', rx: 45, rz: 180, dica: 'Aresta Trás-Topo' },
        { pos: 'aresta-b', rx: -45, rz: 180, dica: 'Aresta Trás-Inferior' },
        { pos: 'aresta-l', rx: 90, rz: 135, dica: 'Aresta Trás-Direita' },
        { pos: 'aresta-r', rx: 90, rz: 225, dica: 'Aresta Trás-Esquerda' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: 45, rz: 135, dica: 'Quina Trás-Topo-Direita' },
        { pos: 'quina-tr', rx: 45, rz: 225, dica: 'Quina Trás-Topo-Esquerda' },
        { pos: 'quina-bl', rx: -45, rz: 135, dica: 'Quina Trás-Inferior-Direita' },
        { pos: 'quina-br', rx: -45, rz: 225, dica: 'Quina Trás-Inferior-Esquerda' },
      ],
    },
    {
      classe: 'esquerda',
      rotulo: 'ESQUERDA',
      rx: 90,
      rz: 270,
      arestas: [
        { pos: 'aresta-t', rx: 45, rz: 270, dica: 'Aresta Esquerda-Topo' },
        { pos: 'aresta-b', rx: -45, rz: 270, dica: 'Aresta Esquerda-Inferior' },
        { pos: 'aresta-l', rx: 90, rz: 225, dica: 'Aresta Esquerda-Trás' },
        { pos: 'aresta-r', rx: 90, rz: 315, dica: 'Aresta Esquerda-Frente' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: 45, rz: 225, dica: 'Quina Esquerda-Topo-Trás' },
        { pos: 'quina-tr', rx: 45, rz: 315, dica: 'Quina Esquerda-Topo-Frente' },
        { pos: 'quina-bl', rx: -45, rz: 225, dica: 'Quina Esquerda-Inferior-Trás' },
        { pos: 'quina-br', rx: -45, rz: 315, dica: 'Quina Esquerda-Inferior-Frente' },
      ],
    },
    {
      classe: 'inferior',
      rotulo: 'INFERIOR',
      rx: -90,
      rz: 0,
      arestas: [
        { pos: 'aresta-t', rx: -45, rz: 0, dica: 'Aresta Inferior-Frente' },
        { pos: 'aresta-b', rx: -45, rz: 180, dica: 'Aresta Inferior-Trás' },
        { pos: 'aresta-l', rx: -45, rz: 270, dica: 'Aresta Inferior-Esquerda' },
        { pos: 'aresta-r', rx: -45, rz: 90, dica: 'Aresta Inferior-Direita' },
      ],
      quinas: [
        { pos: 'quina-tl', rx: -45, rz: 315, dica: 'Quina Inferior-Frente-Esquerda' },
        { pos: 'quina-tr', rx: -45, rz: 45, dica: 'Quina Inferior-Frente-Direita' },
        { pos: 'quina-bl', rx: -45, rz: 225, dica: 'Quina Inferior-Trás-Esquerda' },
        { pos: 'quina-br', rx: -45, rz: 135, dica: 'Quina Inferior-Trás-Direita' },
      ],
    },
  ];

  for (const f of facesConfig) {
    const faceEl = document.createElement('div');
    faceEl.className = `face-3d ${f.classe}`;

    const centroEl = document.createElement('div');
    centroEl.className = 'face-centro';
    centroEl.textContent = f.rotulo;
    centroEl.title = `Vista ${f.rotulo}`;
    centroEl.addEventListener('click', (ev) => {
      ev.stopPropagation();
      if (moveuArrasto) return;
      aoMudarVista?.(f.rx, f.rz);
    });
    faceEl.append(centroEl);

    for (const a of f.arestas) {
      const arestaEl = document.createElement('div');
      arestaEl.className = `aresta-strip ${a.pos}`;
      arestaEl.title = a.dica;
      arestaEl.addEventListener('click', (ev) => {
        ev.stopPropagation();
        if (moveuArrasto) return;
        aoMudarVista?.(a.rx, a.rz);
      });
      faceEl.append(arestaEl);
    }

    for (const q of f.quinas) {
      const quinaEl = document.createElement('div');
      quinaEl.className = `quina-box ${q.pos}`;
      quinaEl.title = q.dica;
      quinaEl.addEventListener('click', (ev) => {
        ev.stopPropagation();
        if (moveuArrasto) return;
        aoMudarVista?.(q.rx, q.rz);
      });
      faceEl.append(quinaEl);
    }

    cubo3d.append(faceEl);
  }

  const palco3d = document.createElement('div');
  palco3d.className = 'palco-3d';
  palco3d.append(bussolaAnel, cubo3d);

  cena.append(rollLeftBtn, rollRightBtn, palco3d);

  // WCS Menu Dropdown
  const wcsWrap = document.createElement('div');
  wcsWrap.className = 'wcs-wrap';

  const wcsBtn = document.createElement('div');
  wcsBtn.className = 'wcs-btn';
  wcsBtn.innerHTML = `<span>WCS</span><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>`;
  wcsBtn.title = 'Menu de Sistema de Coordenadas e Vistas (WCS/UCS)';

  const wcsMenu = document.createElement('div');
  wcsMenu.className = 'wcs-menu';
  wcsMenu.hidden = true;

  let modoPerspectiva = false;
  let estiloVisualAtual = 'arame';

  const fecharMenu = () => (wcsMenu.hidden = true);

  const criarItemMenu = (label: string, acao: () => void, comCheck = false, ativo = false) => {
    const item = document.createElement('div');
    item.className = `wcs-menu-item${ativo ? ' ativo' : ''}`;
    item.innerHTML = `<span>${label}</span>${comCheck && ativo ? '<span>✓</span>' : ''}`;
    item.addEventListener('click', (ev) => {
      ev.stopPropagation();
      acao();
      fecharMenu();
    });
    return item;
  };

  const renderizarMenu = () => {
    wcsMenu.innerHTML = '';

    // Grupo Sistemas
    const gSistemas = document.createElement('div');
    gSistemas.className = 'wcs-menu-grupo';
    gSistemas.textContent = 'Sistema de Coordenadas';
    wcsMenu.append(
      gSistemas,
      criarItemMenu('WCS (Global)', () => aoMudarVista?.(0, 0), true, true),
      criarItemMenu('Novo UCS por 3 Pontos', () => {})
    );

    // Grupo Vistas Padrão
    const gVistas = document.createElement('div');
    gVistas.className = 'wcs-menu-grupo';
    gVistas.textContent = 'Vistas Ortogonais';
    wcsMenu.append(
      gVistas,
      criarItemMenu('Topo (Top)', () => aoMudarVista?.(0, 0)),
      criarItemMenu('Inferior (Bottom)', () => aoMudarVista?.(-90, 0)),
      criarItemMenu('Frente (Front)', () => aoMudarVista?.(90, 0)),
      criarItemMenu('Trás (Back)', () => aoMudarVista?.(90, 180)),
      criarItemMenu('Esquerda (Left)', () => aoMudarVista?.(90, 270)),
      criarItemMenu('Direita (Right)', () => aoMudarVista?.(90, 90))
    );

    // Grupo Isométricas
    const gIso = document.createElement('div');
    gIso.className = 'wcs-menu-grupo';
    gIso.textContent = 'Vistas Isométricas';
    wcsMenu.append(
      gIso,
      criarItemMenu('Isométrica SO (SW)', () => aoMudarVista?.(60, 45)),
      criarItemMenu('Isométrica SE (SE)', () => aoMudarVista?.(60, 135)),
      criarItemMenu('Isométrica NE (NE)', () => aoMudarVista?.(60, 225)),
      criarItemMenu('Isométrica NO (NW)', () => aoMudarVista?.(60, 315))
    );

    // Grupo Projeção
    const gProj = document.createElement('div');
    gProj.className = 'wcs-menu-grupo';
    gProj.textContent = 'Modo de Projeção';
    wcsMenu.append(
      gProj,
      criarItemMenu(
        'Ortográfica (Paralela)',
        () => {
          modoPerspectiva = false;
          aoMudarProjecao?.(false);
        },
        true,
        !modoPerspectiva
      ),
      criarItemMenu(
        'Perspectiva',
        () => {
          modoPerspectiva = true;
          aoMudarProjecao?.(true);
        },
        true,
        modoPerspectiva
      )
    );

    // Grupo Estilos Visuais
    const gEstilos = document.createElement('div');
    gEstilos.className = 'wcs-menu-grupo';
    gEstilos.textContent = 'Estilo Visual';
    wcsMenu.append(
      gEstilos,
      criarItemMenu('Estrutura de Arame 2D', () => aoMudarEstiloVisual?.('arame'), true, estiloVisualAtual === 'arame'),
      criarItemMenu('Conceitual', () => aoMudarEstiloVisual?.('conceitual'), true, estiloVisualAtual === 'conceitual'),
      criarItemMenu('Sombreado com Arestas', () => aoMudarEstiloVisual?.('sombreado'), true, estiloVisualAtual === 'sombreado'),
      criarItemMenu('Realista', () => aoMudarEstiloVisual?.('realista'), true, estiloVisualAtual === 'realista')
    );
  };

  wcsBtn.addEventListener('click', (ev) => {
    ev.stopPropagation();
    const abrindo = wcsMenu.hidden;
    if (abrindo) {
      renderizarMenu();
      wcsMenu.hidden = false;
    } else {
      fecharMenu();
    }
  });

  window.addEventListener('click', fecharMenu);

  wcsWrap.append(wcsBtn, wcsMenu);
  el.append(topoBar, cena, wcsWrap);

  let rxAtual = 0;
  let rzAtual = 0;
  let arrastando = false;
  let moveuArrasto = false;
  let inicioX = 0;
  let inicioY = 0;
  let rxIni = 0;
  let rzIni = 0;

  const iniciarArrasto = (ev: MouseEvent) => {
    if (ev.button !== 0) return;
    ev.stopPropagation();
    arrastando = true;
    moveuArrasto = false;
    inicioX = ev.clientX;
    inicioY = ev.clientY;
    rxIni = rxAtual;
    rzIni = rzAtual;

    const noMove = (mEv: MouseEvent) => {
      if (!arrastando) return;
      const deltaX = mEv.clientX - inicioX;
      const deltaY = mEv.clientY - inicioY;
      if (!moveuArrasto) {
        if (Math.hypot(deltaX, deltaY) < 5) return;
        moveuArrasto = true;
        palco3d.style.transition = 'none';
      }
      let nRz = (rzIni - deltaX * 0.6) % 360;
      if (nRz < 0) nRz += 360;
      let nRx = Math.max(-89, Math.min(89, rxIni - deltaY * 0.6));
      aoMudarVista?.(nRx, nRz);
    };

    const noUp = () => {
      arrastando = false;
      palco3d.style.transition = 'transform 0.2s cubic-bezier(0.2, 0, 0, 1)';
      window.removeEventListener('mousemove', noMove);
      window.removeEventListener('mouseup', noUp);
    };

    window.addEventListener('mousemove', noMove);
    window.addEventListener('mouseup', noUp);
  };

  palco3d.addEventListener('mousedown', iniciarArrasto);

  for (const ev of ['mousemove', 'wheel', 'dblclick'] as const) {
    el.addEventListener(ev, (e) => e.stopPropagation());
  }

  function atualizar(rx: number, rz: number): void {
    rxAtual = rx;
    rzAtual = rz;

    const em2D = Math.abs(rx) < 1e-3 && Math.abs(rz) < 1e-3;

    if (em2D) {
      palco3d.style.transform = `rotateX(0deg) rotateZ(0deg)`;
    } else {
      palco3d.style.transform = `rotateX(${rx}deg) rotateZ(${-rz}deg)`;
    }
  }

  function atualizarCoordenadas(x: number, y: number, z = 0): void {
    const fmt = (num: number) => {
      const dec = Math.abs(num) >= 1000 ? 1 : 2;
      return num.toLocaleString('pt-BR', { minimumFractionDigits: dec, maximumFractionDigits: dec });
    };
    coordsBadge.textContent = `${fmt(x)} ${fmt(y)} ${fmt(z)}`;
    coordsBadge.title = `X: ${x.toFixed(3)}  Y: ${y.toFixed(3)}  Z: ${z.toFixed(3)}`;
  }

  function definirModoProjecao(perspectiva: boolean): void {
    modoPerspectiva = perspectiva;
  }

  return { el, atualizar, atualizarCoordenadas, definirModoProjecao };
}
