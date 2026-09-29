/** O rodapé: as coordenadas do cursor e os interruptores (Grade, Orto, OSNAP). */

import type { Estado, Fundo, RespostaCursor, TipoDeSnap } from '../ponte/index.ts';
import { TIPOS_DE_SNAP } from '../ponte/tipos.ts';
import { Z } from './z.ts';
import { formatar, type Textos } from '../idiomas/index.ts';
import { ANGULOS_DO_ORTO } from './preferencias.ts';

export const RODAPE_CSS = `
.rodape { display: flex; align-items: center; gap: 6px; height: 30px; padding: 0 10px; background: var(--fundo-faixa); border-top: 1px solid var(--borda); }
.rodape .modelo { padding: 2px 10px; border-radius: 4px; background: var(--fundo-painel); border: 1px solid var(--borda); color: var(--texto); font-size: 11px; letter-spacing: 0.06em; }
.rodape .zoom { min-width: 48px; text-align: right; color: var(--texto-suave); font-variant-numeric: tabular-nums; }
.rodape select { background: var(--fundo-painel); color: var(--texto); border: 1px solid var(--borda); border-radius: 4px; padding: 2px 6px; font: inherit; cursor: pointer; }
.rodape select.angulo-orto { margin-left: -4px; padding: 2px 2px; }
.rodape .coordenadas { flex: 1; color: var(--texto-suave); font-variant-numeric: tabular-nums; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.rodape button { background: none; border: 1px solid transparent; border-radius: 4px; padding: 3px 10px; cursor: pointer; color: var(--texto-suave); white-space: nowrap; }
.rodape button:hover { background: var(--fundo-painel); border-color: var(--borda); color: var(--texto); }
.rodape button.ligado { background: var(--destaque-suave); border-color: var(--destaque); color: var(--texto); }

.rodape .osnap-grupo { position: relative; display: flex; align-items: center; }
.rodape .osnap-grupo > button.seta { margin-left: -6px; padding: 3px 6px; }
.rodape .osnap-menu { position: absolute; left: 0; bottom: 30px; z-index: ${Z.menu}; min-width: 210px; padding: 6px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 10px; box-shadow: var(--sombra); }
.rodape .osnap-menu h3 { margin: 4px 8px 6px; font-size: 11px; font-weight: 600; color: var(--texto-suave); }
.rodape .osnap-menu label { display: flex; align-items: center; gap: 8px; padding: 4px 8px; border-radius: 6px; cursor: pointer; color: var(--texto); }
.rodape .osnap-menu label:hover { background: var(--fundo-faixa); }
`;

export type NomeDeInterruptor = 'snap' | 'orto' | 'osnap';

export interface Rodape {
  el: HTMLElement;
  atualizar(e: Estado): void;
  coordenadas(r: RespostaCursor | null): void;
  /** O zoom como texto pronto (`100%`), que vem do motor. */
  zoom(texto: string): void;
}

