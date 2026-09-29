/**
 * Os textos da interface na língua escolhida (`textos()`), com a referência
 * preenchendo o que a língua não traduziu. Ver `registro.ts`.
 */

import { COMANDOS, IDIOMAS, REFERENCIA, type TabelaDeComandos, type Textos } from './registro.ts';
import type { Regional } from '../ponte/tipos.ts';

export { COMANDOS, IDIOMAS, REFERENCIA, type Textos } from './registro.ts';

const CHAVE = 'feathercad.idioma';
const prontos = new Map<string, Textos>();

function mesclar(base: unknown, por: unknown): unknown {
  if (por === undefined) return base;
  if (typeof base !== 'object' || base === null) return por;
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(por as Record<string, unknown>)) out[k] = mesclar(out[k], v);
  return out;
}

/** A língua em uso: a escolhida e guardada, senão a do sistema, senão a referência. */
export function codigoAtual(): string {
  try {
    const guardada = localStorage.getItem(CHAVE);
    if (guardada && guardada in IDIOMAS) return guardada;
  } catch {
    // sem armazenamento (janela privada): segue para a do sistema
  }
  const sistema = typeof navigator === 'undefined' ? '' : navigator.language;
  return sistema in IDIOMAS ? sistema : REFERENCIA;
}

/** Guarda a língua escolhida (vale na próxima largada: a interface recarrega). */
export function guardarIdioma(codigo: string): void {
  try {
    localStorage.setItem(CHAVE, codigo);
  } catch {
    // sem armazenamento: vale a do sistema
  }
}

const CHAVE_APELIDOS = 'feathercad.apelidos';

/** Os apelidos digitados que o usuário acrescentou (`{ mover: ['G'] }`), além dos da língua. */
export function apelidosDoUsuario(): Record<string, string[]> {
  try {
    const bruto: unknown = JSON.parse(localStorage.getItem(CHAVE_APELIDOS) ?? '{}');
    if (typeof bruto !== 'object' || bruto === null) return {};
    const saida: Record<string, string[]> = {};
    for (const [id, v] of Object.entries(bruto)) if (Array.isArray(v)) saida[id] = v.filter((a): a is string => typeof a === 'string' && a.trim() !== '');
    return saida;
  } catch {
    return {};
  }
}

export function guardarApelidosDoUsuario(mapa: Record<string, string[]>): void {
  try {
    localStorage.setItem(CHAVE_APELIDOS, JSON.stringify(mapa));
  } catch {
    // sem armazenamento: vale até fechar a janela
  }
}

/** Os textos de uma língua (a atual, sem argumento), completos. */
export function textos(codigo: string = codigoAtual()): Textos {
  let t = prontos.get(codigo);
  if (!t) {
    t = mesclar(IDIOMAS[REFERENCIA], IDIOMAS[codigo]) as Textos;
    prontos.set(codigo, t);
  }
  return t;
}

/** Troca `{nome}` pelos valores: `formatar('Comando {nome}', { nome: 'X' })`. */
export function formatar(texto: string, valores: Record<string, string | number>): string {
  return texto.replace(/\{(\w+)\}/g, (inteiro, k: string) => (k in valores ? String(valores[k]) : inteiro));
}

/** O texto de uma chave com ponto (`linha.primeiroPonto`), com as variáveis trocadas. */
export function texto(chave: string, vars: Record<string, string> = {}, codigo?: string): string {
  let no: unknown = textos(codigo);
  for (const parte of chave.split('.')) {
    no = typeof no === 'object' && no !== null ? (no as Record<string, unknown>)[parte] : undefined;
  }
  return typeof no === 'string' ? formatar(no, vars) : chave;
}

/** A configuração regional da língua, no formato que o Rust espera. */
export function regionalDe(codigo: string = codigoAtual()): Regional {
  const i = textos(codigo).idioma;
  return { decimal: i.decimal, separador_de_coordenadas: i.separadorDeCoordenadas, casas: i.casas };
}

/** A tabela de comandos da língua, para mandar ao Rust. */
export function comandosDe(codigo: string = codigoAtual()): TabelaDeComandos {
  return comBase(comandosPadraoDe(codigo));
}

/** A tabela da língua, **sem** os apelidos do usuário. */
export function comandosPadraoDe(codigo: string = codigoAtual()): TabelaDeComandos {
  return COMANDOS[codigo] ?? COMANDOS[REFERENCIA]!;
}

/** Junta os apelidos do usuário à tabela da língua (o Rust recebe tudo junto). */
function comBase(base: TabelaDeComandos): TabelaDeComandos {
  const extras = apelidosDoUsuario();
  const comandos: Record<string, string[]> = {};
  for (const [id, nomes] of Object.entries(base.comandos)) comandos[id] = [...nomes, ...(extras[id] ?? []).filter((a) => !nomes.includes(a.toUpperCase()))];
  return { comandos, opcoes: base.opcoes };
}
