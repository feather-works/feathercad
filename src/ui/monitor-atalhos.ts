/**
 * F12 (e Alt+C, Ctrl+Shift+J, Ctrl+Shift+M) abre o Monitor — o que o FeatherSheet
 * já tinha e o usuário mandou manter (29/09/2026). No aplicativo abre a página
 * Monitor numa janela à parte (memória, console e métricas); no navegador, onde
 * não há janela nativa, o painel do console por cima do desenho.
 *
 * **Diferença do AutoCAD, de propósito:** lá F12 alterna a entrada dinâmica. Aqui a
 * tecla é do Monitor; a entrada dinâmica ganha outra tecla quando o botão existir
 * (`docs/ATALHOS.md`). Abrir uma janela não é comando do desenho, por isso esta
 * tecla é tratada aqui e não pelo `atalho` do motor.
 */

import { abrirJanelaMonitor } from '../ponte/nativo.ts';
import { devConsole } from './console-depuracao.ts';
import { ligarMonitor } from './monitor-ponte.ts';

type Tecla = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey'>;

/** A tecla é um dos atalhos do Monitor? */
export function ehAtalhoDoMonitor(e: Tecla): boolean {
  const letra = e.key.toLowerCase();
  return e.key === 'F12' || (e.altKey && letra === 'c') || (e.ctrlKey && e.shiftKey && (letra === 'j' || letra === 'm'));
}

/** Liga os atalhos e a conversa da janela principal com a página Monitor. */
export function ligarAtalhosDoMonitor(): void {
  ligarMonitor();
  window.addEventListener('keydown', (e) => {
    if (!ehAtalhoDoMonitor(e)) return;
    e.preventDefault();
    void abrirJanelaMonitor().then((aberta) => {
      if (!aberta) devConsole.toggle();
    });
  });
}
