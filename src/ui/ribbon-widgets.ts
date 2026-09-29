/**
 * Os controles que moram dentro da faixa (não são botões de comando): a camada atual com as chaves rápidas
 * (ativada, congelada, bloqueada) e as três listas de propriedade dos objetos selecionados (cor, espessura,
 * tipo de linha). O motor decide tudo; aqui só se mostra e se manda.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import type { ConsultaDeCamadas, LinhaDeCamada } from '../ponte/tipos.ts';
import { texto, type Textos } from '../idiomas/index.ts';
import { iconeCadeado, iconeLampada, iconeSolOuNeve } from './camadas-icones.ts';
import { controleDeCor, controleDeEspessura, controleDeTipoDeLinha, fonteDePropriedades } from './propriedades-controles.ts';
import { avisar } from './toast.ts';

export const RIBBON_WIDGETS_CSS = `
.ribbon .widget { display: inline-flex; align-items: center; gap: 2px; height: 24px; align-self: center; padding: 0 3px; }
.ribbon .widget select { height: 22px; max-width: 168px; background: var(--fundo-area); color: var(--texto); border: 1px solid var(--borda); border-radius: 2px; font: inherit; }
.ribbon .widget select:disabled { opacity: 0.55; }
.ribbon .widget button.chave { background: none; border: 1px solid transparent; border-radius: 3px; padding: 1px 2px; cursor: pointer; line-height: 0; }
.ribbon .widget button.chave:hover { border-color: var(--borda); background: var(--fundo-faixa); }
`;

export interface WidgetDaRibbon {
  el: HTMLElement;
  atualizar(e: Estado): void;
}

const CONSULTA: ConsultaDeCamadas = { filtro: 'todas', busca: '', ordem: 'nome', crescente: true };

/** Fabrica os widgets da faixa pelo id que `ribbon-dados.ts` usa. */
export function criarWidgetsDaRibbon(t: Textos, motor: Motor, aoMudar: (e: Estado) => void): Record<string, () => WidgetDaRibbon> {
  const resumoDe = fonteDePropriedades(motor);
  const contexto = (): HTMLElement => Object.assign(document.createElement('span'), { className: 'widget' });

  const camadaAtual = (): WidgetDaRibbon => {
    const el = contexto();
    const sel = document.createElement('select');
    sel.title = t.camadas.combinada;
    sel.addEventListener('keydown', (e) => e.stopPropagation());
    const chaves = document.createElement('span');
    chaves.style.display = 'inline-flex';
    el.append(sel, chaves);
    let linhas: LinhaDeCamada[] = [];
    let chave = '';
    const agir = (acao: 'atual' | 'propriedade', indice: number, propriedade?: 'ligada' | 'congelada' | 'bloqueada', valor?: boolean): void => {
      const r = motor.camadaAcao({ acao, indices: [indice], propriedade, valor, consulta: CONSULTA });
      if (r.erro) avisar(texto(r.erro));
      aoMudar(r.estado);
    };
    sel.addEventListener('change', () => agir('atual', Number(sel.value)));
    const botao = (dica: string, icone: HTMLElement, aplicar: () => void): HTMLButtonElement => {
      const b = document.createElement('button');
      b.className = 'chave';
      b.title = dica;
      b.append(icone);
      b.addEventListener('click', aplicar);
      return b;
    };
    return {
      el,
      atualizar(e) {
        const nova = `${e.versao}:${e.camada_atual}`;
        if (nova === chave) return;
        chave = nova;
        linhas = motor.camadas(CONSULTA).linhas;
        sel.replaceChildren(
          ...linhas.map((l) => {
            const o = document.createElement('option');
            o.value = String(l.indice);
            o.textContent = l.nome;
            o.selected = l.atual;
            return o;
          }),
        );
        const atual = linhas.find((l) => l.atual);
        chaves.replaceChildren();
        if (!atual) return;
        chaves.append(
          botao(t.camadas.ligada, iconeLampada(atual.ligada), () => agir('propriedade', atual.indice, 'ligada', !atual.ligada)),
          botao(t.camadas.congelada, iconeSolOuNeve(atual.congelada), () => agir('propriedade', atual.indice, 'congelada', !atual.congelada)),
          botao(t.camadas.bloqueada, iconeCadeado(atual.bloqueada), () => agir('propriedade', atual.indice, 'bloqueada', !atual.bloqueada)),
        );
      },
    };
  };

  const propriedade = (campo: 'cor' | 'espessura' | 'tipo_linha', fabrica: typeof controleDeCor): (() => WidgetDaRibbon) => () => {
    const el = contexto();
    const c = fabrica(t, (valor) => aoMudar(motor.propriedadesDefinir(campo, valor)));
    el.append(c.el);
    return { el, atualizar: (e) => c.mostrar(resumoDe(e)) };
  };

  return {
    camadaAtual,
    propCor: propriedade('cor', controleDeCor),
    propEspessura: propriedade('espessura', controleDeEspessura),
    propTipo: propriedade('tipo_linha', controleDeTipoDeLinha),
  };
}
