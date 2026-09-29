/**
 * A paleta de propriedades (Ctrl+1): o que os objetos selecionados têm em comum — camada, cor, tipo de
 * linha e espessura — e a troca de qualquer um deles de uma vez. Só desenha o resumo do motor.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import { texto, type Textos } from '../idiomas/index.ts';
import { controleDeCamada, controleDeCor, controleDeEspessura, controleDeTipoDeLinha, fonteDePropriedades } from './propriedades-controles.ts';
import { Z } from './z.ts';

export const PROPRIEDADES_PAINEL_CSS = `
.prop-painel { position: absolute; right: 64px; top: 16px; width: 268px; z-index: ${Z.paleta}; display: flex; flex-direction: column; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 4px; box-shadow: var(--sombra); }
.prop-painel .titulo { display: flex; align-items: center; gap: 8px; padding: 5px 8px; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); cursor: move; user-select: none; font-weight: 600; }
.prop-painel .titulo .espaco { flex: 1; }
.prop-painel button.fechar { background: none; border: 0; cursor: pointer; font-size: 14px; color: var(--texto-suave); }
.prop-painel .resumo { padding: 6px 10px; color: var(--texto-suave); border-bottom: 1px solid var(--borda); }
.prop-painel .secao { padding: 4px 0 8px; }
.prop-painel .secao .cabeca { padding: 2px 10px; font-weight: 600; background: var(--fundo-faixa); }
.prop-painel .linha { display: grid; grid-template-columns: 100px minmax(0, 1fr); align-items: center; gap: 6px; padding: 3px 10px; }
.prop-painel .linha .rotulo { color: var(--texto-suave); }
.prop-painel select.prop-controle { width: 100%; height: 22px; background: var(--fundo-area); color: var(--texto); border: 1px solid var(--borda); border-radius: 2px; font: inherit; }
.prop-painel select.prop-controle:disabled { opacity: 0.55; }
`;

export interface PainelDePropriedades {
  el: HTMLElement;
  alternar(): void;
  atualizar(e: Estado): void;
}

export function criarPainelDePropriedades(t: Textos, motor: Motor, aoMudar: (e: Estado) => void): PainelDePropriedades {
  const el = document.createElement('div');
  el.className = 'prop-painel';
  el.hidden = true;
  const resumoDe = fonteDePropriedades(motor);
  const definir = (campo: 'camada' | 'cor' | 'tipo_linha' | 'espessura') => (valor: string) => aoMudar(motor.propriedadesDefinir(campo, valor));

  const titulo = document.createElement('div');
  titulo.className = 'titulo';
  const nome = document.createElement('span');
  nome.textContent = t.propriedades.titulo;
  const espaco = document.createElement('span');
  espaco.className = 'espaco';
  const fechar = document.createElement('button');
  fechar.className = 'fechar';
  fechar.textContent = '✕';
  fechar.title = t.propriedades.fechar;
  fechar.addEventListener('click', () => (el.hidden = true));
  titulo.append(nome, espaco, fechar);

  const resumo = document.createElement('div');
  resumo.className = 'resumo';
  const secao = document.createElement('div');
  secao.className = 'secao';
  const cabeca = document.createElement('div');
  cabeca.className = 'cabeca';
  cabeca.textContent = t.propriedades.geral;
  secao.append(cabeca);
  const controles = [
    { rotulo: t.propriedades.cor, c: controleDeCor(t, definir('cor')) },
    { rotulo: t.propriedades.camada, c: controleDeCamada(t, motor, definir('camada')) },
    { rotulo: t.propriedades.tipoLinha, c: controleDeTipoDeLinha(t, definir('tipo_linha')) },
    { rotulo: t.propriedades.espessura, c: controleDeEspessura(t, definir('espessura')) },
  ];
  for (const { rotulo, c } of controles) {
    const linha = document.createElement('div');
    linha.className = 'linha';
    const r = document.createElement('span');
    r.className = 'rotulo';
    r.textContent = rotulo;
    linha.append(r, c.el);
    secao.append(linha);
  }
  el.append(titulo, resumo, secao);

  titulo.addEventListener('mousedown', (ev) => {
    if ((ev.target as HTMLElement).closest('button')) return;
    const pai = el.offsetParent as HTMLElement | null;
    const [x0, y0, esq, topo] = [ev.clientX, ev.clientY, el.offsetLeft, el.offsetTop];
    const mover = (m: MouseEvent): void => {
      el.style.right = 'auto';
      el.style.left = `${Math.max(0, Math.min((pai?.clientWidth ?? 0) - 60, esq + m.clientX - x0))}px`;
      el.style.top = `${Math.max(0, Math.min((pai?.clientHeight ?? 0) - 30, topo + m.clientY - y0))}px`;
    };
    const soltar = (): void => {
      window.removeEventListener('mousemove', mover);
      window.removeEventListener('mouseup', soltar);
    };
    window.addEventListener('mousemove', mover);
    window.addEventListener('mouseup', soltar);
    ev.preventDefault();
  });
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick', 'contextmenu'] as const) el.addEventListener(ev, (e) => e.stopPropagation());

  const mostrar = (e: Estado): void => {
    const r = resumoDe(e);
    resumo.textContent = r.n === 0 ? t.propriedades.nenhum : texto('propriedades.objetos', { n: String(r.n) });
    for (const { c } of controles) c.mostrar(r);
  };
  return {
    el,
    alternar() {
      el.hidden = !el.hidden;
      if (!el.hidden) mostrar(motor.estado());
    },
    atualizar(e) {
      if (!el.hidden) mostrar(e);
    },
  };
}
