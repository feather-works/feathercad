/**
 * Pintar um quadro do motor num canvas 2D: preenchimentos, segmentos e textos. **Só desenha o que o motor
 * mandou** (coordenadas já em pixels, cores já resolvidas). Fica fora do `area-desenho.ts` para poder ser
 * medido sozinho (`bench/`) e otimizado sem mexer no resto da área.
 */

import { etapa } from '../ponte/perfil.ts';
import type { Fundo, MarcasDoFundo, Preenchimento, TextoDoQuadro, Vista } from '../ponte/tipos.ts';

/** O que se pinta de um quadro: preenchimentos, segmentos e textos (sem o fundo). */
export interface ObjetosDoQuadro {
  xs: Float32Array;
  cores: Uint32Array;
  espessuras: Float32Array;
  textos: TextoDoQuadro[];
  preenchimentos: Preenchimento[];
}

const COR_CACHE = new Map<number, string>();

export function corCss(cor: number): string {
  let res = COR_CACHE.get(cor);
  if (res === undefined) {
    res = `#${cor.toString(16).padStart(6, '0')}`;
    COR_CACHE.set(cor, res);
  }
  return res;
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
  etapa('pintar/tracos', () => {
    let i = 0;
    const n = cores.length;
    while (i < n) {
      const corAtual = cores[i]!;
      const espessuraAtual = espessuras[i]!;
      ctx.strokeStyle = corCss(corAtual);
      ctx.lineWidth = espessuraAtual;
      ctx.beginPath();
      while (i < n && cores[i] === corAtual && espessuras[i] === espessuraAtual) {
        ctx.moveTo(xs[i * 4]!, xs[i * 4 + 1]!);
        ctx.lineTo(xs[i * 4 + 2]!, xs[i * 4 + 3]!);
        i++;
      }
      ctx.stroke();
    }
  });
}

/**
 * Os textos, cada um na posição, tamanho e giro que o motor mandou. Sem `save`/`restore` (o `restore` devolveria a fonte,
 * o alinhamento e a linha de base ao padrão): só a transformação é refeita a cada texto, e a fonte, o alinhamento e a
 * linha de base só são reatribuídos quando mudam (o WebKit interpreta a string a cada atribuição).
 * Para evitar bitmap pixelado no WebKit em textos pequenos, os tamanhos menores que 14px são vetorizados em tamanho base
 * maior (28px) e escalados no canvas transform.
 */
