/**
 * Aba Métricas da página Monitor: as linhas `[ação]`, `[quadro]`, `[abrir]`…
 * da interface e `[motor]` do Rust numa tabela, em vez de soltas no
 * terminal — filtrável pela origem, só as lentas, e ordenável pelo tempo.
 * É onde se acha a trava que não é do motor ("ms fora").
 */

import { formatar, textos } from '../idiomas/index.ts';
import { lerMetrica, type Metrica } from './metricas-texto.ts';

interface Linha extends Metrica {
  hora: string;
}

const TETO = 2000;
const LENTA_MS = 100;

export class AbaMetricas {
  readonly el = document.createElement('section');
  private readonly tabela = document.createElement('table');
  private readonly origens = document.createElement('select');
  private readonly linhas: Linha[] = [];
  private soLentas = false;
  private porTempo = false;

  constructor() {
    const t = textos().monitor.metricas;
    this.el.className = 'mon-aba mon-metricas';
    const barra = document.createElement('div');
    barra.className = 'mon-barra';
    this.origens.addEventListener('change', () => this.render());
    const lentas = document.createElement('label');
    const c = document.createElement('input');
    c.type = 'checkbox';
    c.addEventListener('change', () => {
      this.soLentas = c.checked;
      this.render();
    });
    lentas.append(c, document.createTextNode(` ${formatar(t.soLentas, { ms: LENTA_MS })}`));
    const ordem = document.createElement('button');
    ordem.textContent = t.ordenar;
    ordem.addEventListener('click', () => {
      this.porTempo = !this.porTempo;
      ordem.setAttribute('aria-pressed', String(this.porTempo));
      this.render();
    });
    const limpar = document.createElement('button');
    limpar.textContent = t.limpar;
    limpar.addEventListener('click', () => {
      this.linhas.length = 0;
      this.render();
    });
    barra.append(this.origens, lentas, ordem, limpar);
    this.tabela.className = 'mon-tabela';
    const rolagem = document.createElement('div');
    rolagem.className = 'mon-lista';
    rolagem.append(this.tabela);
    this.el.append(barra, rolagem);
    this.render();
  }

  /** Uma linha de texto qualquer: só entra se for de métrica. */
  receber(texto: string, hora: string): void {
    const m = lerMetrica(texto);
    if (!m) return;
    this.linhas.push({ ...m, hora });
    if (this.linhas.length > TETO) this.linhas.shift();
    this.render();
  }

  private render(): void {
    const escolhida = this.origens.value || 'todas';
    const origens = ['todas', ...new Set(this.linhas.map((l) => l.origem))].sort((a, b) => (a === 'todas' ? -1 : b === 'todas' ? 1 : a.localeCompare(b)));
    this.origens.replaceChildren(
      ...origens.map((o) => {
        const op = document.createElement('option');
        op.value = o;
        op.textContent = o === 'todas' ? textos().monitor.metricas.todasOrigens : `[${o}]`;
        return op;
      }),
    );
    this.origens.value = origens.includes(escolhida) ? escolhida : 'todas';
    let vistas = this.linhas.filter(
      (l) => (this.origens.value === 'todas' || l.origem === this.origens.value) && (!this.soLentas || (l.ms ?? 0) >= LENTA_MS),
    );
    vistas = this.porTempo ? [...vistas].sort((a, b) => (b.ms ?? -1) - (a.ms ?? -1)) : [...vistas].reverse();
    const cab = document.createElement('tr');
    const col = textos().monitor.metricas.colunas;
    for (const t of [col.hora, col.origem, col.oQueFoi, col.tempo]) {
      const th = document.createElement('th');
      th.textContent = t;
      cab.append(th);
    }
    this.tabela.replaceChildren(
      cab,
      ...vistas.slice(0, 500).map((l) => {
        const tr = document.createElement('tr');
        if ((l.ms ?? 0) >= LENTA_MS) tr.className = 'mon-lenta';
        for (const t of [l.hora, `[${l.origem}]`, l.texto, l.ms === null ? '' : `${l.ms} ms`]) {
          const td = document.createElement('td');
          td.textContent = t;
          tr.append(td);
        }
        return tr;
      }),
    );
  }
}
