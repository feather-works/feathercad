/**
 * O núcleo da pintura do quadro: decodifica os bytes do motor e pinta a grade, os traços e os textos em duas telas
 * (a do desenho e a dos textos). É o mesmo código na thread principal (telas de verdade, sem suporte a Worker) e num
 * Worker (`OffscreenCanvas`, que devolve `ImageBitmap` para a thread principal instalar de uma vez). **Só desenha o
 * que o motor mandou.**
 */

import { lerQuadro } from '../ponte/quadro-binario.ts';
import type { Fundo, Quadro, Vista } from '../ponte/tipos.ts';
import { pintarGrade, pintarImagens, pintarTextos, pintarTracos, type CoresDaGrade, type ImagemCarregada, type ImagemFaltando } from './pintura.ts';

export type Tela = OffscreenCanvas | HTMLCanvasElement;
type Ctx = OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D;
type QuadroNovo = Extract<Quadro, { tipo: 'novo' }>;

/** O que a pintura de um quadro devolve: o que havia nele e quanto cada parte custou (ms). */
export interface Resumo {
  tipo: 'igual' | 'novo';
  versao: number;
  segmentos: number;
  textos: number;
  ms: { decodificar: number; fundo: number; textos: number };
  /** Imagens do quadro que faltam carregar (ou pedem mais resolução); quem pinta as busca e chama `instalarImagem`. */
  imagensFaltando: ImagemFaltando[];
}

export interface Pintado {
  resumo: Resumo;
  /** Só num Worker: a imagem pronta de cada camada (`null` = a camada não foi refeita). */
  bitmapFundo: ImageBitmap | null;
  bitmapTextos: ImageBitmap | null;
}

function contexto(tela: Tela): Ctx {
  const ctx = tela.getContext('2d');
  if (!ctx) throw new Error('canvas 2D indisponível');
  return ctx as Ctx;
}

const RESUMO_IGUAL: Resumo = { tipo: 'igual', versao: 0, segmentos: 0, textos: 0, ms: { decodificar: 0, fundo: 0, textos: 0 }, imagensFaltando: [] };

export class Nucleo {
  private q: QuadroNovo | null = null;
  /** Como o último quadro foi pintado (para repintar o fundo quando chega uma imagem). */
  private ultimo: { vista: Vista; modo: Fundo; cores: CoresDaGrade } | null = null;
  private dpr = 1;
  private readonly imagens = new Map<string, ImagemCarregada>();
  private readonly ctxFundo: Ctx;
  private readonly ctxTextos: Ctx;

  constructor(
    private readonly fundo: Tela,
    private readonly textos: Tela,
    /** Num Worker as telas são `OffscreenCanvas` e a imagem pronta sai como `ImageBitmap`. */
    private readonly comBitmaps: boolean,
  ) {
    this.ctxFundo = contexto(fundo);
    this.ctxTextos = contexto(textos);
  }

  /** As duas telas passam a ter `largura` × `altura` (pixels CSS) na densidade `dpr`. */
  tamanho(largura: number, altura: number, dpr: number): void {
    this.dpr = dpr;
    for (const [tela, ctx] of [[this.fundo, this.ctxFundo], [this.textos, this.ctxTextos]] as const) {
      tela.width = Math.max(1, Math.round(largura * dpr));
      tela.height = Math.max(1, Math.round(altura * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
  }

  /** Decodifica e pinta o quadro `bytes`, feito para a `vista`. Com `comTextos` falso a camada de textos fica como está. */
  pintar(bytes: Uint8Array, vista: Vista, modo: Fundo, cores: CoresDaGrade, comTextos: boolean): Pintado {
    const t0 = performance.now();
    const q = lerQuadro(bytes);
    const t1 = performance.now();
    if (q.tipo === 'igual') return { resumo: RESUMO_IGUAL, bitmapFundo: null, bitmapTextos: null };
    this.q = q;
    this.ultimo = { vista, modo, cores };
    const imagensFaltando = this.pintarFundo();
    const t2 = performance.now();
    if (comTextos) this.desenharTextos(vista);
    const t3 = performance.now();
    return {
      resumo: { tipo: 'novo', versao: q.versao, segmentos: q.cores.length, textos: q.textos.length, ms: { decodificar: t1 - t0, fundo: t2 - t1, textos: t3 - t2 }, imagensFaltando },
      bitmapFundo: this.comBitmaps ? (this.fundo as OffscreenCanvas).transferToImageBitmap() : null,
      bitmapTextos: this.comBitmaps && comTextos ? (this.textos as OffscreenCanvas).transferToImageBitmap() : null,
    };
  }

  /** Grade, imagens (por baixo) e traços do último quadro. Devolve as imagens que faltam. */
  private pintarFundo(): ImagemFaltando[] {
    const { q, ultimo } = this;
    if (!q || !ultimo) return [];
    const { vista, modo, cores } = ultimo;
    this.ctxFundo.clearRect(0, 0, vista.largura, vista.altura);
    pintarGrade(this.ctxFundo, q.fundoX, q.fundoY, vista.largura, vista.altura, modo, cores, vista);
    const faltando = pintarImagens(this.ctxFundo, q.imagens, this.imagens, this.dpr);
    pintarTracos(this.ctxFundo, q);
    return faltando;
  }

  /** Guarda a imagem carregada e repinta o fundo com ela. Num Worker devolve a camada pronta para instalar. */
  instalarImagem(caminho: string, imagem: ImagemCarregada): ImageBitmap | null {
    this.imagens.get(caminho)?.bitmap.close();
    this.imagens.set(caminho, imagem);
    if (!this.q?.imagens.some((im) => im.caminho === caminho)) return null;
    this.pintarFundo();
    return this.comBitmaps ? (this.fundo as OffscreenCanvas).transferToImageBitmap() : null;
  }

  /** Só a camada de textos, com o último quadro decodificado (o texto nítido que volta quando o movimento para). */
  pintarSoTextos(vista: Vista): ImageBitmap | null {
    if (!this.q) return null;
    this.desenharTextos(vista);
    return this.comBitmaps ? (this.textos as OffscreenCanvas).transferToImageBitmap() : null;
  }

  private desenharTextos(vista: Vista): void {
    this.ctxTextos.clearRect(0, 0, vista.largura, vista.altura);
    if (this.q) pintarTextos(this.ctxTextos, this.q.textos);
  }
}
