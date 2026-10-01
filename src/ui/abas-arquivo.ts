/**
 * As abas de arquivo, logo abaixo da Ribbon: **Início** (a página inicial), cada
 * desenho aberto (com `*` se há alteração não salva e o ✕ para fechar) e o `+`
 * (desenho novo). Suporta múltiplos desenhos abertos em abas concorrentes.
 */

import type { Textos } from '../idiomas/index.ts';

export const ABAS_ARQUIVO_CSS = `
.abas-arquivo { display: flex; align-items: flex-end; gap: 2px; padding: 4px 8px 0; background: var(--fundo); border-bottom: 1px solid var(--borda); overflow-x: auto; }
.abas-arquivo .arquivo { display: flex; align-items: center; gap: 8px; padding: 4px 12px; background: var(--fundo-faixa); border: 1px solid var(--borda); border-bottom: 0; border-radius: 3px 3px 0 0; color: var(--texto-suave); cursor: pointer; user-select: none; max-width: 200px; }
.abas-arquivo .arquivo .nome-aba { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.abas-arquivo .arquivo.ativa { background: var(--fundo-area); color: var(--texto); font-weight: 500; }
.abas-arquivo .fechar { background: none; border: 0; cursor: pointer; color: inherit; padding: 0 2px; opacity: .6; font-size: 13px; border-radius: 2px; }
.abas-arquivo .fechar:hover { opacity: 1; background: var(--borda); }
.abas-arquivo .mais { background: none; border: 0; cursor: pointer; padding: 4px 10px; font-size: 15px; color: var(--texto-suave); }
.abas-arquivo .mais:hover { color: var(--texto); }
`;

export interface ItemAba {
  id: string;
  nome: string;
  caminho?: string;
  sujo: boolean;
}

export interface AcoesAbas {
  aoInicio: () => void;
  aoSelecionarAba: (id: string) => void;
  aoFecharAba: (id: string) => void;
  aoNovo: () => void;
}

export interface AbasArquivo {
  el: HTMLElement;
  atualizar(abas: ItemAba[], ativaId: string): void;
}

export function criarAbasArquivo(t: Textos, acoes: AcoesAbas): AbasArquivo {
  const el = document.createElement('div');
  el.className = 'abas-arquivo';

  return {
    el,
    atualizar(abas: ItemAba[], ativaId: string) {
      const inicio = document.createElement('div');
      inicio.className = `arquivo${ativaId === 'inicio' ? ' ativa' : ''}`;
      const inicioSpan = document.createElement('span');
      inicioSpan.className = 'nome-aba';
      inicioSpan.textContent = t.abas.inicio;
      inicio.append(inicioSpan);
      inicio.title = t.abas.inicio;
      inicio.addEventListener('click', () => acoes.aoInicio());

      const partes: HTMLElement[] = [inicio];

      for (const aba of abas) {
        const d = document.createElement('div');
        const eAtiva = aba.id === ativaId;
        d.className = `arquivo${eAtiva ? ' ativa' : ''}`;

        const nomeSpan = document.createElement('span');
        nomeSpan.className = 'nome-aba';
        nomeSpan.textContent = `${aba.nome}${aba.sujo ? '*' : ''}`;
        d.append(nomeSpan);
        d.title = aba.caminho ?? aba.nome;

        d.addEventListener('click', () => acoes.aoSelecionarAba(aba.id));

        const fechar = document.createElement('button');
        fechar.className = 'fechar';
        fechar.textContent = '✕';
        fechar.title = t.abas.fechar;
        fechar.addEventListener('click', (ev) => {
          ev.stopPropagation();
          acoes.aoFecharAba(aba.id);
        });

        d.append(fechar);
        partes.push(d);
      }

      const mais = document.createElement('div');
      mais.className = 'mais';
      mais.textContent = '+';
      mais.title = t.abas.novo;
      mais.addEventListener('click', () => acoes.aoNovo());
      partes.push(mais);

      el.replaceChildren(...partes);
    },
  };
}
