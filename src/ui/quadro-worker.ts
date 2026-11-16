/**
 * O Worker que decodifica e pinta o quadro do desenho, fora da thread principal (a pintura de centenas de milhares de
 * traços custava 50–70 ms por quadro e travava o mouse). Recebe os bytes do motor (transferidos, sem cópia), pinta em
 * duas `OffscreenCanvas` e devolve as imagens prontas como `ImageBitmap`, que a thread principal instala **de uma vez
 * junto com a nova matriz CSS** (troca atômica, sem piscar). Só desenha: nada de regra do desenho aqui.
 */

import type { Fundo, Vista } from '../ponte/tipos.ts';
import { Nucleo, type Resumo } from './pintor-nucleo.ts';
import type { CoresDaGrade, ImagemCarregada } from './pintura.ts';

export type PedidoAoWorker =
  | { t: 'tamanho'; largura: number; altura: number; dpr: number }
  | { t: 'pintar'; id: number; bytes: Uint8Array; vista: Vista; modo: Fundo; cores: CoresDaGrade; comTextos: boolean }
  | { t: 'textos'; id: number; vista: Vista }
  | { t: 'imagem'; caminho: string; imagem: ImagemCarregada };

export type RespostaDoWorker =
  | { t: 'pronto'; id: number; resumo: Resumo | null; bitmapFundo: ImageBitmap | null; bitmapTextos: ImageBitmap | null }
  | { t: 'erro'; id: number; mensagem: string };

interface Escopo {
  onmessage: ((e: MessageEvent<PedidoAoWorker>) => void) | null;
  postMessage(mensagem: RespostaDoWorker, transferir: Transferable[]): void;
}

const escopo = self as unknown as Escopo;
const nucleo = new Nucleo(new OffscreenCanvas(1, 1), new OffscreenCanvas(1, 1), true);

escopo.onmessage = (e) => {
  const m = e.data;
  if (m.t === 'tamanho') {
    nucleo.tamanho(m.largura, m.altura, m.dpr);
    return;
  }
  if (m.t === 'imagem') {
    // Sem pedido esperando (id 0): a thread principal só instala a camada repintada.
    const b = nucleo.instalarImagem(m.caminho, m.imagem);
    if (b) escopo.postMessage({ t: 'pronto', id: 0, resumo: null, bitmapFundo: b, bitmapTextos: null }, [b]);
    return;
  }
  try {
    if (m.t === 'pintar') {
      const r = nucleo.pintar(m.bytes, m.vista, m.modo, m.cores, m.comTextos);
      escopo.postMessage({ t: 'pronto', id: m.id, resumo: r.resumo, bitmapFundo: r.bitmapFundo, bitmapTextos: r.bitmapTextos }, [r.bitmapFundo, r.bitmapTextos].filter((b): b is ImageBitmap => b !== null));
    } else {
      const b = nucleo.pintarSoTextos(m.vista);
      escopo.postMessage({ t: 'pronto', id: m.id, resumo: null, bitmapFundo: null, bitmapTextos: b }, b ? [b] : []);
    }
  } catch (erro) {
    escopo.postMessage({ t: 'erro', id: m.id, mensagem: erro instanceof Error ? erro.message : String(erro) }, []);
  }
};
