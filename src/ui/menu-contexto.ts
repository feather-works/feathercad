/**
 * O menu do botão direito. **A lista vem do motor** (`menu_contexto`, `menu_contexto.rs`): ele
 * decide o que cabe em cada momento (comando em andamento, objetos selecionados, nada); aqui só
 * se desenha, se navega por teclado e se devolve o item escolhido. Ícones são só enfeite.
 */

import { formatar, texto } from '../idiomas/index.ts';
import type { ItemDeMenu } from '../ponte/tipos.ts';
import { Z } from './z.ts';

export const MENU_CONTEXTO_CSS = `
.menu-contexto { position: fixed; z-index: ${Z.menu}; min-width: 230px; max-width: 320px; padding: 6px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 12px; box-shadow: var(--sombra); animation: menu-contexto-entra 110ms ease-out; transform-origin: top left; }
@keyframes menu-contexto-entra { from { opacity: 0; transform: scale(.96); } }
@media (prefers-reduced-motion: reduce) { .menu-contexto { animation: none; } }
.menu-contexto .item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 7px 10px; background: none; border: 0; border-radius: 8px; color: var(--texto); font: inherit; text-align: left; cursor: pointer; }
.menu-contexto .item:hover:not(:disabled), .menu-contexto .item.foco:not(:disabled) { background: var(--destaque-suave); }
.menu-contexto .item:disabled { color: var(--texto-suave); opacity: .55; cursor: default; }
.menu-contexto .icone { flex: none; width: 18px; text-align: center; color: var(--destaque); font-size: 14px; }
.menu-contexto .rotulo { flex: 1; white-space: nowrap; }
.menu-contexto .atalho { color: var(--texto-suave); font-size: 11.5px; }
.menu-contexto .visto { flex: none; width: 18px; text-align: center; color: var(--destaque); font-weight: 700; }
.menu-contexto hr { margin: 5px 6px; border: 0; border-top: 1px solid var(--borda); }
`;

/** Enfeite por item (o que o item faz é do motor). */
const ICONES: Record<string, string> = {
  enter: '↵', cancelar: '✕', fechar: '◯', mover: '✥', copiar: '⧉', girar: '↻', escala: '⤢', espelhar: '⇋', esticar: '↔',
  matriz: '▦', recortar: '✂', copiar_area: '⎘', colar: '📋', criar_bloco: '▣', agrupar: '⬚', apagar: '🗑',
  desfazer: '↶', refazer: '↷', propriedades: '☰', zoom_ajustar: '⛶', configuracoes: '⚙',
  linha: '╱', polilinha: '⌇', circulo: '○', arco: '◠', retangulo: '▭', elipse: '⬭', hachura: '▨', texto: 'A', cota: '⟷',
};

export interface MenuContexto {
  el: HTMLElement;
  /** Abre em (x, y) da janela com estes itens. */
  abrir(x: number, y: number, itens: ItemDeMenu[]): void;
  fechar(): void;
}

function rotuloDe(i: ItemDeMenu): string {
  if (i.repetido) return formatar(texto(i.chave), { nome: texto(`menu.item.${i.repetido}`) });
  return texto(i.chave);
}

/** Cria o menu (escondido); `aoEscolher` recebe o item clicado (ou aceito com Enter). */
export function criarMenuContexto(aoEscolher: (i: ItemDeMenu) => void): MenuContexto {
  const el = document.createElement('div');
  el.className = 'menu-contexto';
  el.setAttribute('role', 'menu');
  el.hidden = true;
  el.addEventListener('contextmenu', (e) => e.preventDefault());
  let botoes: { b: HTMLButtonElement; item: ItemDeMenu }[] = [];
  let foco = -1;

  const marcarFoco = (i: number): void => {
    foco = i;
    botoes.forEach((x, j) => x.b.classList.toggle('foco', j === i));
  };
  const mover = (passo: 1 | -1): void => {
    const ativos = botoes.map((x, j) => (x.item.ativo ? j : -1)).filter((j) => j >= 0);
    if (ativos.length === 0) return;
    const pos = ativos.indexOf(foco);
    marcarFoco(ativos[(pos + passo + ativos.length) % ativos.length]!);
  };
  const fechar = (): void => {
    if (el.hidden) return;
    el.hidden = true;
    window.removeEventListener('keydown', aoTeclar, true);
    window.removeEventListener('mousedown', aoClicarFora, true);
    window.removeEventListener('wheel', fechar, true);
    window.removeEventListener('blur', fechar);
  };
  const escolher = (item: ItemDeMenu): void => {
    fechar();
    aoEscolher(item);
  };
  const aoTeclar = (e: KeyboardEvent): void => {
    if (e.key === 'Escape') fechar();
    else if (e.key === 'ArrowDown') mover(1);
    else if (e.key === 'ArrowUp') mover(-1);
    else if (e.key === 'Enter' && foco >= 0) escolher(botoes[foco]!.item);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };
  const aoClicarFora = (e: MouseEvent): void => {
    if (!el.contains(e.target as Node)) fechar();
  };

  return {
    el,
    fechar,
    abrir(x, y, itens) {
      fechar();
      el.replaceChildren();
      botoes = [];
      foco = -1;
      for (const item of itens) {
        if (item.separador) {
          el.append(document.createElement('hr'));
          continue;
        }
        const b = document.createElement('button');
        b.className = 'item';
        b.disabled = !item.ativo;
        b.setAttribute('role', item.marcado === undefined || item.marcado === null ? 'menuitem' : 'menuitemcheckbox');
        if (item.marcado !== null && item.marcado !== undefined) b.setAttribute('aria-checked', String(item.marcado));
        const icone = document.createElement('span');
        icone.className = item.marcado !== null && item.marcado !== undefined ? 'visto' : 'icone';
        icone.textContent = item.marcado !== null && item.marcado !== undefined ? (item.marcado ? '✓' : '') : (ICONES[item.id.split(':').pop()!] ?? '');
        const rotulo = document.createElement('span');
        rotulo.className = 'rotulo';
        rotulo.textContent = rotuloDe(item);
        b.append(icone, rotulo);
        if (item.atalho) b.append(Object.assign(document.createElement('span'), { className: 'atalho', textContent: item.atalho }));
        b.addEventListener('click', () => escolher(item));
        b.addEventListener('mouseenter', () => marcarFoco(botoes.findIndex((z) => z.b === b)));
        botoes.push({ b, item });
        el.append(b);
      }
      el.hidden = false;
      // Cabe na janela: se passaria da borda, abre para o outro lado.
      const { width, height } = el.getBoundingClientRect();
      el.style.left = `${Math.max(4, Math.min(x, window.innerWidth - width - 4))}px`;
      el.style.top = `${Math.max(4, y + height > window.innerHeight - 4 ? y - height : y)}px`;
      window.addEventListener('keydown', aoTeclar, true);
      window.addEventListener('mousedown', aoClicarFora, true);
      window.addEventListener('wheel', fechar, true);
      window.addEventListener('blur', fechar);
    },
  };
}
