/**
 * A página Monitor (copiada do FeatherSheet em 29/09/2026): o monitor de memória e o
 * que o F12 mostrava, juntos numa janela à parte — como a do OpenQuery. Três
 * abas: Memória (a árvore de processos, pelo Rust), Console (as entradas da
 * janela principal, pelo `BroadcastChannel` de `ui/app/monitor-ponte.ts`) e
 * Métricas (as linhas `[ação]`, `[quadro]`… da interface e `[motor]` do
 * Rust, numa tabela).
 */

import { formatar, textos } from '../idiomas/index.ts';
import { metricasDoMotor } from '../ponte/nativo.ts';
import { BASE_CSS } from '../ui/base.ts';
import { CANAL_DO_MONITOR, type MensagemDoMonitor } from '../ui/monitor-ponte.ts';
import type { LogEntry } from '../ui/console-depuracao.ts';
import { AbaConsole } from './aba-console.ts';
import { AbaMemoria } from './aba-memoria.ts';
import { AbaMetricas } from './aba-metricas.ts';
import { MONITOR_CSS } from './monitor-css.ts';

/**
 * Monta a página em `raiz` e começa a ouvir. Devolve quem fecha tudo (o
 * canal e os intervalos) — é o que o teste usa; a janela de verdade só
 * fecha junto com ela (`principal.ts`).
 */
export function montarMonitor(raiz: HTMLElement): () => void {
  const intervalos: ReturnType<typeof setInterval>[] = [];
  let canal: BroadcastChannel | null = null;
  const t = textos();
  const estilo = document.createElement('style');
  estilo.textContent = BASE_CSS + MONITOR_CSS;
  document.head.append(estilo);

  raiz.className = 'mon';
  const memoria = new AbaMemoria();
  const consoleAba = new AbaConsole();
  const metricas = new AbaMetricas();

  const topo = document.createElement('header');
  topo.className = 'mon-topo';
  const titulo = document.createElement('h1');
  titulo.textContent = formatar(t.monitor.titulo, { nome: t.app.nome });
  const abas = document.createElement('nav');
  abas.className = 'mon-abas';
  const contador = document.createElement('span');
  contador.className = 'mon-contador';
  contador.hidden = true;
  const paineis: [string, HTMLElement][] = [
    [t.monitor.abas.memoria, memoria.el],
    [t.monitor.abas.console, consoleAba.el],
    [t.monitor.abas.metricas, metricas.el],
  ];
  const botoes = paineis.map(([nome, painel], i) => {
    const b = document.createElement('button');
    b.textContent = nome;
    b.setAttribute('role', 'tab');
    if (painel === consoleAba.el) b.append(contador);
    b.addEventListener('click', () => mostrar(i));
    painel.hidden = true;
    return b;
  });
  const mostrar = (i: number) => {
    paineis.forEach(([, p], j) => (p.hidden = j !== i));
    botoes.forEach((b, j) => b.setAttribute('aria-selected', String(j === i)));
  };
  abas.append(...botoes);
  topo.append(titulo, abas);
  const corpo = document.createElement('main');
  corpo.className = 'mon-corpo';
  corpo.append(...paineis.map(([, p]) => p));
  raiz.append(topo, corpo);
  mostrar(0);

  // Da janela principal: o histórico ao abrir, e cada entrada nova.
  const receber = (entradas: LogEntry[]) => {
    consoleAba.receber(entradas);
    for (const e of entradas) metricas.receber(e.message, e.timestamp);
    const erros = consoleAba.quantosErros;
    contador.hidden = erros === 0;
    contador.textContent = String(erros);
  };
  if (typeof BroadcastChannel !== 'undefined') {
    canal = new BroadcastChannel(CANAL_DO_MONITOR);
    canal.onmessage = (ev: MessageEvent<MensagemDoMonitor>) => {
      if (ev.data?.tipo === 'log') receber([ev.data.entrada]);
      else if (ev.data?.tipo === 'historico') receber(ev.data.entradas);
    };
    canal.postMessage({ tipo: 'pedir-historico' } satisfies MensagemDoMonitor);
  }

  // Do Rust: as linhas [motor], que antes só iam para o terminal.
  let desde = 0;
  const buscarMotor = async () => {
    try {
      for (const [id, linha] of await metricasDoMotor(desde)) {
        desde = Math.max(desde, id);
        metricas.receber(linha, new Date().toLocaleTimeString(undefined, { hour12: false }));
      }
    } catch {
      // Fora do aplicativo não há motor nativo: a aba fica com o que vier da interface.
    }
  };
  void buscarMotor();
  intervalos.push(setInterval(() => void buscarMotor(), 1000));
  intervalos.push(memoria.iniciar());
  return () => {
    canal?.close();
    for (const i of intervalos) clearInterval(i);
  };
}