/** Cria o rodapé; `aoAlternar` recebe o interruptor e o valor que o botão pede. */
export function criarRodape(t: Textos, aoAlternar: (nome: NomeDeInterruptor, ligado: boolean) => void, aoEscolherFundo: (f: Fundo) => void, aoEscolherAnguloDoOrto: (graus: number) => void, aoEscolherSnap: (modos: Partial<Record<TipoDeSnap, boolean>>) => void): Rodape {
  const el = document.createElement('footer');
  el.className = 'rodape';
  const modelo = document.createElement('span');
  modelo.className = 'modelo';
  modelo.textContent = t.rodape.modelo;
  const coords = document.createElement('span');
  coords.className = 'coordenadas';
  const zoom = document.createElement('span');
  zoom.className = 'zoom';
  const fundo = document.createElement('select');
  fundo.title = t.rodape.fundo;
  for (const [valor, rotulo] of [['nada', t.rodape.fundoNada], ['pontos', t.rodape.fundoPontos], ['grade', t.rodape.fundoGrade]] as const) {
    const o = document.createElement('option');
    o.value = valor;
    o.textContent = rotulo;
    fundo.append(o);
  }
  fundo.addEventListener('change', () => aoEscolherFundo(fundo.value as Fundo));
  el.append(modelo, coords, zoom, fundo);

  // O passo do Orto: 90° = só horizontal e vertical; 45°, 30°, 15°… travam nesses ângulos.
  const angulo = document.createElement('select');
  angulo.className = 'angulo-orto';
  angulo.title = t.rodape.anguloOrto;
  angulo.addEventListener('change', () => aoEscolherAnguloDoOrto(Number(angulo.value)));
  const listarAngulos = (atual: number): void => {
    const graus = [...ANGULOS_DO_ORTO] as number[];
    if (!graus.includes(atual)) graus.push(atual);
    angulo.replaceChildren(...graus.map((g) => Object.assign(document.createElement('option'), { value: String(g), textContent: `${g}°` })));
    angulo.value = String(atual);
  };
  listarAngulos(90);

  const botoes = new Map<NomeDeInterruptor, HTMLButtonElement>();
  let atual: Estado | null = null;
  for (const nome of ['snap', 'orto', 'osnap'] as const) {
    const b = document.createElement('button');
    b.textContent = t.rodape[nome];
    b.addEventListener('click', () => atual && aoAlternar(nome, !atual[nome]));
    botoes.set(nome, b);
    if (nome === 'osnap') el.append(grupoOsnap(b));
    else el.append(b);
    if (nome === 'orto') el.append(angulo);
  }

  /** O botão OSNAP com a seta (ou o botão direito) que abre a lista dos tipos de snap: cada um liga e desliga. */
  function grupoOsnap(botao: HTMLButtonElement): HTMLElement {
    const grupo = document.createElement('div');
    grupo.className = 'osnap-grupo';
    const seta = Object.assign(document.createElement('button'), { type: 'button', className: 'seta', textContent: '▾', title: t.rodape.osnapModos });
    let menu: HTMLElement | null = null;
    const fechar = (): void => {
      menu?.remove();
      menu = null;
      window.removeEventListener('pointerdown', foraDoMenu, true);
      window.removeEventListener('keydown', teclaNoMenu, true);
    };
    const foraDoMenu = (e: Event): void => {
      if (menu && !menu.contains(e.target as Node) && e.target !== seta) fechar();
    };
    const teclaNoMenu = (e: KeyboardEvent): void => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      fechar();
    };
    const abrir = (): void => {
      if (menu) return fechar();
      menu = document.createElement('div');
      menu.className = 'osnap-menu';
      menu.append(Object.assign(document.createElement('h3'), { textContent: t.rodape.osnapModos }));
      for (const tipo of TIPOS_DE_SNAP) {
        const caixa = Object.assign(document.createElement('input'), { type: 'checkbox', checked: atual?.osnap_modos[tipo] ?? false });
        caixa.addEventListener('change', () => aoEscolherSnap({ [tipo]: caixa.checked }));
        const rotulo = document.createElement('label');
        rotulo.append(caixa, t.rodape.snapTipos[tipo]);
        menu.append(rotulo);
      }
      grupo.append(menu);
      window.addEventListener('pointerdown', foraDoMenu, true);
      window.addEventListener('keydown', teclaNoMenu, true);
    };
    seta.addEventListener('click', abrir);
    botao.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      abrir();
    });
    grupo.append(botao, seta);
    return grupo;
  }

  return {
    el,
    atualizar(e) {
      atual = e;
      modelo.textContent = e.unidade ? `${t.rodape.modelo} · ${e.unidade}` : t.rodape.modelo;
      modelo.title = e.unidade ? formatar(t.unidade.dicaRodape, { nome: t.unidade.nomes[e.unidade] }) : t.unidade.semUnidade;
      for (const [nome, b] of botoes) b.classList.toggle('ligado', e[nome]);
      fundo.value = e.fundo;
      if (angulo.value !== String(e.orto_angulo)) listarAngulos(e.orto_angulo);
    },
    zoom(texto) {
      zoom.textContent = texto;
    },
    coordenadas(r) {
      coords.textContent = r ? `X ${r.texto_x}   Y ${r.texto_y}` : '';
    },
  };
}
