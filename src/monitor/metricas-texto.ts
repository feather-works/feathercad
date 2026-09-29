/**
 * Lê uma linha de métrica — as que o app imprime (`[ação]`, `[quadro]`,
 * `[abrir]`… em `src/ponte/metricas.ts`) e as do Rust (`[motor]`,
 * `src-tauri/src/metricas.rs`) — para a tabela da página Monitor. Puro.
 */

export interface Metrica {
  /** O que vem entre colchetes: `ação`, `quadro`, `motor`… */
  origem: string;
  /** O resto da linha. */
  texto: string;
  /** O primeiro tempo da linha, em ms (`— 54 ms`), ou null. */
  ms: number | null;
}

/** `null` quando a linha não é de métrica (não começa com `[algo]`). */
export function lerMetrica(linha: string): Metrica | null {
  const m = linha.match(/^\[([^\]\s][^\]]*)\]\s*([\s\S]*)$/);
  if (!m) return null;
  const texto = m[2]!.trim();
  const tempo = texto.match(/(\d+(?:[.,]\d+)?)\s*ms\b/);
  return { origem: m[1]!, texto, ms: tempo ? Number(tempo[1]!.replace(',', '.')) : null };
}
