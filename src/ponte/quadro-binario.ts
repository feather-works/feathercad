/**
 * Ler os bytes do quadro (formato em `crates/feathercad-core/src/quadro.rs`). Só funções puras, sem nada da janela:
 * roda na thread principal, num Worker e nos testes.
 */

import type { ImagemDoQuadro, MarcasDoFundo, Preenchimento, Quadro, TextoDoQuadro } from './tipos.ts';

const BYTES_POR_SEGMENTO = 24;
const BYTES_POR_MARCA = 5;
const CABECALHO = 16;

function lerMarcas(v: DataView, inicio: number, n: number): MarcasDoFundo {
  const pos = new Float32Array(n);
  const principal = new Uint8Array(n);
  for (let i = 0; i < n; i++) {
    pos[i] = v.getFloat32(inicio + i * BYTES_POR_MARCA, true);
    principal[i] = v.getUint8(inicio + i * BYTES_POR_MARCA + 4);
  }
  return { pos, principal };
}

const decodificador = new TextDecoder();
const ALINHA_H = ['left', 'center', 'right'] as const;
const ALINHA_V = ['alphabetic', 'bottom', 'middle', 'top'] as const;

/** Lê a seção de textos: contagem e, por texto, posição, altura, giro, alinhamentos, cor e conteúdo. */
function lerTextos(v: DataView, bytes: Uint8Array, inicio: number): { textos: TextoDoQuadro[]; fim: number } {
  const n = v.getUint32(inicio, true);
  const textos: TextoDoQuadro[] = [];
  let o = inicio + 4;
  for (let i = 0; i < n; i++) {
    const x = v.getFloat32(o, true);
    const y = v.getFloat32(o + 4, true);
    const altura = v.getFloat32(o + 8, true);
    const giro = v.getFloat32(o + 12, true);
    const h = ALINHA_H[v.getUint8(o + 16)] ?? 'left';
    const base = ALINHA_V[v.getUint8(o + 17)] ?? 'alphabetic';
    const cor = v.getUint32(o + 18, true);
    const tamanho = v.getUint16(o + 22, true);
    const conteudo = decodificador.decode(bytes.subarray(v.byteOffset + o + 24, v.byteOffset + o + 24 + tamanho));
    textos.push({ x, y, altura, giro, h, base, cor, conteudo });
    o += 24 + tamanho;
  }
  return { textos, fim: o };
}

/** Lê a seção de preenchimentos: por área, cor e laços de pontos em pixels. */
function lerPreenchimentos(v: DataView, inicio: number): { preenchimentos: Preenchimento[]; fim: number } {
  const n = v.getUint32(inicio, true);
  const preenchimentos: Preenchimento[] = [];
  let o = inicio + 4;
  for (let i = 0; i < n; i++) {
    const cor = v.getUint32(o, true);
    const nl = v.getUint32(o + 4, true);
    o += 8;
    const lacos: Float32Array[] = [];
    for (let l = 0; l < nl; l++) {
      const nv = v.getUint32(o, true);
      o += 4;
      const pts = new Float32Array(nv * 2);
      for (let k = 0; k < nv * 2; k++) pts[k] = v.getFloat32(o + k * 4, true);
      o += nv * 8;
      lacos.push(pts);
    }
    preenchimentos.push({ cor, lacos });
  }
  return { preenchimentos, fim: o };
}

/** Lê a seção de imagens (a última do quadro; ausente num quadro mais antigo = nenhuma imagem). */
function lerImagens(v: DataView, bytes: Uint8Array, inicio: number): ImagemDoQuadro[] {
  if (inicio + 4 > v.byteLength) return [];
  const n = v.getUint32(inicio, true);
  const imagens: ImagemDoQuadro[] = [];
  let o = inicio + 4;
  for (let i = 0; i < n; i++) {
    const f = (k: number): number => v.getFloat32(o + k * 4, true);
    const tamanho = v.getUint16(o + 24, true);
    const caminho = decodificador.decode(bytes.subarray(o + 26, o + 26 + tamanho));
    imagens.push({ x: f(0), y: f(1), ux: f(2), uy: f(3), vx: f(4), vy: f(5), caminho });
    o += 26 + tamanho;
  }
  return imagens;
}

/** Lê os bytes do quadro (formato em `crates/feathercad-core/src/quadro.rs`). */
export function lerQuadro(bytes: Uint8Array): Quadro {
  const magia = String.fromCharCode(...bytes.subarray(0, 4));
  if (magia === 'FCQ0') return { tipo: 'igual' };
  if (magia !== 'FCQ1') throw new Error(`quadro desconhecido: ${magia}`);
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const versao = Number(v.getBigUint64(4, true));
  const n = v.getUint32(12, true);
  const xs = new Float32Array(n * 4);
  const cores = new Uint32Array(n);
  const espessuras = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const o = CABECALHO + i * BYTES_POR_SEGMENTO;
    for (let k = 0; k < 4; k++) xs[i * 4 + k] = v.getFloat32(o + k * 4, true);
    cores[i] = v.getUint32(o + 16, true);
    espessuras[i] = v.getFloat32(o + 20, true);
  }
  // Depois dos segmentos vêm os textos e, por fim, o fundo (duas contagens e as marcas de X e de Y).
  const { textos, fim: fimTextos } = lerTextos(v, bytes, CABECALHO + n * BYTES_POR_SEGMENTO);
  const { preenchimentos, fim } = lerPreenchimentos(v, fimTextos);
  const nx = v.getUint32(fim, true);
  const ny = v.getUint32(fim + 4, true);
  const fundoX = lerMarcas(v, fim + 8, nx);
  const fundoY = lerMarcas(v, fim + 8 + nx * BYTES_POR_MARCA, ny);
  const imagens = lerImagens(v, bytes, fim + 8 + (nx + ny) * BYTES_POR_MARCA);
  return { tipo: 'novo', versao, xs, cores, espessuras, textos, preenchimentos, fundoX, fundoY, imagens };
}
