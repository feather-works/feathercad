/**
 * O perfil de quadro: cronometra **cada etapa** de um quadro (ponte, motor, decodificação, pintura, DOM) com
 * `performance.now()` e resume no console (F12 / Monitor) — para ver onde o movimento e o zoom agarram.
 *
 * Ligado junto com o medidor de FPS (Configurações → Sobre, ou Ctrl+Shift+F). Desligado, `etapa` só chama a
 * função (custo desprezível). Nomes com `/` são partes de outra etapa (`quadro/canal` está dentro de `quadro`),
 * e o "resto" de um quadro é o que não coube em nenhuma etapa de primeiro nível.
 *
 * Saída, uma linha por segundo e por tipo de quadro:
 *   `[perfil] zoom: 31 quadros, média 9,2 ms, pior 41,0 ms | vista_zoom 0,3 · quadro 2,1 (…) · pintar_fundo 5,0 · resto 0,4`
 * e, para um quadro individual acima de `LENTO_MS`:
 *   `[perfil] quadro lento (zoom): 72,1 ms → pintar_fundo 60,0 · quadro 8,0 · …`
 */

/** Um quadro acima disto é impresso sozinho, com a lista das suas etapas. */
export const LENTO_MS = 33;
/** De quanto em quanto tempo o resumo sai. */
const RESUMO_MS = 1000;

interface Soma {
  total: number;
  pior: number;
}

interface Grupo {
  quadros: number;
  total: Soma;
  etapas: Map<string, Soma>;
  desde: number;
}

let ligado = false;
let atual: Map<string, number> | null = null;
const grupos = new Map<string, Grupo>();

/** Liga ou desliga o perfil. */
export function ligarPerfil(v: boolean): void {
  ligado = v;
  if (!v) {
    atual = null;
    grupos.clear();
  }
}

export function perfilLigado(): boolean {
  return ligado;
}

/** Cronometra `f` como a etapa `nome` do quadro em andamento (soma, se a etapa se repete no quadro). */
export function etapa<T>(nome: string, f: () => T): T {
  if (!ligado || atual === null) return f();
  const registro = atual;
  const inicio = performance.now();
  try {
    return f();
  } finally {
    registro.set(nome, (registro.get(nome) ?? 0) + (performance.now() - inicio));
  }
}

/** Soma `ms` (medidos de outro jeito, por exemplo uma resposta assíncrona) à etapa `nome` do quadro em andamento. */
export function registrar(nome: string, ms: number): void {
  if (!ligado || atual === null) return;
  atual.set(nome, (atual.get(nome) ?? 0) + ms);
}

const ms = (v: number): string => v.toFixed(1).replace('.', ',');

/** Só as etapas de primeiro nível (sem `/`) somam o total; as demais são detalhe. */
function primeiroNivel(nome: string): boolean {
  return !nome.includes('/');
}

function listar(etapas: Iterable<[string, number]>): string {
  return [...etapas]
    .sort((a, b) => b[1] - a[1])
    .map(([nome, v]) => `${nome} ${ms(v)}`)
    .join(' · ');
}

function somar(s: Soma, v: number): void {
  s.total += v;
  if (v > s.pior) s.pior = v;
}

function imprimirResumo(tipo: string, g: Grupo): void {
  const medias = [...g.etapas].map(([nome, s]) => [nome, s.total / g.quadros] as [string, number]);
  const somaDoPrimeiroNivel = medias.filter(([nome]) => primeiroNivel(nome)).reduce((a, [, v]) => a + v, 0);
  const media = g.total.total / g.quadros;
  const resto = Math.max(0, media - somaDoPrimeiroNivel);
  const pioresPorEtapa = [...g.etapas].map(([nome, s]) => [nome, s.pior] as [string, number]).sort((a, b) => b[1] - a[1]).slice(0, 3);
  console.log(
    `[perfil] ${tipo}: ${g.quadros} quadros, média ${ms(media)} ms, pior ${ms(g.total.pior)} ms | ${listar([...medias, ['resto', resto]])} | maiores picos: ${listar(pioresPorEtapa)}`,
  );
}

/**
 * Um quadro de tipo `tipo` (`zoom`, `pan`, `mouse`…): tudo o que `f` faz é cronometrado por etapa. Quando o
 * segundo acaba, sai o resumo do tipo; um quadro acima de `LENTO_MS` sai na hora.
 */
export function quadroDePerfil<T>(tipo: string, f: () => T): T {
  if (!ligado) return f();
  const registro = new Map<string, number>();
  atual = registro;
  const inicio = performance.now();
  try {
    return f();
  } finally {
    const fim = performance.now();
    const duracao = fim - inicio;
    atual = null;
    const g = grupos.get(tipo) ?? { quadros: 0, total: { total: 0, pior: 0 }, etapas: new Map(), desde: inicio };
    grupos.set(tipo, g);
    g.quadros++;
    somar(g.total, duracao);
    for (const [nome, v] of registro) {
      const s = g.etapas.get(nome) ?? { total: 0, pior: 0 };
      g.etapas.set(nome, s);
      somar(s, v);
    }
    if (duracao > LENTO_MS) {
      const resto = Math.max(0, duracao - [...registro].filter(([nome]) => primeiroNivel(nome)).reduce((a, [, v]) => a + v, 0));
      console.log(`[perfil] quadro lento (${tipo}): ${ms(duracao)} ms → ${listar([...registro, ['resto', resto]])}`);
    }
    if (fim - g.desde >= RESUMO_MS) {
      imprimirResumo(tipo, g);
      grupos.delete(tipo);
    }
  }
}
