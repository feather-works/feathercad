/** A Ribbon: abas na fileira de cima, painéis da aba ativa embaixo. Só desenha `RIBBON`. */

import type { Estado } from '../ponte/index.ts';
import type { Textos } from '../idiomas/index.ts';
import { RIBBON } from './ribbon-dados.ts';
import type { WidgetDaRibbon } from './ribbon-widgets.ts';
import { obterIconeSVG } from './icones-svg.ts';
import { Z } from './z.ts';

export const RIBBON_CSS = `
.ribbon { position: relative; z-index: ${Z.ribbon}; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); }
.ribbon .abas { display: flex; gap: 2px; padding: 4px 8px 0; }
.ribbon .aba { background: none; border: 0; padding: 5px 12px; cursor: pointer; color: var(--texto-suave); border-radius: 3px 3px 0 0; }
.ribbon .aba.ativa { background: var(--fundo-painel); color: var(--texto); }
.ribbon .paineis { display: flex; overflow-x: auto; scrollbar-width: thin; padding: 4px 8px; background: var(--fundo-painel); }
.ribbon .painel { flex: none; display: flex; flex-direction: column; align-items: center; padding: 0 5px; border-right: 1px solid var(--borda); }
.ribbon .botoes { display: grid; grid-auto-flow: column; grid-template-rows: repeat(3, 26px); grid-auto-columns: max-content; gap: 0 1px; align-content: start; }
.ribbon .rotulo-painel { color: var(--texto-suave); font-size: 11px; white-space: nowrap; padding-top: 2px; }
.ribbon .botao { grid-row: span 3; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; min-width: 52px; max-width: 72px; padding: 2px 3px; line-height: 1.15; white-space: normal; background: none; border: 1px solid transparent; border-radius: 3px; cursor: pointer; font-size: 11.5px; text-align: center; }
.ribbon .botao .simbolo { font-size: 26px; line-height: 1; }
.ribbon .botao.pequeno { grid-row: span 1; flex-direction: row; justify-content: flex-start; gap: 6px; min-width: 0; max-width: none; padding: 0 4px; white-space: nowrap; }
.ribbon .botao.pequeno .simbolo { font-size: 15px; width: 18px; text-align: center; }
.ribbon .rotulo-painel .lancador { margin-left: 6px; padding: 0 3px; background: none; border: 0; cursor: pointer; color: var(--texto-suave); font-size: 11px; }
.ribbon .embreve { align-self: center; color: var(--texto-suave); padding: 0 12px; }
.ribbon .botao:disabled { opacity: 0.45; cursor: default; }
.ribbon .botao:hover:not(:disabled) { border-color: var(--borda); background: var(--fundo-faixa); }
.ribbon .botao.ativo { background: var(--destaque-suave); border-color: var(--destaque); }
`;

export interface Ribbon {
  el: HTMLElement;
  atualizar(e: Estado): void;
}

/** Cria a Ribbon; `aoComando` recebe o nome interno do comando do botão clicado. */
export function criarRibbon(t: Textos, aoComando: (comando: string) => void, aoAcao: (acao: string) => void, widgets: Record<string, () => WidgetDaRibbon>): Ribbon {
  const el = document.createElement('nav');
  el.className = 'ribbon';
  const abas = document.createElement('div');
  abas.className = 'abas';
  const paineis = document.createElement('div');
  paineis.className = 'paineis';
  el.append(abas, paineis);
  const botoes = new Map<string, HTMLButtonElement>();
  let vivos: WidgetDaRibbon[] = [];
  let ultimo: Estado | null = null;

  const mostrar = (id: string): void => {
    for (const b of abas.children) b.classList.toggle('ativa', (b as HTMLElement).dataset['aba'] === id);
    paineis.replaceChildren(...painelDaAba(id));
    if (ultimo) for (const w of vivos) w.atualizar(ultimo);
  };

  function painelDaAba(id: string): HTMLElement[] {
    botoes.clear();
    vivos = [];
    const aba = RIBBON.find((a) => a.id === id);
    if (aba && aba.paineis.length === 0) {
      const vazio = document.createElement('div');
      vazio.className = 'embreve';
      vazio.textContent = t.ribbon.embreve;
      return [vazio];
    }
    return (aba?.paineis ?? []).map((p) => {
      const painel = document.createElement('div');
      painel.className = 'painel';
      const linha = document.createElement('div');
      linha.className = 'botoes';
      for (const b of p.botoes) {
        if (b.widget) {
          const w = widgets[b.widget]?.();
          if (w) {
            w.el.classList.add('pequeno-widget');
            vivos.push(w);
            linha.append(w.el);
          }
          continue;
        }
        const textos = t.ribbon.botoes[b.id as keyof typeof t.ribbon.botoes];
        const botao = document.createElement('button');
        botao.className = b.pequeno ? 'botao pequeno' : 'botao';
        botao.title = textos.dica;
        botao.innerHTML = `<span class="simbolo"></span><span></span>`;
        const svgIcone = obterIconeSVG(b.id, b.simbolo);
        if (svgIcone.startsWith('<svg')) {
          botao.children[0]!.innerHTML = svgIcone;
        } else {
          botao.children[0]!.textContent = svgIcone;
        }
        botao.children[1]!.textContent = textos.rotulo;
        if (b.acao) {
          const acao = b.acao;
          botao.addEventListener('click', () => aoAcao(acao));
        } else if (b.comando === null) {
          botao.disabled = true;
        } else {
          const comando = b.comando;
          botao.addEventListener('click', () => aoComando(comando));
          botoes.set(comando, botao);
        }
        linha.append(botao);
      }
      const rotulo = document.createElement('div');
      rotulo.className = 'rotulo-painel';
      rotulo.textContent = t.ribbon.paineis[p.id as keyof typeof t.ribbon.paineis];
      if (p.lancador) {
        const acao = p.lancador;
        const lancador = document.createElement('button');
        lancador.className = 'lancador';
        lancador.textContent = '↘';
        lancador.title = t.propriedades.dica;
        lancador.addEventListener('click', () => aoAcao(acao));
        rotulo.append(lancador);
      }
      painel.append(linha, rotulo);
      return painel;
    });
  }

  for (const a of RIBBON) {
    const b = document.createElement('button');
    b.className = 'aba';
    b.dataset['aba'] = a.id;
    b.textContent = t.ribbon.abas[a.id as keyof typeof t.ribbon.abas];
    b.addEventListener('click', () => mostrar(a.id));
    abas.append(b);
  }
  mostrar(RIBBON[0]?.id ?? '');

  return {
    el,
    atualizar(e) {
      ultimo = e;
      for (const w of vivos) w.atualizar(e);
      for (const [comando, botao] of botoes) botao.classList.toggle('ativo', e.ferramenta === comando);
    },
  };
}
