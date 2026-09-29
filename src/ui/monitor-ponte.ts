/**
 * A janela principal falando com a página Monitor (`src/monitor/`): cada entrada do console (e com ela as métricas `[ação]`,
 * `[quadro]`… que passam pelo `console.log`) vai pelo `BroadcastChannel`, e
 * quando a página abre ela pede o histórico do que aconteceu antes.
 */

import { devConsole, type LogEntry } from './console-depuracao.ts';

export const CANAL_DO_MONITOR = 'feathercad_monitor';

/** O que passa pelo canal. */
export type MensagemDoMonitor =
  | { tipo: 'log'; entrada: LogEntry }
  | { tipo: 'historico'; entradas: LogEntry[] }
  | { tipo: 'pedir-historico' };

/** Liga a janela principal ao canal. Sem `BroadcastChannel` (testes antigos), não faz nada. */
export function ligarMonitor(): void {
  if (typeof BroadcastChannel === 'undefined') return;
  const canal = new BroadcastChannel(CANAL_DO_MONITOR);
  devConsole.ouvir((entrada) => canal.postMessage({ tipo: 'log', entrada } satisfies MensagemDoMonitor));
  canal.onmessage = (ev: MessageEvent<MensagemDoMonitor>) => {
    if (ev.data?.tipo === 'pedir-historico') {
      canal.postMessage({ tipo: 'historico', entradas: devConsole.historico() } satisfies MensagemDoMonitor);
    }
  };
}
