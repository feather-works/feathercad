/**
 * A largada: escolhe o canal até o motor (o `ponte://` do aplicativo, ou o
 * mesmo motor em WebAssembly no navegador) e monta a janela.
 */

import { canalTauri, canalWasm, type Canal } from './ponte/canal.ts';
import { Motor } from './ponte/index.ts';
import { iniciarApp } from './ui/app.ts';
import { ligarAtalhosDoMonitor } from './ui/monitor-atalhos.ts';
import { aplicarAparencia } from './ui/preferencias.ts';
import { iniciarTema } from './ui/tema.ts';

async function escolherCanal(): Promise<Canal> {
  if ('__TAURI_INTERNALS__' in window) return canalTauri();
  // No navegador (`npm run dev`) o motor Rust vem como WebAssembly.
  const url = (await import('../crates/feathercad-ponte/target/wasm32-unknown-unknown/release/feathercad_ponte.wasm?url')).default;
  const modulo = await WebAssembly.compileStreaming(fetch(url));
  return canalWasm(modulo);
}

// O tema antes de tudo, para a janela não piscar na cor errada.
iniciarTema();
aplicarAparencia();

// O console captura `console.*` desde já; o F12 abre o Monitor.
ligarAtalhosDoMonitor();

const raiz = document.getElementById('app');
if (!raiz) throw new Error('faltou o #app no index.html');
const canal = await escolherCanal();
iniciarApp(raiz, new Motor(canal));
