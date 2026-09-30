/**
 * Quem pinta o quadro do desenho para a área de desenho: num **Worker** (`OffscreenCanvas`, a thread principal fica livre)
 * ou, se a webview não suporta, na própria thread principal. As duas formas têm a mesma interface, e as duas devolvem
 * o resultado **depois de a imagem estar instalada na tela** (para a área trocar a matriz CSS no mesmo instante).
 */

import type { Fundo, Vista } from '../ponte/tipos.ts';
import { Nucleo, type Resumo } from './pintor-nucleo.ts';
import type { CoresDaGrade } from './pintura.ts';
import type { PedidoAoWorker, RespostaDoWorker } from './quadro-worker.ts';

export interface PedidoDePintura {
  bytes: Uint8Array;
  vista: Vista;
  modo: Fundo;
  cores: CoresDaGrade;
  comTextos: boolean;
}

export interface Pintor {
  /** `true` se pinta num Worker. */
  readonly emWorker: boolean;
  tamanho(largura: number, altura: number, dpr: number): void;
  pintar(pedido: PedidoDePintura): Promise<Resumo>;
  /** Refaz só os textos (nítidos) com o último quadro. */
  pintarTextos(vista: Vista): Promise<void>;
}

/** A webview sabe pintar num Worker com `OffscreenCanvas` e instalar `ImageBitmap`? */
export function suportaPintarEmSegundoPlano(): boolean {
  try {
    if (typeof Worker === 'undefined' || typeof OffscreenCanvas === 'undefined' || typeof createImageBitmap === 'undefined') return false;
    if (!new OffscreenCanvas(1, 1).getContext('2d')) return false;
    return document.createElement('canvas').getContext('bitmaprenderer') !== null;
  } catch {
    return false;
  }
}

/** Cria o pintor das duas telas. `segundoPlano` falso força a thread principal (Configurações). */
export function criarPintor(fundo: HTMLCanvasElement, textos: HTMLCanvasElement, segundoPlano: boolean): Pintor {
  return segundoPlano && suportaPintarEmSegundoPlano() ? pintorEmWorker(fundo, textos) : pintorNaThreadPrincipal(fundo, textos);
}

function pintorNaThreadPrincipal(fundo: HTMLCanvasElement, textos: HTMLCanvasElement): Pintor {
  const nucleo = new Nucleo(fundo, textos, false);
  return {
    emWorker: false,
    tamanho: (l, a, dpr) => nucleo.tamanho(l, a, dpr),
    pintar: (p) => Promise.resolve(nucleo.pintar(p.bytes, p.vista, p.modo, p.cores, p.comTextos).resumo),
    pintarTextos: (vista) => {
      nucleo.pintarSoTextos(vista);
      return Promise.resolve();
    },
  };
}

function pintorEmWorker(fundo: HTMLCanvasElement, textos: HTMLCanvasElement): Pintor {
  const worker = new Worker(new URL('./quadro-worker.ts', import.meta.url), { type: 'module' });
  const ctxFundo = fundo.getContext('bitmaprenderer');
  const ctxTextos = textos.getContext('bitmaprenderer');
  if (!ctxFundo || !ctxTextos) throw new Error('bitmaprenderer indisponível');
  let proximo = 1;
  const esperando = new Map<number, { ok: (r: Resumo | null) => void; falhou: (e: Error) => void }>();

  worker.onmessage = (e: MessageEvent<RespostaDoWorker>) => {
    const r = e.data;
    const espera = esperando.get(r.id);
    esperando.delete(r.id);
    if (r.t === 'erro') {
      espera?.falhou(new Error(r.mensagem));
      return;
    }
    // Instala as imagens e resolve na mesma tarefa: quem espera troca a matriz CSS logo em seguida, sem um quadro no meio.
    if (r.bitmapFundo) ctxFundo.transferFromImageBitmap(r.bitmapFundo);
    if (r.bitmapTextos) ctxTextos.transferFromImageBitmap(r.bitmapTextos);
    espera?.ok(r.resumo);
  };
  worker.onerror = (e) => {
    for (const [, espera] of esperando) espera.falhou(new Error(e.message || 'o Worker de pintura falhou'));
    esperando.clear();
  };

  const enviar = (pedido: PedidoAoWorker, transferir: Transferable[] = []): Promise<Resumo | null> =>
    new Promise((ok, falhou) => {
      if (pedido.t !== 'tamanho') esperando.set(pedido.id, { ok, falhou });
      worker.postMessage(pedido, transferir);
      if (pedido.t === 'tamanho') ok(null);
    });

  return {
    emWorker: true,
    tamanho: (largura, altura, dpr) => void enviar({ t: 'tamanho', largura, altura, dpr }),
    async pintar(p) {
      // Os bytes seguem para o Worker sem cópia (a memória muda de dono).
      const bytes = p.bytes.byteOffset === 0 && p.bytes.byteLength === p.bytes.buffer.byteLength ? p.bytes : p.bytes.slice();
      const r = await enviar({ t: 'pintar', id: proximo++, bytes, vista: p.vista, modo: p.modo, cores: p.cores, comTextos: p.comTextos }, [bytes.buffer]);
      if (!r) throw new Error('o Worker não devolveu o resumo do quadro');
      return r;
    },
    async pintarTextos(vista) {
      await enviar({ t: 'textos', id: proximo++, vista });
    },
  };
}
