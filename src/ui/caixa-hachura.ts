/**
 * A escolha do padrão de hachura: uma lista com todos os padrões da biblioteca do motor, a escala e uma amostra desenhada
 * pelo Rust (SVG). Não calcula nada: manda o nome e a escala e o motor valida. Teclas: Enter aplica, Esc fecha.
 */

import type { Textos } from '../idiomas/index.ts';
import type { Motor } from '../ponte/index.ts';
import { campo, listaSuspensa } from './caixa-exportar.ts';

export function escolherHachura(t: Textos, motor: Motor): Promise<void> {
  return new Promise((resolve) => {
    const opcoes = motor.hachuraOpcoes();
    let padrao = opcoes.atual;
    let escala = String(opcoes.escala);
    const c = t.hachura.escolha;
    const nomes = t.hachura.padroes as Record<string, string>;
    const fundo = document.createElement('div');
    fundo.className = 'exportar-fundo';
    const caixa = document.createElement('div');
    caixa.className = 'exportar';
    caixa.setAttribute('role', 'dialog');
    caixa.setAttribute('aria-modal', 'true');
    caixa.setAttribute('aria-label', c.titulo);
    const previa = Object.assign(document.createElement('img'), { alt: '' });
    previa.style.cssText = 'width:100%;border-radius:8px;margin:4px 0 12px;display:block';
    const atualizar = (): void => {
      previa.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(motor.hachuraPrevia(padrao, Number(escala)))}`;
    };
    const duas = document.createElement('div');
    duas.className = 'duas';
    duas.append(
      campo(c.padrao, listaSuspensa(opcoes.padroes, (v) => nomes[v] ?? v, padrao, (v) => ((padrao = v), atualizar()))),
      campo(c.escala, listaSuspensa(opcoes.escalas.map(([v]) => String(v)), (v) => opcoes.escalas.find(([x]) => String(x) === v)?.[1] ?? v, escala, (v) => ((escala = v), atualizar()))),
    );
    const fechar = (): void => {
      window.removeEventListener('keydown', aoTeclar, true);
      fundo.remove();
      resolve();
    };
    const aplicar = (): void => {
      motor.hachuraDefinir(padrao, Number(escala));
      fechar();
    };
    const aoTeclar = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') fechar();
      else if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) aplicar();
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    const pe = document.createElement('div');
    pe.className = 'pe';
    const cancelar = Object.assign(document.createElement('button'), { type: 'button', textContent: c.cancelar });
    cancelar.addEventListener('click', fechar);
    const ok = Object.assign(document.createElement('button'), { type: 'button', className: 'principal', textContent: c.aplicar });
    ok.addEventListener('click', aplicar);
    pe.append(cancelar, ok);
    caixa.append(Object.assign(document.createElement('h2'), { textContent: c.titulo }), Object.assign(document.createElement('p'), { textContent: c.corpo }), previa, duas, pe);
    fundo.append(caixa);
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    atualizar();
    ok.focus();
  });
}
