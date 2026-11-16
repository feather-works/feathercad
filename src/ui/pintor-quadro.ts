/**
 * Quem pinta o quadro do desenho para a área de desenho: num **Worker** (`OffscreenCanvas`, a thread principal fica livre)
 * ou, se a webview não suporta, na própria thread principal. As duas formas têm a mesma interface, e as duas devolvem
 * o resultado **depois de a imagem estar instalada na tela** (para a área trocar a matriz CSS no mesmo instante).
 */

import { lerImagem, noApp } from '../ponte/nativo.ts';
import type { Fundo, Vista } from '../ponte/tipos.ts';
import { Nucleo, type Resumo } from './pintor-nucleo.ts';
import type { CoresDaGrade, ImagemCarregada, ImagemFaltando } from './pintura.ts';
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
  /** Uma imagem do desenho acabou de carregar: guarda e repinta o fundo com ela. */
  instalarImagem(caminho: string, imagem: ImagemCarregada): void;
}

/** Maior largura (pixels) em que uma imagem é decodificada, por mais perto que se chegue. */
const LARGURA_MAXIMA_DA_IMAGEM = 8192;

/**
 * Busca as imagens que o quadro pediu, uma de cada vez, no tamanho que a tela pede (em degraus de potência de 2) e as
 * entrega a `instalar`. A foto inteira de uma prancha passa de 40 MB decodificada; longe, basta uma fração disso.
 */
function carregadorDeImagens(instalar: (caminho: string, imagem: ImagemCarregada) => void): (faltando: ImagemFaltando[]) => void {
  const pedidas = new Map<string, number>();
  const falhas = new Set<string>();
  const fila: ImagemFaltando[] = [];
  let ocupado = false;
  const andar = async (): Promise<void> => {
    if (ocupado) return;
    ocupado = true;
    for (let f = fila.shift(); f; f = fila.shift()) {
      try {
        const inteira = await createImageBitmap(new Blob([(await lerImagem(f.caminho)).slice()]));
        const larguraOriginal = inteira.width;
        let bitmap = inteira;
        if (f.larguraDesejada < larguraOriginal) {
          const resizeHeight = Math.max(1, Math.round((inteira.height * f.larguraDesejada) / larguraOriginal));
          bitmap = await createImageBitmap(inteira, { resizeWidth: f.larguraDesejada, resizeHeight, resizeQuality: 'high' });
          inteira.close();
        }
        instalar(f.caminho, { bitmap, larguraOriginal });
      } catch (erro) {
        falhas.add(f.caminho);
        console.warn(`[FeatherCAD] imagem não carregou: ${f.caminho} — ${erro instanceof Error ? erro.message : String(erro)}`);
      }
    }
    ocupado = false;
  };
  return (faltando) => {
    for (const f of faltando) {
      const alvo = Math.min(LARGURA_MAXIMA_DA_IMAGEM, 2 ** Math.ceil(Math.log2(Math.max(64, f.larguraDesejada))));
      if (falhas.has(f.caminho) || (pedidas.get(f.caminho) ?? 0) >= alvo) continue;
      pedidas.set(f.caminho, alvo);
      fila.push({ caminho: f.caminho, larguraDesejada: alvo });
    }
    void andar();
  };
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
  const base = segundoPlano && suportaPintarEmSegundoPlano() ? pintorEmWorker(fundo, textos) : pintorNaThreadPrincipal(fundo, textos);
  // As imagens vêm do disco pela casca nativa; no navegador (sem ela) o desenho aparece sem as imagens.
  if (!noApp()) return base;
  const carregar = carregadorDeImagens((caminho, imagem) => base.instalarImagem(caminho, imagem));
  return {
    ...base,
    async pintar(p) {
      const r = await base.pintar(p);
      if (r.imagensFaltando.length > 0) carregar(r.imagensFaltando);
      return r;
    },
  };
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
    instalarImagem: (caminho, imagem) => void nucleo.instalarImagem(caminho, imagem),
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
      if (pedido.t !== 'tamanho' && pedido.t !== 'imagem') esperando.set(pedido.id, { ok, falhou });
      worker.postMessage(pedido, transferir);
      if (pedido.t === 'tamanho' || pedido.t === 'imagem') ok(null);
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
    instalarImagem: (caminho, imagem) => void enviar({ t: 'imagem', caminho, imagem }, [imagem.bitmap]),
  };
}
