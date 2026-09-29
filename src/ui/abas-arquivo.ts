/**
 * As abas de arquivo, logo abaixo da Ribbon: **Início** (a página inicial), o
 * desenho aberto (com `*` se há alteração não salva e o ✕ para fechar) e o `+`
 * (desenho novo). Por ora o motor guarda um desenho só; com vários abertos, cada
 * um ganha a sua aba aqui.
 */

import type { Estado } from '../ponte/index.ts';
import type { Textos } from '../idiomas/index.ts';

export const ABAS_ARQUIVO_CSS = `
.abas-arquivo { display: flex; align-items: flex-end; gap: 2px; padding: 4px 8px 0; background: var(--fundo); border-bottom: 1px solid var(--borda); }
.abas-arquivo .arquivo { display: flex; align-items: center; gap: 8px; padding: 4px 12px; background: var(--fundo-faixa); border: 1px solid var(--borda); border-bottom: 0; border-radius: 3px 3px 0 0; color: var(--texto-suave); cursor: pointer; }
.abas-arquivo .arquivo.ativa { background: var(--fundo-area); color: var(--texto); }
.abas-arquivo .fechar { background: none; border: 0; cursor: pointer; color: inherit; padding: 0 2px; opacity: .6; }
.abas-arquivo .fechar:hover { opacity: 1; }
.abas-arquivo .mais { background: none; border: 0; cursor: pointer; padding: 4px 10px; font-size: 15px; color: var(--texto-suave); }
`;

export type AbaAtiva = 'inicio' | 'desenho';

interface Acoes {
  aoInicio: () => void;
  aoDesenho: () => void;
  aoFechar: () => void;
  aoNovo: () => void;
}

export interface AbasArquivo {
  el: HTMLElement;
  atualizar(e: Estado, ativa: AbaAtiva, temDesenho: boolean): void;
}

export function criarAbasArquivo(t: Textos, acoes: Acoes): AbasArquivo {
  const el = document.createElement('div');
  el.className = 'abas-arquivo';
  const botao = (classe: string, texto: string, dica: string, acao: () => void): HTMLElement => {
    const b = document.createElement('div');
    b.className = classe;
    b.textContent = texto;
    b.title = dica;
    b.addEventListener('click', acao);
    return b;
  };
  return {
    el,
    atualizar(e, ativa, temDesenho) {
      const inicio = botao(`arquivo${ativa === 'inicio' ? ' ativa' : ''}`, t.abas.inicio, t.abas.inicio, acoes.aoInicio);
      const partes: HTMLElement[] = [inicio];
      if (temDesenho) {
        const nome = `${e.arquivo?.nome ?? t.app.semTitulo}${e.sujo ? '*' : ''}`;
        const desenho = botao(`arquivo${ativa === 'desenho' ? ' ativa' : ''}`, nome, e.arquivo?.caminho ?? nome, acoes.aoDesenho);
        const fechar = document.createElement('button');
        fechar.className = 'fechar';
        fechar.textContent = '✕';
        fechar.title = t.abas.fechar;
        fechar.addEventListener('click', (ev) => {
          ev.stopPropagation();
          acoes.aoFechar();
        });
        desenho.append(fechar);
        partes.push(desenho);
      }
      partes.push(botao('mais', '+', t.abas.novo, acoes.aoNovo));
      el.replaceChildren(...partes);
    },
  };
}
