/**
 * Aba Memória da página Monitor: a árvore de processos do aplicativo como o
 * Rust mede (`src-tauri/src/memoria_arvore.rs`) — o processo principal e o
 * da página (no Linux o JavaScript roda num processo FILHO, e foi medir o
 * processo errado que escondeu picos de gigabytes em 22/09/2026). Atualiza
 * a cada segundo, com o desenho dos últimos dois minutos.
 */

import { usoMemoriaArvore, type MemoriaArvore } from '../ponte/nativo.ts';
import { codigoAtual, formatar, textos } from '../idiomas/index.ts';

const AMOSTRAS = 120;

export const mb = (bytes: number): string => `${(bytes / 1048576).toLocaleString(codigoAtual(), { maximumFractionDigits: 0 })} MB`;

export class AbaMemoria {
  readonly el = document.createElement('section');
  private readonly cartoes = document.createElement('div');
  private readonly grafico = document.createElement('canvas');
  private readonly tabela = document.createElement('table');
  private readonly historico: number[] = [];

  constructor() {
    this.el.className = 'mon-aba mon-memoria';
    this.cartoes.className = 'mon-cartoes';
    this.grafico.className = 'mon-grafico';
    this.grafico.height = 120;
    this.tabela.className = 'mon-tabela';
    const titulo = document.createElement('h3');
    titulo.textContent = textos().monitor.memoria.processos;
    this.el.append(this.cartoes, this.grafico, titulo, this.tabela);
  }

  /** Mede agora e a cada segundo; devolve o intervalo, para quem fecha. */
  iniciar(): ReturnType<typeof setInterval> {
    void this.medir();
    return setInterval(() => void this.medir(), 1000);
  }

  private async medir(): Promise<void> {
    let m: MemoriaArvore | null = null;
    try {
      m = await usoMemoriaArvore();
    } catch {
      m = null;
    }
    if (!m) {
      this.cartoes.textContent = textos().monitor.memoria.indisponivel;
      return;
    }
    this.historico.push(m.total);
    if (this.historico.length > AMOSTRAS) this.historico.shift();
    this.desenharCartoes(m);
    this.desenharGrafico(m.pico);
    this.desenharTabela(m);
  }

  private desenharCartoes(m: MemoriaArvore): void {
    const cartao = (rotulo: string, valor: string, dica: string) => {
      const c = document.createElement('div');
      c.className = 'mon-cartao';
      c.title = dica;
      const v = document.createElement('strong');
      v.textContent = valor;
      const r = document.createElement('span');
      r.textContent = rotulo;
      c.append(v, r);
      return c;
    };
    const c = textos().monitor.memoria.cartoes;
    this.cartoes.replaceChildren(
      cartao(c.total.rotulo, mb(m.total), c.total.dica),
      cartao(c.principal.rotulo, mb(m.principal), c.principal.dica),
      cartao(c.webview.rotulo, mb(m.webview), c.webview.dica),
      cartao(c.pico.rotulo, mb(m.pico), c.pico.dica),
    );
  }

  private desenharGrafico(pico: number): void {
    const c = this.grafico;
    c.width = c.clientWidth || 600;
    const ctx = c.getContext('2d');
    if (!ctx || this.historico.length < 2) return;
    const teto = Math.max(pico, ...this.historico) * 1.05;
    const x = (i: number) => (i / (AMOSTRAS - 1)) * c.width;
    const y = (v: number) => c.height - (v / teto) * (c.height - 8) - 4;
    ctx.clearRect(0, 0, c.width, c.height);
    const estilo = getComputedStyle(c);
    ctx.strokeStyle = estilo.getPropertyValue('--destaque').trim() || '#1a73e8';
    ctx.lineWidth = 2;
    ctx.beginPath();
    const inicio = AMOSTRAS - this.historico.length;
    this.historico.forEach((v, i) => (i === 0 ? ctx.moveTo(x(inicio + i), y(v)) : ctx.lineTo(x(inicio + i), y(v))));
    ctx.stroke();
    ctx.fillStyle = estilo.getPropertyValue('--texto-suave').trim() || '#888';
    ctx.font = '11px sans-serif';
    ctx.fillText(formatar(textos().monitor.memoria.ultimos, { teto: mb(teto) }), 6, 12);
  }

  private desenharTabela(m: MemoriaArvore): void {
    const cab = document.createElement('tr');
    const col = textos().monitor.memoria.colunas;
    for (const t of [col.pid, col.processo, col.agora, col.pico]) {
      const th = document.createElement('th');
      th.textContent = t;
      cab.append(th);
    }
    const linhas = [...m.processos]
      .sort((a, b) => b.rss - a.rss)
      .map((p) => {
        const tr = document.createElement('tr');
        for (const t of [String(p.pid), p.nome, mb(p.rss), mb(p.pico)]) {
          const td = document.createElement('td');
          td.textContent = t;
          tr.append(td);
        }
        return tr;
      });
    this.tabela.replaceChildren(cab, ...linhas);
  }
}
