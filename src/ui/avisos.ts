/**
 * Os avisos do último abrir/salvar (objetos que ficaram de fora, cores
 * aproximadas…) vão para o **console** — é ele que a aba Console do Monitor (F12)
 * mostra e que chega ao terminal do aplicativo. Ficar só na linha de comando não
 * bastava: quem abre o Monitor procurando o que faltou no desenho não achava.
 */

import type { Estado } from '../ponte/index.ts';
import { texto } from '../idiomas/index.ts';

/** Registra cada aviso do estado, traduzido. `registrar` é o `console.warn` (troca nos testes). */
export function registrarAvisos(e: Estado, nomeDoArquivo: string, registrar: (msg: string) => void = (m) => console.warn(m)): void {
  // `?? []`: um motor mais antigo (app não recompilado) não manda o campo; abrir o arquivo não pode falhar por causa do registro.
  for (const a of e.avisos_do_arquivo ?? []) registrar(`[arquivo] ${nomeDoArquivo}: ${texto(a.chave, a.vars)}`);
}
