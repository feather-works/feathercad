/**
 * Pintar um quadro do motor num canvas 2D: preenchimentos, segmentos e textos. **Só desenha o que o motor
 * mandou** (coordenadas já em pixels, cores já resolvidas). Fica fora do `area-desenho.ts` para poder ser
 * medido sozinho (`bench/`) e otimizado sem mexer no resto da área.
 */

import { etapa } from '../ponte/perfil.ts';
import type { Fundo, MarcasDoFundo, Preenchimento, TextoDoQuadro } from '../ponte/tipos.ts';

/** O que se pinta de um quadro: preenchimentos, segmentos e textos (sem o fundo). */
export interface ObjetosDoQuadro {
  xs: Float32Array;
  cores: Uint32Array;
  espessuras: Float32Array;
  textos: TextoDoQuadro[];
  preenchimentos: Preenchimento[];
}

export function corCss(cor: number): string {
  return `#${cor.toString(16).padStart(6, '0')}`;
}

/** Preenchimentos, segmentos (um caminho por cor e espessura) e textos de um quadro. */
export function pintarObjetos(ctx: CanvasRenderingContext2D, q: ObjetosDoQuadro): void {
  pintarTracos(ctx, q);
  etapa('pintar/textos', () => pintarTextos(ctx, q.textos));
}

/** Só os preenchimentos e os segmentos (o texto vai para a camada própria da área de desenho). */
export function pintarTracos(ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, q: ObjetosDoQuadro): void {
  // As etapas `pintar/…` aparecem no perfil (`ponte/perfil.ts`) como detalhe de `pintar_fundo`.
  etapa('pintar/preenchimentos', () => {
    for (const p of q.preenchimentos) {
      ctx.fillStyle = corCss(p.cor);
      ctx.beginPath();
      for (const laco of p.lacos) {
        ctx.moveTo(laco[0]!, laco[1]!);
        for (let k = 2; k < laco.length; k += 2) ctx.lineTo(laco[k]!, laco[k + 1]!);
        ctx.closePath();
      }
      ctx.fill('evenodd');
    }
  });
  ctx.lineWidth = 1;
  const { xs, cores, espessuras } = q;
  // Um caminho por cor e espessura (não um `stroke` por segmento): com dezenas de milhares de segmentos a diferença é enorme.
  const porCor = etapa('pintar/agrupar', () => {
    const grupos = new Map<number, number[]>();
    for (let i = 0; i < cores.length; i++) {
      // chave numérica (cor × espessura em décimos de pixel): sem criar um texto por segmento
      const chave = cores[i]! * 1024 + Math.min(1023, Math.round(espessuras[i]! * 10));
      const lista = grupos.get(chave);
      if (lista) lista.push(i);
      else grupos.set(chave, [i]);
    }
    return grupos;
  });
  etapa('pintar/tracos', () => {
    for (const indices of porCor.values()) {
      const primeiro = indices[0]!;
      ctx.strokeStyle = corCss(cores[primeiro]!);
      ctx.lineWidth = espessuras[primeiro]!;
      ctx.beginPath();
      for (const i of indices) {
        ctx.moveTo(xs[i * 4]!, xs[i * 4 + 1]!);
        ctx.lineTo(xs[i * 4 + 2]!, xs[i * 4 + 3]!);
      }
      ctx.stroke();
    }
  });
}

/**
 * Os textos, cada um na posição, tamanho e giro que o motor mandou. Sem `save`/`restore` (o `restore` devolveria a fonte,
 * o alinhamento e a linha de base ao padrão): só a transformação é refeita a cada texto, e a fonte, o alinhamento e a
 * linha de base só são reatribuídos quando mudam (o WebKit interpreta a string a cada atribuição).
 */
export function pintarTextos(ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, textos: TextoDoQuadro[]): void {
  const base = ctx.getTransform();
  let fonte = '';
  let alinhamento = '';
  let linhaDeBase = '';
  for (const t of textos) {
    ctx.setTransform(base);
    ctx.translate(t.x, t.y);
    ctx.rotate(t.giro);
    ctx.fillStyle = corCss(t.cor);
    const nova = `${t.altura}px system-ui, sans-serif`;
    if (nova !== fonte) ctx.font = fonte = nova;
    if (t.h !== alinhamento) ctx.textAlign = alinhamento = t.h;
    if (t.base !== linhaDeBase) ctx.textBaseline = linhaDeBase = t.base;
    ctx.fillText(t.conteudo, 0, 0);
  }
  ctx.setTransform(base);
}

/** As cores da grade (do tema), já resolvidas: o Worker não enxerga o CSS. */
export interface CoresDaGrade {
  linha: string;
  principal: string;
  ponto: string;
}

/** O fundo (linhas ou pontos) nas posições que o motor mandou, numa tela de `largura` × `altura` pixels. */
export function pintarGrade(ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, gx: MarcasDoFundo, gy: MarcasDoFundo, largura: number, altura: number, modo: Fundo, cores: CoresDaGrade): void {
  if (modo === 'grade') {
    ctx.lineWidth = 1;
    for (const principal of [false, true]) {
      ctx.strokeStyle = principal ? cores.principal : cores.linha;
      ctx.beginPath();
      gx.pos.forEach((x, i) => {
        if ((gx.principal[i] === 1) === principal) {
          ctx.moveTo(Math.round(x) + 0.5, 0);
          ctx.lineTo(Math.round(x) + 0.5, altura);
        }
      });
      gy.pos.forEach((y, i) => {
        if ((gy.principal[i] === 1) === principal) {
          ctx.moveTo(0, Math.round(y) + 0.5);
          ctx.lineTo(largura, Math.round(y) + 0.5);
        }
      });
      ctx.stroke();
    }
  } else if (modo === 'pontos') {
    ctx.fillStyle = cores.ponto;
    for (const x of gx.pos) for (const y of gy.pos) ctx.fillRect(Math.round(x), Math.round(y), 1.5, 1.5);
  }
}
