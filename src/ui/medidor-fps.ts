/**
 * O medidor de quadros por segundo (FPS) da área de desenho — para achar onde o movimento trava.
 * Liga e desliga em Configurações → Sobre ou com Ctrl+Shift+F (a escolha fica em `localStorage`; de fábrica, desligado). **Só mede e mostra**: não decide nada
 * sobre o desenho. Mostra o FPS **durante o movimento** (o navegador desenha a 60 fps parado, o que não diz nada),
 * o pior quadro da última janela e o tempo médio de cada etapa: motor (quadro), cursor e pintura. Se as etapas
 * somam pouco e o FPS é baixo, o gargalo está na tela (webview/canvas), não no motor.
 */

import type { Textos } from '../idiomas/index.ts';
import { ligarPerfil } from '../ponte/perfil.ts';
import { fpsLigado, guardarFps } from './preferencias.ts';
import { Z } from './z.ts';

/** A janela da conta, em ms. */
const JANELA_MS = 1000;
/** Um intervalo maior que isto é pausa (mouse parado), não quadro lento: não entra na conta. */
const PAUSA_MS = 250;

export type Etapa = 'motor' | 'cursor' | 'pintura' | 'total';

export interface Leitura {
  /** Quadros por segundo durante o movimento (0 se não houve movimento na janela). */
  fps: number;
  /** O maior intervalo entre dois quadros seguidos na janela, em ms. */
  pior: number;
  /** Tempo médio de cada etapa por quadro, em ms. */
  etapas: Record<Etapa, number>;
  /** O que sobra de cada quadro depois do nosso trabalho (`total`): o tempo da webview compondo e entregando o quadro, em ms. */
  webview: number;
}

/** A conta, sem tela: recebe o instante de cada quadro e o tempo de cada etapa. */
export function criarConta(): { quadro(agora: number): void; tempo(etapa: Etapa, ms: number, agora: number): void; ler(agora: number): Leitura } {
  let quadros: number[] = [];
  let tempos: { etapa: Etapa; ms: number; em: number }[] = [];
  const podar = (agora: number): void => {
    quadros = quadros.filter((q) => agora - q <= JANELA_MS);
    tempos = tempos.filter((x) => agora - x.em <= JANELA_MS);
  };
  return {
    quadro(agora) {
      quadros.push(agora);
      podar(agora);
    },
    tempo(etapa, ms, agora) {
      tempos.push({ etapa, ms, em: agora });
    },
    ler(agora) {
      podar(agora);
      const intervalos = quadros.slice(1).map((q, i) => q - (quadros[i] ?? q)).filter((d) => d < PAUSA_MS);
      const soma = intervalos.reduce((a, b) => a + b, 0);
      const media = (etapa: Etapa): number => {
        const v = tempos.filter((x) => x.etapa === etapa);
        return v.length === 0 ? 0 : v.reduce((a, x) => a + x.ms, 0) / v.length;
      };
      return {
        fps: intervalos.length === 0 || soma === 0 ? 0 : (intervalos.length / soma) * 1000,
        pior: intervalos.length === 0 ? 0 : Math.max(...intervalos),
        etapas: { motor: media('motor'), cursor: media('cursor'), pintura: media('pintura'), total: media('total') },
        webview: intervalos.length === 0 || soma === 0 ? 0 : Math.max(0, soma / intervalos.length - media('total')),
      };
    },
  };
}

export const MEDIDOR_FPS_CSS = `
.medidor-fps { position: absolute; top: 6px; left: 50%; transform: translateX(-50%); z-index: ${Z.entradaDinamica}; padding: 2px 8px; background: var(--caixa-dinamica); border: 1px solid var(--destaque); color: var(--texto); font: 11px var(--fonte); pointer-events: none; white-space: nowrap; }
`;

export interface MedidorFps {
  el: HTMLElement;
  /** Chame uma vez por quadro de tela em que algo foi refeito. */
  quadro(): void;
  /** Uma linha de contexto (quantos segmentos/textos há no quadro), mostrada no fim. */
  info(texto: string): void;
  /** Mede `f` e a soma na etapa. */
  medir<T>(etapa: Etapa, f: () => T): T;
}

/** Cria o medidor (Configurações ou Ctrl+Shift+F). */
export function criarMedidorFps(t: Textos): MedidorFps {
  const conta = criarConta();
  const el = document.createElement('div');
  el.className = 'medidor-fps';
  el.hidden = !fpsLigado();
  // O perfil de etapas (`ponte/perfil.ts`, resumo no console/Monitor) liga e desliga junto com o medidor.
  ligarPerfil(!el.hidden);
  let contexto = '';
  const agora = (): number => performance.now();
  const desenhar = (): void => {
    if (el.hidden) return;
    const l = conta.ler(agora());
    const n = (v: number): string => v.toFixed(1);
    el.textContent = `${t.fps.rotulo} ${l.fps === 0 ? '—' : Math.round(l.fps)} · ${t.fps.pior} ${n(l.pior)} ms · ${t.fps.motor} ${n(l.etapas.motor)} · ${t.fps.cursor} ${n(l.etapas.cursor)} · ${t.fps.pintura} ${n(l.etapas.pintura)} ms · ${t.fps.trabalho} ${n(l.etapas.total)} · ${t.fps.webview} ${n(l.webview)} ms${contexto ? ` · ${contexto}` : ''}`;
  };
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      guardarFps(el.hidden);
    }
  });
  // Ligado/desligado pelas Configurações ou pelo atalho: os dois passam por `guardarFps`.
  window.addEventListener('feathercad-fps', (e) => {
    el.hidden = !(e as CustomEvent<boolean>).detail;
    ligarPerfil(!el.hidden);
    desenhar();
  });
  // Atualiza o texto 4 vezes por segundo (mais que isso só gastaria o que se quer medir).
  setInterval(desenhar, 250);
  return {
    el,
    quadro() {
      conta.quadro(agora());
    },
    info(texto) {
      contexto = texto;
    },
    medir(etapa, f) {
      const ini = agora();
      try {
        return f();
      } finally {
        conta.tempo(etapa, agora() - ini, agora());
      }
    },
  };
}
