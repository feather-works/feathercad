/** As abas de espaço (Modelo vs Layout 1, Layout 2, +). */

import type { Estado } from '../ponte/index.ts';

export const ABAS_LAYOUT_CSS = `
.abas-layout { display: flex; align-items: center; gap: 2px; }
.abas-layout button {
  padding: 2px 10px;
  border-radius: 4px;
  background: var(--fundo-painel);
  border: 1px solid var(--borda);
  color: var(--texto-suave);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  height: 22px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.abas-layout button:hover { background: var(--fundo-faixa); color: var(--texto); }
.abas-layout button.ativa {
  background: var(--destaque-suave);
  border-color: var(--destaque);
  color: var(--texto);
  font-weight: 600;
}
.abas-layout button.nova-aba {
  padding: 2px 6px;
  font-weight: bold;
}
`;

export interface AbasLayout {
  el: HTMLElement;
  atualizar(estado: Estado): void;
}

export function criarAbasLayout(
  aoAlternarEspaco: (nomeEspaco: string) => void,
  aoCriarLayout: (nome: string, papel: string, w: number, h: number) => void,
): AbasLayout {
  const el = document.createElement('div');
  el.className = 'abas-layout';

  function render(e: Estado): void {
    el.replaceChildren();

    const espacoAtivo = e.espaco_ativo;
    const isModelo = espacoAtivo === 'modelo';
    const activeLayoutName = typeof espacoAtivo === 'object' && espacoAtivo !== null && 'layout' in espacoAtivo
      ? (espacoAtivo as { layout: string }).layout
      : (typeof espacoAtivo === 'string' && espacoAtivo !== 'modelo' ? espacoAtivo : null);

    // Aba Modelo
    const btnModelo = document.createElement('button');
    btnModelo.type = 'button';
    btnModelo.className = isModelo ? 'ativa' : '';
    btnModelo.textContent = 'Modelo';
    btnModelo.addEventListener('click', () => aoAlternarEspaco('modelo'));
    el.append(btnModelo);

    // Abas de Layout
    const layouts = e.layouts || [];
    for (const layout of layouts) {
      const btnLayout = document.createElement('button');
      btnLayout.type = 'button';
      btnLayout.className = activeLayoutName === layout.nome ? 'ativa' : '';
      btnLayout.textContent = layout.nome;
      btnLayout.addEventListener('click', () => aoAlternarEspaco(layout.nome));
      el.append(btnLayout);
    }

    // Botão Nova Aba +
    const btnNovo = document.createElement('button');
    btnNovo.type = 'button';
    btnNovo.className = 'nova-aba';
    btnNovo.textContent = '+';
    btnNovo.title = 'Novo Layout (Paper Space)';
    btnNovo.addEventListener('click', () => {
      const num = layouts.length + 1;
      const nomeNovo = `Layout ${num}`;
      aoCriarLayout(nomeNovo, 'A4', 297, 210);
    });
    el.append(btnNovo);
  }

  return {
    el,
    atualizar(e: Estado): void {
      render(e);
    },
  };
}
