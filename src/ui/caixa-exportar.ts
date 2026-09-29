/**
 * A pergunta do "Exportar PDF": papel, orientação e quais folhas entram. Só junta a escolha
 * num pedido; quem monta o PDF (e valida papel, orientação e índices) é o Rust.
 * Teclas: Enter exporta, Esc cancela.
 */

import type { Textos } from '../idiomas/index.ts';
import type { OpcoesPdf } from '../ponte/tipos.ts';
import { Z } from './z.ts';

export const CAIXA_EXPORTAR_CSS = `
.exportar-fundo { position: fixed; inset: 0; z-index: ${Z.menu}; background: var(--veu); display: flex; align-items: center; justify-content: center; }
.exportar { width: 460px; max-width: 94vw; max-height: 90vh; overflow: auto; padding: 24px 26px 20px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 16px; box-shadow: var(--sombra); }
.exportar h2 { margin: 0 0 4px; font-size: 17px; font-weight: 600; }
.exportar > p { margin: 0 0 16px; color: var(--texto-suave); line-height: 1.45; }
.exportar fieldset { margin: 0 0 14px; padding: 0; border: 0; }
.exportar legend { padding: 0; margin-bottom: 6px; font-size: 12px; color: var(--texto-suave); }
.exportar .grupo { display: flex; gap: 8px; flex-wrap: wrap; }
.exportar .grupo button { padding: 6px 14px; background: var(--fundo-faixa); color: var(--texto); border: 1.5px solid var(--borda); border-radius: 8px; cursor: pointer; }
.exportar .grupo button:hover { border-color: var(--destaque); }
.exportar .grupo button[aria-pressed='true'] { background: var(--destaque-suave); border-color: var(--destaque); }
.exportar input[type='text'] { width: 100%; box-sizing: border-box; padding: 6px 10px; background: var(--fundo-faixa); color: var(--texto); border: 1.5px solid var(--borda); border-radius: 8px; font: inherit; }
.exportar input[type='text']:focus { border-color: var(--destaque); outline: none; }
.exportar .duas { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.exportar .duas fieldset { margin: 0; }
.exportar select { width: 100%; padding: 7px 10px; background: var(--fundo-faixa); color: var(--texto); border: 1.5px solid var(--borda); border-radius: 8px; font: inherit; cursor: pointer; }
.exportar select:hover, .exportar select:focus { border-color: var(--destaque); outline: none; }
.exportar .folhas { display: flex; flex-direction: column; gap: 4px; max-height: 180px; overflow: auto; }
.exportar label { display: flex; align-items: center; gap: 8px; cursor: pointer; }
.exportar .folhas .medida { margin-left: auto; color: var(--texto-suave); font-size: 12px; }
.exportar .atalhos { display: flex; gap: 10px; margin-bottom: 6px; }
.exportar .atalhos button { padding: 0; background: none; border: 0; color: var(--destaque); cursor: pointer; font-size: 12px; }
.exportar .aviso { color: var(--texto-suave); font-size: 12px; }
.exportar .pe { display: flex; justify-content: flex-end; gap: 10px; margin-top: 18px; }
.exportar .pe button { padding: 6px 18px; background: none; color: var(--texto); border: 1px solid var(--borda); border-radius: 8px; cursor: pointer; }
.exportar .pe button.principal { background: var(--destaque); color: var(--fundo-painel); border-color: var(--destaque); }
`;

const PAPEIS: OpcoesPdf['papel'][] = ['A4', 'A3', 'A2', 'A1', 'A0', '2A0', '4A0', '8A0', '16A0', 'FOLHA'];
const ORIENTACOES: OpcoesPdf['orientacao'][] = ['automatica', 'horizontal', 'vertical'];

/** Um grupo de botões de escolha única; `aoMudar` guarda o valor. */
function grupo<T extends string>(valores: T[], rotulo: (v: T) => string, inicial: T, aoMudar: (v: T) => void): HTMLDivElement {
  const div = document.createElement('div');
  div.className = 'grupo';
  const botoes = valores.map((v) => {
    const b = Object.assign(document.createElement('button'), { type: 'button', textContent: rotulo(v) });
    b.setAttribute('aria-pressed', String(v === inicial));
    b.addEventListener('click', () => {
      aoMudar(v);
      valores.forEach((w, i) => botoes[i]?.setAttribute('aria-pressed', String(w === v)));
    });
    return b;
  });
  div.append(...botoes);
  return div;
}

/** Uma lista suspensa de escolha única; `aoMudar` guarda o valor. */
export function listaSuspensa<T extends string>(valores: T[], rotulo: (v: T) => string, inicial: T, aoMudar: (v: T) => void): HTMLSelectElement {
  const sel = document.createElement('select');
  for (const v of valores) sel.append(Object.assign(document.createElement('option'), { value: v, textContent: rotulo(v), selected: v === inicial }));
  sel.addEventListener('change', () => aoMudar(sel.value as T));
  return sel;
}

