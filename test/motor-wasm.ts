/**
 * Um motor novo por teste, rodando o `feathercad_ponte.wasm` — o mesmo motor
 * Rust do aplicativo, compilado por `npm run ponte:wasm`. Teste de
 * comportamento passa por aqui, nunca por um dublê em TypeScript.
 */

import { readFileSync } from 'node:fs';
import { canalWasm } from '../src/ponte/canal.ts';
import { Motor } from '../src/ponte/index.ts';
import { comandosDe, regionalDe } from '../src/idiomas/index.ts';

const caminho = new URL('../crates/feathercad-ponte/target/wasm32-unknown-unknown/release/feathercad_ponte.wasm', import.meta.url);
const modulo = new WebAssembly.Module(readFileSync(caminho));

/** Um motor limpo, já com a língua (`pt-BR` por padrão) configurada. */
export function motorNovo(codigo = 'pt-BR'): Motor {
  const m = new Motor(canalWasm(modulo));
  m.idioma(regionalDe(codigo), comandosDe(codigo));
  return m;
}
