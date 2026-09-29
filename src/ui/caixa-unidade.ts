/**
 * A pergunta que abre todo desenho novo: em que unidade ele vai ser trabalhado (mm, cm
 * ou m). A escolha vai para o motor (`documento_novo`), que a guarda no arquivo; aqui só
 * se pergunta. Já vem marcada a última usada. Teclas: 1/2/3 escolhem, ←/→ movem, Enter
 * confirma, Esc cancela o "Novo".
 */

import type { Textos } from '../idiomas/index.ts';
import type { Unidade } from '../ponte/tipos.ts';
import { Z } from './z.ts';

export const CAIXA_UNIDADE_CSS = `
.unidade-fundo { position: fixed; inset: 0; z-index: ${Z.menu}; background: var(--veu); display: flex; align-items: center; justify-content: center; animation: unidade-fundo 160ms ease-out; }
.unidade { width: 640px; max-width: 94vw; padding: 26px 28px 20px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 16px; box-shadow: var(--sombra); animation: unidade-entra 200ms cubic-bezier(.2,.8,.3,1); }
@keyframes unidade-fundo { from { opacity: 0; } }
@keyframes unidade-entra { from { opacity: 0; transform: translateY(10px) scale(.97); } }
@media (prefers-reduced-motion: reduce) { .unidade-fundo, .unidade { animation: none; } }
.unidade .cabeca { display: flex; align-items: center; gap: 14px; margin-bottom: 20px; }
.unidade .selo { flex: none; display: grid; place-items: center; width: 46px; height: 46px; border-radius: 14px; background: var(--destaque-suave); color: var(--destaque); }
.unidade .selo svg { width: 26px; height: 26px; }
.unidade h2 { margin: 0; font-size: 17px; font-weight: 600; }
.unidade .cabeca p { margin: 3px 0 0; color: var(--texto-suave); line-height: 1.45; }
.unidade .opcoes { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.unidade .opcao { position: relative; display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 14px 14px 12px; text-align: left; background: var(--fundo-faixa); color: var(--texto); border: 1.5px solid var(--borda); border-radius: 12px; cursor: pointer; transition: border-color 120ms, background 120ms, transform 120ms; }
.unidade .opcao:hover { border-color: var(--destaque); transform: translateY(-2px); }
.unidade .opcao:focus-visible { outline: 2px solid var(--destaque); outline-offset: 2px; }
.unidade .opcao[aria-pressed='true'] { background: var(--destaque-suave); border-color: var(--destaque); }
.unidade .opcao .tecla { position: absolute; top: 10px; right: 10px; min-width: 20px; padding: 1px 6px; text-align: center; font-size: 11px; color: var(--texto-suave); background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 5px; }
.unidade .opcao .simbolo { font-size: 30px; font-weight: 600; line-height: 1.1; color: var(--destaque); }
.unidade .opcao .extenso { font-size: 13px; }
.unidade .opcao svg.regua { width: 100%; height: 34px; margin: 10px 0 6px; color: var(--texto-suave); }
.unidade .opcao .uso { color: var(--texto-suave); font-size: 11.5px; line-height: 1.4; }
.unidade .pe { display: flex; align-items: center; gap: 12px; margin-top: 18px; }
.unidade .pe .aviso { flex: 1; color: var(--texto-suave); font-size: 12px; }
.unidade .pe button { padding: 6px 16px; background: none; color: var(--texto); border: 1px solid var(--borda); border-radius: 8px; cursor: pointer; }
.unidade .pe button:hover { border-color: var(--destaque); }
@media (max-width: 560px) { .unidade .opcoes { grid-template-columns: 1fr; } }
`;

const UNIDADES: Unidade[] = ['mm', 'cm', 'm'];
const SVG = 'http://www.w3.org/2000/svg';

function svg(nome: string, atributos: Record<string, string>, texto?: string): SVGElement {
  const el = document.createElementNS(SVG, nome);
  for (const [k, v] of Object.entries(atributos)) el.setAttribute(k, v);
  if (texto !== undefined) el.textContent = texto;
  return el;
}

/** O ícone de régua do cabeçalho. */
function iconeRegua(): SVGElement {
  const s = svg('svg', { viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.8', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true' });
  s.append(svg('rect', { x: '2.5', y: '8', width: '19', height: '8', rx: '1.6' }));
  for (const x of [6, 9.5, 13, 16.5]) s.append(svg('path', { d: `M${x} 8v${x === 9.5 || x === 16.5 ? 2.6 : 4}` }));
  return s;
}