export function campo(titulo: string, conteudo: HTMLElement): HTMLFieldSetElement {
  const f = document.createElement('fieldset');
  f.append(Object.assign(document.createElement('legend'), { textContent: titulo }), conteudo);
  return f;
}

/** Pergunta como exportar; `null` se o usuário desistiu. `folhas` traz largura × altura de cada folha (só para mostrar). */
export function perguntarExportacao(t: Textos, folhas: Array<[number, number]>, unidade: string): Promise<OpcoesPdf | null> {
  return new Promise((resolve) => {
    const escolha: OpcoesPdf = { papel: 'A3', orientacao: 'automatica', cores: false };
    const marcadas = folhas.map(() => true);
    const fundo = document.createElement('div');
    fundo.className = 'exportar-fundo';
    const caixa = document.createElement('div');
    caixa.className = 'exportar';
    caixa.setAttribute('role', 'dialog');
    caixa.setAttribute('aria-modal', 'true');
    caixa.setAttribute('aria-label', t.exportar.titulo);

    const fechar = (r: OpcoesPdf | null): void => {
      window.removeEventListener('keydown', aoTeclar, true);
      fundo.remove();
      resolve(r);
    };
    const confirmar = (): void => {
      if (folhas.length > 0) {
        const indices = marcadas.flatMap((m, i) => (m ? [i] : []));
        if (indices.length === 0) return;
        escolha.folhas = indices;
      }
      fechar(escolha);
    };
    const aoTeclar = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') fechar(null);
      else if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) confirmar();
      else return;
      e.preventDefault();
      e.stopPropagation();
    };

    caixa.append(Object.assign(document.createElement('h2'), { textContent: t.exportar.titulo }), Object.assign(document.createElement('p'), { textContent: t.exportar.corpo }));
    caixa.append(campo(t.exportar.papel, listaSuspensa(PAPEIS, (p) => p, escolha.papel, (v) => (escolha.papel = v))));
    caixa.append(campo(t.exportar.orientacao, grupo(ORIENTACOES, (o) => t.exportar.orientacoes[o], escolha.orientacao, (v) => (escolha.orientacao = v))));

    const lista = document.createElement('div');
    if (folhas.length === 0) {
      lista.append(Object.assign(document.createElement('p'), { className: 'aviso', textContent: t.exportar.semFolhas }));
    } else {
      const caixas: HTMLInputElement[] = [];
      const todas = (valor: boolean): void => caixas.forEach((c, i) => ((c.checked = valor), (marcadas[i] = valor)));
      const atalhos = document.createElement('div');
      atalhos.className = 'atalhos';
      for (const [rotulo, valor] of [[t.exportar.todas, true], [t.exportar.nenhuma, false]] as const) {
        const b = Object.assign(document.createElement('button'), { type: 'button', textContent: rotulo });
        b.addEventListener('click', () => todas(valor));
        atalhos.append(b);
      }
      const itens = document.createElement('div');
      itens.className = 'folhas';
      folhas.forEach(([w, h], i) => {
        const rotulo = document.createElement('label');
        const c = Object.assign(document.createElement('input'), { type: 'checkbox', checked: true });
        c.addEventListener('change', () => (marcadas[i] = c.checked));
        caixas.push(c);
        const medida = Object.assign(document.createElement('span'), { className: 'medida', textContent: `${+w.toFixed(1)} × ${+h.toFixed(1)} ${unidade}` });
        rotulo.append(c, t.exportar.folha.replace('{n}', String(i + 1)), medida);
        itens.append(rotulo);
      });
      lista.append(atalhos, itens);
    }
    caixa.append(campo(t.exportar.folhas, lista));

    const cores = document.createElement('label');
    const caixaCores = Object.assign(document.createElement('input'), { type: 'checkbox' });
    caixaCores.addEventListener('change', () => (escolha.cores = caixaCores.checked));
    cores.append(caixaCores, t.exportar.cores);
    caixa.append(cores);

    const pe = document.createElement('div');
    pe.className = 'pe';
    const cancelar = Object.assign(document.createElement('button'), { type: 'button', textContent: t.exportar.cancelar });
    cancelar.addEventListener('click', () => fechar(null));
    const ok = Object.assign(document.createElement('button'), { type: 'button', className: 'principal', textContent: t.exportar.exportar });
    ok.addEventListener('click', confirmar);
    pe.append(cancelar, ok);
    caixa.append(pe);

    fundo.append(caixa);
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    ok.focus();
  });
}
