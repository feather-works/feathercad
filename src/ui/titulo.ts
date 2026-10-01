/** A barra de título: acesso rápido (novo, abrir, salvar, desfazer, refazer) e o nome do arquivo (`*` se sujo). */

import type { Estado } from '../ponte/index.ts';
import type { Textos } from '../idiomas/index.ts';
import { obterIconeSVG } from './icones-svg.ts';
import type { ModoDoTema } from './tema.ts';

export const TITULO_CSS = `
.titulo { display: flex; align-items: center; gap: 2px; height: 30px; padding: 0 8px; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); }
.titulo .nome { flex: 1; text-align: center; color: var(--texto-suave); }
.titulo .separador { width: 1px; height: 16px; margin: 0 4px; background: var(--borda); }
.titulo button { background: none; border: 0; padding: 2px 8px; border-radius: 3px; cursor: pointer; font-size: 15px; display: inline-flex; align-items: center; justify-content: center; color: var(--texto); }
.titulo button:hover:not(:disabled) { background: var(--fundo-painel); }
.titulo button:disabled { opacity: .35; cursor: default; }
`;

interface Acoes {
  novo: () => void;
  abrir: () => void;
  salvar: () => void;
  /** Exporta as folhas em PDF. */
  exportar: () => void;
  desfazer: () => void;
  refazer: () => void;
  /** Gira o tema (automático → claro → escuro). */
  tema: () => void;
  /** Abre a janela de Configurações. */
  configuracoes: () => void;
}

export interface Titulo {
  el: HTMLElement;
  atualizar(e: Estado, temDesenho: boolean): void;
  /** Mostra o modo atual do tema no botão. */
  modoDoTema(modo: ModoDoTema): void;
}

/** Cria a barra de título; cada botão só repassa o pedido. */
export function criarTitulo(t: Textos, acoes: Acoes): Titulo {
  const el = document.createElement('header');
  el.className = 'titulo';
  const botao = (id: string, dica: string, acao: () => void): HTMLButtonElement => {
    const b = document.createElement('button');
    const svgIcone = obterIconeSVG(id);
    if (svgIcone.startsWith('<svg')) {
      b.innerHTML = svgIcone;
    } else {
      b.textContent = id;
    }
    b.title = dica;
    b.addEventListener('click', acao);
    return b;
  };
  const novo = botao('novo', t.titulo.novo, acoes.novo);
  const abrir = botao('abrir', t.titulo.abrir, acoes.abrir);
  const salvar = botao('salvar', t.titulo.salvar, acoes.salvar);
  const exportar = botao('exportar', t.titulo.exportar, acoes.exportar);
  const desfazer = botao('desfazer', t.titulo.desfazer, acoes.desfazer);
  const refazer = botao('refazer', t.titulo.refazer, acoes.refazer);
  const nome = document.createElement('span');
  nome.className = 'nome';
  const separador = document.createElement('span');
  separador.className = 'separador';
  const config = botao('configuracoes', t.config.abrir, acoes.configuracoes);
  const tema = botao('tema', '', acoes.tema);
  tema.style.marginLeft = '4px';
  el.append(novo, abrir, salvar, exportar, separador, desfazer, refazer, nome, tema, config);
  return {
    el,
    modoDoTema(modo) {
      tema.title = t.tema.dica.replace('{modo}', t.tema.modos[modo]);
    },
    atualizar(e, temDesenho) {
      salvar.disabled = !temDesenho;
      exportar.disabled = !temDesenho;
      desfazer.disabled = !temDesenho || !e.pode_desfazer;
      refazer.disabled = !temDesenho || !e.pode_refazer;
      nome.textContent = temDesenho ? `${t.app.nome} — ${e.arquivo?.nome ?? t.app.semTitulo}${e.sujo ? '*' : ''}` : t.app.nome;
    },
  };
}
