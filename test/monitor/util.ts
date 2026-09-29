import { motorNovo } from '../motor-wasm.ts';

/** O motor trata a tecla F12? (Não deve: ela abre o Monitor, fora do motor.) */
export function acaoDoMotorParaF12(): boolean {
  return motorNovo().atalho('F12').tratado;
}
