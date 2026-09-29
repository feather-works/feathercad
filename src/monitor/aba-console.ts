/**
 * Aba Console da página Monitor: o que o F12 mostrava por cima da planilha
 * (`src/ui/console-depuracao.ts`) — erros, avisos e `console.log` da janela
 * principal —, agora numa janela à parte, com filtro por nível e busca.
 */

import type { LogEntry } from '../ui/console-depuracao.ts';
import { textos } from '../idiomas/index.ts';

type Nivel = 'all' | LogEntry['level'];
const TETO = 2000;

export class AbaConsole {
  readonly el = document.createElement('section');
  private readonly lista = document.createElement('div');
  private readonly entradas: LogEntry[] = [];
  private nivel: Nivel = 'all';
  private busca = '';

  constructor() {
    this.el.className = 'mon-aba mon-console';
    const barra = document.createElement('div');
    barra.className = 'mon-barra';
    const c = textos().monitor.console;
    const niveis: [Nivel, string][] = [['all', c.tudo], ['error', c.erros], ['warn', c.avisos], ['info', c.info]];
    for (const [n, rotulo] of niveis) {
      const b = document.createElement('button');
      b.textContent = rotulo;
      b.dataset.nivel = n;
      b.setAttribute('aria-pressed', String(n === this.nivel));
      b.addEventListener('click', () => {
        this.nivel = n;
        for (const irmao of barra.querySelectorAll('button[data-nivel]')) irmao.setAttribute('aria-pressed', String(irmao === b));
        this.render();
      });
      barra.append(b);
    }
    const busca = document.createElement('input');
    busca.type = 'search';
    busca.placeholder = c.buscar;
    busca.addEventListener('input', () => {
      this.busca = busca.value.toLowerCase();
      this.render();
    });
    const limpar = document.createElement('button');
    limpar.textContent = c.limpar;
    limpar.addEventListener('click', () => {
      this.entradas.length = 0;
      this.render();
    });
    barra.append(busca, limpar);
    this.lista.className = 'mon-lista';
    this.el.append(barra, this.lista);
  }

  /** Entradas novas (ou o histórico, na abertura). Repetidas pelo `id` são ignoradas. */
  receber(novas: LogEntry[]): void {
    const vistos = new Set(this.entradas.map((e) => e.id));
    for (const e of novas) if (!vistos.has(e.id)) this.entradas.push(e);
    this.entradas.sort((a, b) => a.id - b.id);
    if (this.entradas.length > TETO) this.entradas.splice(0, this.entradas.length - TETO);
    this.render();
  }

  get quantosErros(): number {
    return this.entradas.filter((e) => e.level === 'error').length;
  }

  private render(): void {
    const noFim = this.lista.scrollTop + this.lista.clientHeight >= this.lista.scrollHeight - 20;
    const visiveis = this.entradas.filter(
      (e) => (this.nivel === 'all' || e.level === this.nivel) && (!this.busca || e.message.toLowerCase().includes(this.busca)),
    );
    this.lista.replaceChildren(
      ...visiveis.map((e) => {
        const item = document.createElement(e.stack ? 'details' : 'div');
        item.className = `mon-log mon-${e.level}`;
        const linha = document.createElement(e.stack ? 'summary' : 'div');
        const hora = document.createElement('time');
        hora.textContent = e.timestamp;
        const msg = document.createElement('span');
        msg.textContent = e.message;
        linha.append(hora, msg);
        item.append(linha);
        if (e.stack) {
          const pilha = document.createElement('pre');
          pilha.textContent = e.stack;
          item.append(pilha);
        }
        return item;
      }),
    );
    if (noFim) this.lista.scrollTop = this.lista.scrollHeight;
  }
}