export function pintarTextos(ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, textos: TextoDoQuadro[]): void {
  const base = ctx.getTransform();
  let fonte = '';
  let alinhamento = '';
  let linhaDeBase = '';
  const fontStyle = 'system-ui, sans-serif';

  for (const t of textos) {
    ctx.setTransform(base);
    ctx.translate(t.x, t.y);
    ctx.rotate(t.giro);

    const altura = t.altura;
    const precisaEscala = altura > 0 && altura < 14;
    const tamanhoBase = precisaEscala ? 28 : Math.max(1, altura);
    const esc = precisaEscala ? altura / 28 : 1;

    if (precisaEscala) {
      ctx.scale(esc, esc);
    }

    ctx.fillStyle = corCss(t.cor);
    const nova = `${tamanhoBase}px ${fontStyle}`;
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
export function pintarGrade(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  gx: MarcasDoFundo,
  gy: MarcasDoFundo,
  largura: number,
  altura: number,
  modo: Fundo,
  cores: CoresDaGrade,
  vista?: Vista
): void {
  if (modo === 'nada') return;

  const rx = vista?.rotacao_x ?? 0;
  const rz = vista?.rotacao_z ?? 0;
  const em3D = Math.abs(rx) > 1e-4 || Math.abs(rz) > 1e-4;

  if (!em3D) {
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
    return;
  }

  // 3D Grid on Ground Plane Z=0
  if (!vista) return;
  const es = vista.escala;

  const radX = (rx * Math.PI) / 180;
  const radZ = (rz * Math.PI) / 180;
  const cosZ = Math.cos(radZ), sinZ = Math.sin(radZ);
  const cosX = Math.cos(radX), sinX = Math.sin(radX);

  const cxProj = vista.centro.x * cosZ + vista.centro.y * sinZ;
  const cy1 = -vista.centro.x * sinZ + vista.centro.y * cosZ;
  const cyProj = cy1 * cosX + (vista.centro.z ?? 0) * sinX;

  function paraTela3D(wx: number, wy: number, wz = 0): [number, number] {
    const x1 = wx * cosZ + wy * sinZ;
    const y1 = -wx * sinZ + wy * cosZ;
    const yv = y1 * cosX + wz * sinX;
    const sx = (x1 - cxProj) * es + largura / 2.0;
    const sy = altura / 2.0 - (yv - cyProj) * es;
    return [sx, sy];
  }

  // Se a visão for rasante/perpendicular ao plano horizontal Z=0 (ex: vistas frontal e lateral), desenha o plano de terra
  if (Math.abs(cosX) < 0.08) {
    const [, yHorizonte] = paraTela3D(vista.centro.x, vista.centro.y, 0);
    ctx.lineWidth = 1;
    ctx.strokeStyle = cores.principal;
    ctx.beginPath();
    ctx.moveTo(0, yHorizonte);
    ctx.lineTo(largura, yHorizonte);
    ctx.stroke();

    const range = largura / (es || 1);
    const step = Math.max(10, Math.pow(10, Math.floor(Math.log10(range / 20 || 1))));
    const count = Math.ceil(largura / (step * es || 1)) + 2;
    ctx.strokeStyle = cores.linha;
    ctx.beginPath();
    for (let k = -count; k <= count; k++) {
      const sx = largura / 2.0 + k * step * es;
      ctx.moveTo(sx, 0);
      ctx.lineTo(sx, altura);
    }
    ctx.stroke();
    return;
  }

  function paraMundo3D(sx: number, sy: number, wz = 0): [number, number] {
    const xv = cxProj + (sx - largura / 2.0) / (es || 1);
    const yv = cyProj + (altura / 2.0 - sy) / (es || 1);
    const yv_sem_z = yv - wz * sinX;
    const y1 = Math.abs(cosX) > 1e-4 ? yv_sem_z / cosX : yv_sem_z;
    const px = xv * cosZ - y1 * sinZ;
    const py = xv * sinZ + y1 * cosZ;
    return [px, py];
  }

  const c1 = paraMundo3D(0, 0, 0);
  const c2 = paraMundo3D(largura, 0, 0);
  const c3 = paraMundo3D(largura, altura, 0);
  const c4 = paraMundo3D(0, altura, 0);

  const minX = Math.min(c1[0], c2[0], c3[0], c4[0]);
  const maxX = Math.max(c1[0], c2[0], c3[0], c4[0]);
  const minY = Math.min(c1[1], c2[1], c3[1], c4[1]);
  const maxY = Math.max(c1[1], c2[1], c3[1], c4[1]);

  const range = Math.max(maxX - minX, maxY - minY, 10);
  const step = Math.max(10, Math.pow(10, Math.floor(Math.log10(range / 20 || 1))));

  const startI = Math.floor(minX / step) - 2;
  const endI = Math.ceil(maxX / step) + 2;
  const startJ = Math.floor(minY / step) - 2;
  const endJ = Math.ceil(maxY / step) + 2;

  const maxLines = 150;
  const iCount = Math.min(endI - startI, maxLines);
  const jCount = Math.min(endJ - startJ, maxLines);

  ctx.lineWidth = 1;
  for (const principal of [false, true]) {
    ctx.strokeStyle = principal ? cores.principal : cores.linha;
    ctx.beginPath();

    for (let i = startI; i <= startI + iCount; i++) {
      const isPrinc = Math.abs(i) % 5 === 0;
      if (isPrinc !== principal) continue;
      const wx = i * step;
      const [x1, y1] = paraTela3D(wx, startJ * step, 0);
      const [x2, y2] = paraTela3D(wx, (startJ + jCount) * step, 0);
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
    }

    for (let j = startJ; j <= startJ + jCount; j++) {
      const isPrinc = Math.abs(j) % 5 === 0;
      if (isPrinc !== principal) continue;
      const wy = j * step;
      const [x1, y1] = paraTela3D(startI * step, wy, 0);
      const [x2, y2] = paraTela3D((startI + iCount) * step, wy, 0);
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
    }

    ctx.stroke();
  }
}

/** Desenha os eixos de coordenadas X (vermelho), Y (verde) e Z (azul) no canto inferior esquerdo e na origem (0,0,0). */
export function pintarEixoUCS(
  ctx: CanvasRenderingContext2D,
  vista: Vista,
  largura: number,
  altura: number
): void {
  const rx = ((vista.rotacao_x ?? 0) * Math.PI) / 180;
  const rz = ((vista.rotacao_z ?? 0) * Math.PI) / 180;

  function projetarVetor(dx: number, dy: number, dz: number): [number, number] {
    const cosZ = Math.cos(rz), sinZ = Math.sin(rz);
    const cosX = Math.cos(rx), sinX = Math.sin(rx);
    const x1 = dx * cosZ + dy * sinZ;
    const y1 = -dx * sinZ + dy * cosZ;
    const yv = y1 * cosX + dz * sinX;
    return [x1, -yv];
  }

  const pad = 48;
  const x0 = pad;
  const y0 = altura - pad;
  const L = 30;

  const [vxX, vyX] = projetarVetor(1, 0, 0);
  const lenX = Math.hypot(vxX, vyX) || 1;
  const exX = x0 + (vxX / lenX) * L;
  const eyX = y0 + (vyX / lenX) * L;

  const [vxY, vyY] = projetarVetor(0, 1, 0);
  const lenY = Math.hypot(vxY, vyY) || 1;
  const exY = x0 + (vxY / lenY) * L;
  const eyY = y0 + (vyY / lenY) * L;

  const [vxZ, vyZ] = projetarVetor(0, 0, 1);
  const lenZ = Math.hypot(vxZ, vyZ) || 1;
  const exZ = x0 + (vxZ / lenZ) * L;
  const eyZ = y0 + (vyZ / lenZ) * L;

  ctx.save();
  ctx.lineWidth = 2;
  ctx.font = 'bold 11px sans-serif';

  // X Axis (Red)
  ctx.strokeStyle = '#FF4444';
  ctx.fillStyle = '#FF4444';
  ctx.beginPath();
  ctx.moveTo(x0, y0);
  ctx.lineTo(exX, eyX);
  ctx.stroke();
  ctx.fillText('X', exX + (vxX / lenX) * 10 - 4, eyX + (vyX / lenX) * 10 + 4);

  // Y Axis (Green)
  ctx.strokeStyle = '#44FF44';
  ctx.fillStyle = '#44FF44';
  ctx.beginPath();
  ctx.moveTo(x0, y0);
  ctx.lineTo(exY, eyY);
  ctx.stroke();
  ctx.fillText('Y', exY + (vxY / lenY) * 10 - 4, eyY + (vyY / lenY) * 10 + 4);

  // Z Axis (Blue)
  if (Math.abs(rx) > 0.05 || Math.abs(rz) > 0.05 || Math.hypot(vxZ, vyZ) > 0.2) {
    ctx.strokeStyle = '#4488FF';
    ctx.fillStyle = '#4488FF';
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(exZ, eyZ);
    ctx.stroke();
    ctx.fillText('Z', exZ + (vxZ / lenZ) * 10 - 4, eyZ + (vyZ / lenZ) * 10 + 4);
  }

  // Origin square box
  ctx.fillStyle = '#FFFFFF';
  ctx.fillRect(x0 - 2, y0 - 2, 5, 5);

  // 2. World Origin (0,0,0) Triad on Drawing if Visible
  const es = vista.escala;
  const cosZ = Math.cos(rz), sinZ = Math.sin(rz);
  const cosX = Math.cos(rx), sinX = Math.sin(rx);
  const cxProj = vista.centro.x * cosZ + vista.centro.y * sinZ;
  const cy1 = -vista.centro.x * sinZ + vista.centro.y * cosZ;
  const cyProj = cy1 * cosX + (vista.centro.z ?? 0) * sinX;

  const ox1 = 0 * cosZ + 0 * sinZ;
  const oy1 = -0 * sinZ + 0 * cosZ;
  const oyv = oy1 * cosX + 0 * sinX;
  const scrX = (ox1 - cxProj) * es + largura / 2.0;
  const scrY = altura / 2.0 - (oyv - cyProj) * es;

  if (scrX >= -50 && scrX <= largura + 50 && scrY >= -50 && scrY <= altura + 50) {
    const L_orig = Math.min(60, Math.max(20, 32 * (es / 50)));
    const oexX = scrX + (vxX / lenX) * L_orig;
    const oeyX = scrY + (vyX / lenX) * L_orig;
    const oexY = scrX + (vxY / lenY) * L_orig;
    const oeyY = scrY + (vyY / lenY) * L_orig;
    const oexZ = scrX + (vxZ / lenZ) * L_orig;
    const oeyZ = scrY + (vyZ / lenZ) * L_orig;

    ctx.lineWidth = 1.5;

    // X Axis
    ctx.strokeStyle = 'rgba(255, 68, 68, 0.9)';
    ctx.fillStyle = '#FF4444';
    ctx.beginPath();
    ctx.moveTo(scrX, scrY);
    ctx.lineTo(oexX, oeyX);
    ctx.stroke();
    ctx.fillText('X', oexX + 4, oeyX + 4);

    // Y Axis
    ctx.strokeStyle = 'rgba(68, 255, 68, 0.9)';
    ctx.fillStyle = '#44FF44';
    ctx.beginPath();
    ctx.moveTo(scrX, scrY);
    ctx.lineTo(oexY, oeyY);
    ctx.stroke();
    ctx.fillText('Y', oexY + 4, oeyY + 4);

    // Z Axis
    if (Math.abs(rx) > 0.05 || Math.abs(rz) > 0.05) {
      ctx.strokeStyle = 'rgba(68, 136, 255, 0.9)';
      ctx.fillStyle = '#4488FF';
      ctx.beginPath();
      ctx.moveTo(scrX, scrY);
      ctx.lineTo(oexZ, oeyZ);
      ctx.stroke();
      ctx.fillText('Z', oexZ + 4, oeyZ + 4);
    }

    // Origin square at (0,0,0)
    ctx.strokeStyle = '#FFFFFF';
    ctx.strokeRect(scrX - 3, scrY - 3, 6, 6);
  }

  ctx.restore();
}
