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
  private readonly cartoesUso = document.createElement('div');
  private readonly grafico = document.createElement('canvas');
  private readonly graficoUso = document.createElement('canvas');
  private readonly tabela = document.createElement('table');
  private readonly historico: number[] = [];
  private readonly historicoCpu: number[] = [];
  private readonly historicoGpu: number[] = [];

  constructor() {
    this.el.className = 'mon-aba mon-memoria';
    this.cartoes.className = 'mon-cartoes';
    this.cartoesUso.className = 'mon-cartoes';
    this.grafico.className = 'mon-grafico';
    this.grafico.height = 120;
    this.graficoUso.className = 'mon-grafico';
    this.graficoUso.height = 100;
    this.tabela.className = 'mon-tabela';
    const titulo = document.createElement('h3');
    titulo.textContent = textos().monitor.memoria.processos;
    this.el.append(this.cartoes, this.grafico, this.cartoesUso, this.graficoUso, titulo, this.tabela);
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
    this.historicoCpu.push(m.cpu_total);
    this.historicoGpu.push(m.gpu?.uso ?? 0);
    for (const h of [this.historicoCpu, this.historicoGpu]) if (h.length > AMOSTRAS) h.shift();
    this.desenharCartoes(m);
    this.desenharGrafico(m.pico);
    this.desenharGraficoDeUso(m);
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
    const pct = (v: number): string => `${v.toLocaleString(codigoAtual(), { maximumFractionDigits: 0 })} %`;
    const gpu = m.gpu;
    this.cartoesUso.replaceChildren(
      cartao(c.cpu.rotulo, pct(m.cpu_total), formatar(c.cpu.dica, { nucleos: String(m.nucleos) })),
      cartao(c.gpu.rotulo, gpu?.uso != null ? pct(gpu.uso) : '—', gpu ? `${c.gpu.dica} — ${gpu.nome}` : c.gpu.dica),
      cartao(c.vram.rotulo, gpu && gpu.vram_total > 0 ? `${mb(gpu.vram_usada)} / ${mb(gpu.vram_total)}` : '—', c.vram.dica),
    );
  }

  /** CPU (total, em % de um núcleo) e GPU (0–100) dos últimos dois minutos, no mesmo gráfico. */
  private desenharGraficoDeUso(m: MemoriaArvore): void {
    const c = this.graficoUso;
    c.width = c.clientWidth || 600;
    const ctx = c.getContext('2d');
    if (!ctx || this.historicoCpu.length < 2) return;
    const teto = Math.max(100, ...this.historicoCpu, ...this.historicoGpu) * 1.05;
    const x = (i: number) => (i / (AMOSTRAS - 1)) * c.width;
    const y = (v: number) => c.height - (v / teto) * (c.height - 8) - 4;
    ctx.clearRect(0, 0, c.width, c.height);
    const estilo = getComputedStyle(c);
    const linha = (valores: number[], cor: string): void => {
      ctx.strokeStyle = cor;
      ctx.lineWidth = 2;
      ctx.beginPath();
      const inicio = AMOSTRAS - valores.length;
      valores.forEach((v, i) => (i === 0 ? ctx.moveTo(x(inicio + i), y(v)) : ctx.lineTo(x(inicio + i), y(v))));
      ctx.stroke();
    };
    linha(this.historicoCpu, estilo.getPropertyValue('--destaque').trim() || '#1a73e8');
    linha(this.historicoGpu, '#e8710a');
    ctx.fillStyle = estilo.getPropertyValue('--texto-suave').trim() || '#888';
    ctx.font = '11px sans-serif';
    const pct = (v: number): string => `${v.toFixed(0)} %`;
    ctx.fillText(formatar(textos().monitor.memoria.usoGrafico, { cpu: pct(m.cpu_total), gpu: m.gpu?.uso != null ? pct(m.gpu.uso) : '—' }), 6, 12);
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
    for (const t of [col.pid, col.processo, col.agora, col.pico, col.cpu, col.gpu, col.vram]) {
      const th = document.createElement('th');
      th.textContent = t;
      cab.append(th);
    }
    const linhas = [...m.processos]
      .sort((a, b) => b.rss - a.rss)
      .map((p) => {
        const tr = document.createElement('tr');
        const pct = (v: number): string => (v < 0.5 ? '—' : `${v.toFixed(0)} %`);
        for (const t of [String(p.pid), p.nome, mb(p.rss), mb(p.pico), pct(p.cpu), pct(p.gpu), p.vram > 0 ? mb(p.vram) : '—']) {
          const td = document.createElement('td');
          td.textContent = t;
          tr.append(td);
        }
        return tr;
      });
    this.tabela.replaceChildren(cab, ...linhas);
  }
}
