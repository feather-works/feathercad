/**
 * O que só existe dentro da janela nativa (Tauri): abrir a janela do Monitor,
 * medir a memória, ler as métricas do Rust. Vai direto em
 * `window.__TAURI_INTERNALS__.invoke`, sem o pacote `@tauri-apps/api` (o
 * invólucro oficial é uma linha em cima disso; a regra da casa é não ter
 * dependência de execução). No navegador nada disto existe: `noApp()` dá `false`.
 *
 * Isto NÃO é a ponte do motor (`canal.ts`, um XHR síncrono por comando): é a
 * conversa com a casca do aplicativo, e é assíncrona.
 */

import type { OpcoesPdf } from './tipos.ts';

interface TauriInternals {
  invoke(cmd: string, args?: Record<string, unknown>, options?: unknown): Promise<unknown>;
}

function internals(): TauriInternals | null {
  const g = globalThis as unknown as { __TAURI_INTERNALS__?: TauriInternals };
  return g.__TAURI_INTERNALS__ && typeof g.__TAURI_INTERNALS__.invoke === 'function' ? g.__TAURI_INTERNALS__ : null;
}

/** Estamos dentro da janela nativa? */
export function noApp(): boolean {
  return internals() !== null;
}

async function chamar<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const t = internals();
  if (!t) throw new Error('Esta janela não é o aplicativo (sem acesso à casca nativa)');
  return (await t.invoke(cmd, args)) as T;
}

/**
 * Um comando do motor de resposta grande (o quadro), pelo `invoke` **assíncrono** do Tauri: o Rust roda em outra thread
 * e devolve o corpo **cru** (`ipc::Response` → `ArrayBuffer`, sem JSON nem texto) — o padrão do `ler_arquivo` do
 * FeatherSheet. O XHR síncrono do `ponte://` entrega bytes como texto, e converter 10 MB assim custava ~100 ms.
 * Se o protocolo binário falhar, o Tauri cai sozinho para o caminho antigo (um array de números), que ainda funciona.
 */
export async function ponteBinaria(comando: string, corpo: unknown): Promise<Uint8Array> {
  const r = await chamar<ArrayBuffer | number[] | Uint8Array>('ponte_binaria', { comando, corpo: JSON.stringify(corpo) });
  if (r instanceof ArrayBuffer) return new Uint8Array(r);
  return r instanceof Uint8Array ? r : new Uint8Array(r);
}

// ---------------------------------------------------------------------------
// Arquivos e página inicial (`src-tauri/src/arquivos.rs`). O que abre e grava é o
// Rust; aqui só o pedido. Cada um devolve o **estado novo** do motor, ou `null`
// se o usuário cancelou a caixa.
// ---------------------------------------------------------------------------

import type { Estado } from './tipos.ts';

/** Um desenho no disco, como a página inicial o mostra. */
export interface ItemDeArquivo {
  nome: string;
  caminho: string;
  tipo: 'dxf' | 'fcdz';
  modificado_ms: number;
  criado_ms: number | null;
  pasta: string;
}

export interface RecenteDescrito extends ItemDeArquivo {
  aberto_em_ms: number;
}

export interface EstadoInicio {
  recentes: RecenteDescrito[];
  projetos: string[];
}

/** Abre o desenho do `caminho`, ou o que o usuário escolher na caixa. */
export function desenhoAbrir(caminho?: string): Promise<Estado | null> {
  return chamar<Estado | null>('desenho_abrir', { caminho: caminho ?? null });
}

/** Salva; `como` força a caixa "Salvar como". `formatoPadrao` vale para arquivo novo. */
export function desenhoSalvar(como: boolean, formatoPadrao: 'dxf' | 'fcdz'): Promise<Estado | null> {
  return chamar<Estado | null>('desenho_salvar', { como, formatoPadrao });
}

/** O que o Rust achou ao olhar o arquivo aberto no disco (`null` = nada mudou). */
export type MudancaNoDisco = { recarregado: Estado } | { conflito: string };

/** O arquivo aberto mudou por fora (o servidor MCP, por exemplo)? O Rust decide e, se dá, já recarrega. */
export const desenhoVerificarDisco = (): Promise<MudancaNoDisco | null> => chamar<MudancaNoDisco | null>('desenho_verificar_disco');

/** Exporta o desenho em PDF (uma página por folha) num arquivo escolhido na caixa; `null` se cancelou. */
export const desenhoExportarPdf = (opcoes: OpcoesPdf): Promise<{ caminho: string; bytes: number } | null> =>
  chamar<{ caminho: string; bytes: number } | null>('desenho_exportar_pdf', { opcoes });

export const inicioEstado = (): Promise<EstadoInicio> => chamar<EstadoInicio>('inicio_estado');
export const inicioRecenteRemover = (caminho: string): Promise<EstadoInicio> => chamar<EstadoInicio>('inicio_recente_remover', { caminho });
export const inicioProjetoAdicionar = (): Promise<EstadoInicio> => chamar<EstadoInicio>('inicio_projeto_adicionar');
export const inicioProjetoRemover = (pasta: string): Promise<EstadoInicio> => chamar<EstadoInicio>('inicio_projeto_remover', { pasta });
/** A miniatura (SVG) de um desenho do disco; texto vazio se não há o que mostrar. */
export const inicioMiniatura = (caminho: string): Promise<string> => chamar<string>('inicio_miniatura', { caminho });
export const inicioPastaListar = (pasta: string): Promise<ItemDeArquivo[]> => chamar<ItemDeArquivo[]>('inicio_pasta_listar', { pasta });

/** Um processo da árvore do aplicativo, como o Rust o mede. */
export interface ProcessoMedido {
  pid: number;
  nome: string;
  rss: number;
  /** Pico histórico deste processo (`VmHWM`). */
  pico: number;
}

/** O retrato de memória da árvore inteira. Bytes. */
export interface MemoriaArvore {
  total: number;
  principal: number;
  webview: number;
  pico: number;
  pico_principal: number;
  processos: ProcessoMedido[];
}

/**
 * Memória da árvore inteira (o principal e os processos de WebView, onde a
 * interface mora), em bytes. `null` fora do Linux. Ver `src-tauri/src/memoria_arvore.rs`.
 */
export async function usoMemoriaArvore(): Promise<MemoriaArvore | null> {
  return chamar<MemoriaArvore | null>('uso_memoria_arvore');
}

/** As linhas `[motor]` do Rust depois do número `desde`: `[número, linha]`. */
export async function metricasDoMotor(desde: number): Promise<[number, string][]> {
  return chamar<[number, string][]>('metricas_do_motor', { desde });
}

/** Abre (ou traz para a frente) a janela do Monitor. `false` fora do aplicativo. */
export async function abrirJanelaMonitor(): Promise<boolean> {
  if (!noApp()) return false;
  await chamar<void>('abrir_janela_monitor');
  return true;
}

// ---------------------------------------------------------------------------
// O servidor MCP (`src-tauri/src/mcp.rs`): onde está o programa e se ele responde. Quem o abre é o
// cliente de IA; o aplicativo só mostra o comando pronto e faz um teste de conversa.
// ---------------------------------------------------------------------------

export interface EstadoDoMcp {
  caminho: string;
  existe: boolean;
}

export interface ResultadoDoTeste {
  ok: boolean;
  ferramentas: number;
  mensagem: string;
}

export const mcpEstado = (): Promise<EstadoDoMcp> => chamar<EstadoDoMcp>('mcp_estado');
export const mcpTestar = (): Promise<ResultadoDoTeste> => chamar<ResultadoDoTeste>('mcp_testar');