/** Uma régua de 10 divisões cujo fim diz quanto ela mede (`10 mm`, `10 cm`, `1 m`). */
function regua(fim: string): SVGElement {
  const s = svg('svg', { viewBox: '0 0 120 34', class: 'regua', 'aria-hidden': 'true' });
  s.append(svg('line', { x1: '4', y1: '10', x2: '116', y2: '10', stroke: 'currentColor', 'stroke-width': '1.4' }));
  for (let i = 0; i <= 10; i++) {
    const x = 4 + i * 11.2;
    const alto = i === 0 || i === 10 ? 12 : i === 5 ? 9 : 5;
    s.append(svg('line', { x1: String(x), y1: '10', x2: String(x), y2: String(10 + alto), stroke: 'currentColor', 'stroke-width': '1.4', 'stroke-linecap': 'round' }));
  }
  s.append(svg('text', { x: '4', y: '32', 'font-size': '9', fill: 'currentColor' }, '0'));
  s.append(svg('text', { x: '116', y: '32', 'font-size': '9', fill: 'currentColor', 'text-anchor': 'end' }, fim));
  return s;
}

/** Pergunta a unidade; `null` se o usuário desistiu. `sugerida` é a que já vem marcada. */
export function perguntarUnidade(t: Textos, sugerida: Unidade): Promise<Unidade | null> {
  return new Promise((resolve) => {
    const fundo = document.createElement('div');
    fundo.className = 'unidade-fundo';
    const caixa = document.createElement('div');
    caixa.className = 'unidade';
    caixa.setAttribute('role', 'dialog');
    caixa.setAttribute('aria-modal', 'true');
    caixa.setAttribute('aria-label', t.unidade.titulo);

    const cabeca = document.createElement('div');
    cabeca.className = 'cabeca';
    const selo = document.createElement('div');
    selo.className = 'selo';
    selo.append(iconeRegua());
    const textos = document.createElement('div');
    const titulo = document.createElement('h2');
    titulo.textContent = t.unidade.titulo;
    const corpo = document.createElement('p');
    corpo.textContent = t.unidade.corpo;
    textos.append(titulo, corpo);
    cabeca.append(selo, textos);

    const opcoes = document.createElement('div');
    opcoes.className = 'opcoes';
    const botoes: HTMLButtonElement[] = [];
    const fechar = (r: Unidade | null): void => {
      window.removeEventListener('keydown', aoTeclar, true);
      fundo.remove();
      resolve(r);
    };
    const marcar = (i: number): void => {
      botoes.forEach((b, j) => b.setAttribute('aria-pressed', i === j ? 'true' : 'false'));
      botoes[i]?.focus();
    };
    const aoTeclar = (e: KeyboardEvent): void => {
      const atual = botoes.findIndex((b) => b.getAttribute('aria-pressed') === 'true');
      const numero = UNIDADES[Number(e.key) - 1];
      if (e.key === 'Escape') fechar(null);
      else if (numero) fechar(numero);
      else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') marcar((atual + (e.key === 'ArrowRight' ? 1 : UNIDADES.length - 1)) % UNIDADES.length);
      else if (e.key === 'Enter') fechar(UNIDADES[atual] ?? sugerida);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    UNIDADES.forEach((u, i) => {
      const b = document.createElement('button');
      b.className = 'opcao';
      b.setAttribute('aria-pressed', u === sugerida ? 'true' : 'false');
      const tecla = Object.assign(document.createElement('span'), { className: 'tecla', textContent: String(i + 1) });
      const simbolo = Object.assign(document.createElement('span'), { className: 'simbolo', textContent: u });
      const extenso = Object.assign(document.createElement('span'), { className: 'extenso', textContent: t.unidade.nomes[u] });
      const uso = Object.assign(document.createElement('span'), { className: 'uso', textContent: t.unidade.uso[u] });
      b.append(tecla, simbolo, extenso, regua(t.unidade.regua[u]), uso);
      b.addEventListener('click', () => fechar(u));
      b.addEventListener('mouseenter', () => marcar(i));
      botoes.push(b);
      opcoes.append(b);
    });

    const pe = document.createElement('div');
    pe.className = 'pe';
    const aviso = Object.assign(document.createElement('span'), { className: 'aviso', textContent: t.unidade.trocarDepois });
    const cancelar = Object.assign(document.createElement('button'), { textContent: t.unidade.cancelar });
    cancelar.addEventListener('click', () => fechar(null));
    pe.append(aviso, cancelar);

    caixa.append(cabeca, opcoes, pe);
    fundo.append(caixa);
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    botoes[Math.max(0, UNIDADES.indexOf(sugerida))]?.focus();
  });
}
