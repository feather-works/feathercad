/** A linha de comando: histórico + o prompt do comando ativo + a caixa onde se digita. */

import type { Entrada, Estado } from '../ponte/index.ts';
import { texto } from '../idiomas/index.ts';
import type { Textos } from '../idiomas/index.ts';

export const LINHA_COMANDO_CSS = `
.linha-comando { background: var(--fundo-faixa); border-top: 1px solid var(--borda); padding: 6px 10px 8px; font-family: ui-monospace, "JetBrains Mono", Consolas, monospace; font-size: 12px; }
.linha-comando .historico { height: 3.9em; overflow-y: auto; color: var(--texto-suave); line-height: 1.5; padding-bottom: 4px; scrollbar-width: thin; }
.linha-comando .escolhas { display: flex; flex-wrap: wrap; gap: 6px; max-height: 4.6em; overflow-y: auto; padding: 4px 0; }
.linha-comando .escolhas button { background: var(--fundo-painel); color: var(--texto); border: 1px solid var(--borda); border-radius: 999px; padding: 1px 12px; font: inherit; cursor: pointer; }
.linha-comando .escolhas button:hover { border-color: var(--destaque); background: var(--destaque-suave); }
.linha-comando .entrada { display: flex; gap: 8px; align-items: center; background: var(--fundo-area); border: 1px solid var(--borda); border-radius: 6px; padding: 0 6px 0 10px; }
.linha-comando .entrada:focus-within { border-color: var(--destaque); box-shadow: 0 0 0 2px var(--destaque-suave); }
.linha-comando .entrada::before { content: '\\276F'; color: var(--destaque); }
.linha-comando .prompt { color: var(--texto-suave); white-space: nowrap; }
.linha-comando input { flex: 1; min-width: 0; background: none; color: var(--texto); border: 0; padding: 6px 0; font: inherit; outline: none; }
`;

export interface LinhaComando {
  el: HTMLElement;
  atualizar(e: Estado): void;
  /** Foca a caixa (quem digita numa tecla solta cai aqui, como no AutoCAD). */
  focar(): void;
  /** A caixa já está em foco? */
  emFoco(): boolean;
}

/** Uma linha do histórico como texto na língua atual. */
function linhaDoHistorico(e: Entrada): string {
  return 'digitado' in e ? `> ${e.digitado}` : texto(e.mensagem.chave, e.mensagem.vars);
}

/** Cria a linha de comando; `aoDigitar` recebe o texto do Enter. */
export function criarLinhaComando(t: Textos, aoDigitar: (texto: string) => void): LinhaComando {
  const el = document.createElement('footer');
  el.className = 'linha-comando';
  const historico = document.createElement('div');
  historico.className = 'historico';
  const escolhas = document.createElement('div');
  escolhas.className = 'escolhas';
  escolhas.hidden = true;
  const entrada = document.createElement('div');
  entrada.className = 'entrada';
  const prompt = document.createElement('span');
  prompt.className = 'prompt';
  const caixa = document.createElement('input');
  caixa.type = 'text';
  caixa.placeholder = t.comando.placeholder;
  caixa.spellcheck = false;
  caixa.autocomplete = 'off';
  entrada.append(prompt, caixa);
  el.append(historico, escolhas, entrada);

  caixa.addEventListener('keydown', (ev) => {
    if (ev.key !== 'Enter') return;
    ev.preventDefault();
    const valor = caixa.value;
    caixa.value = '';
    aoDigitar(valor);
  });

  return {
    el,
    atualizar(e) {
      historico.replaceChildren(
        ...e.historico.map((h) => {
          const d = document.createElement('div');
          d.textContent = linhaDoHistorico(h);
          return d;
        }),
      );
      historico.scrollTop = historico.scrollHeight;
      escolhas.hidden = e.blocos.length === 0;
      escolhas.replaceChildren(
        ...e.blocos.map((nome) => {
          const b = document.createElement('button');
          b.textContent = nome;
          b.addEventListener('click', () => aoDigitar(nome));
          return b;
        }),
      );
      prompt.textContent = texto(e.prompt.chave, e.prompt.vars);
    },
    focar: () => caixa.focus(),
    emFoco: () => document.activeElement === caixa,
  };
}
