/**
 * A área de desenho: duas telas (o quadro do desenho e, por cima, cursor +
 * guias + prévia) e as caixinhas da entrada dinâmica. **Só desenha o que o
 * motor manda** — o ponto do mouse vai cru ao `cursor_mover` e volta resolvido
 * (snap, orto, grade), já com o texto das coordenadas.
 *
 * Zoom e pan também são contas do motor (`vista_zoom`, `vista_deslocar`): o
 * gesto vai e a vista nova volta. O TypeScript guarda a vista, não mexe nela.
 */

import { medir } from '../ponte/metricas.ts';
import { criarBarraNavegacao } from './barra-navegacao.ts';
import { criarVistaCubo } from './vista-cubo.ts';
import { aoMudarTema, corDoTema, temaVisivel } from './tema.ts';
import type { Estado, Fundo, ItemDeMenu, MarcasDoFundo, Motor, Preenchimento, Quadro, RespostaCursor, Segmento, TextoDoQuadro, TipoDeSnap, Vista } from '../ponte/index.ts';
import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const AREA_DESENHO_CSS = `
.area { position: relative; overflow: hidden; background: var(--fundo-area); cursor: none; }
.area canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
.area .desenho { z-index: ${Z.desenho}; }
.area .sobreposicao { z-index: ${Z.sobreposicao}; }
.area .caixa { position: absolute; z-index: ${Z.entradaDinamica}; padding: 1px 5px; background: var(--caixa-dinamica); border: 1px solid var(--destaque); color: var(--texto); font: 11px var(--fonte); pointer-events: none; white-space: nowrap; }
`;

const TAMANHO_DA_MIRA = 10;
const FATOR_DO_ZOOM = 1.25;
/** Dois cliques da roda em menos que isto enquadram o desenho (como no AutoCAD). */
const DUPLO_CLIQUE_MS = 350;
/** Quanto o mouse precisa andar, em pixels, para o "grab" valer como arrasto e não como clique. */
const LIMIAR_DO_ARRASTO_PX = 4;
/** As cores do desenho vêm das variáveis do tema (`base.ts`), relidas quando o tema muda. */
/** A marca de cada tipo de snap, como no AutoCAD: quadrado, triângulo, círculo, losango, X… */
function marcaDeSnap(ctx: CanvasRenderingContext2D, tipo: TipoDeSnap | null, x: number, y: number): void {
  const r = 6;
  ctx.beginPath();
  switch (tipo) {
    case 'meio':
      ctx.moveTo(x - r, y + r - 1);
      ctx.lineTo(x + r, y + r - 1);
      ctx.lineTo(x, y - r);
      ctx.closePath();
      break;
    case 'centro':
      ctx.arc(x, y, r, 0, Math.PI * 2);
      break;
    case 'quadrante':
      ctx.moveTo(x, y - r - 1);
      ctx.lineTo(x + r + 1, y);
      ctx.lineTo(x, y + r + 1);
      ctx.lineTo(x - r - 1, y);
      ctx.closePath();
      break;
    case 'intersecao':
      ctx.moveTo(x - r, y - r);
      ctx.lineTo(x + r, y + r);
      ctx.moveTo(x + r, y - r);
      ctx.lineTo(x - r, y + r);
      break;
    case 'perpendicular':
      ctx.moveTo(x - r, y - r);
      ctx.lineTo(x - r, y + r);
      ctx.lineTo(x + r, y + r);
      ctx.moveTo(x - r, y);
      ctx.lineTo(x, y);
      ctx.lineTo(x, y + r);
      break;
    case 'tangente':
      ctx.arc(x, y, r - 1, 0, Math.PI * 2);
      ctx.moveTo(x - r - 2, y - r - 1);
      ctx.lineTo(x + r + 2, y - r - 1);
      break;
    case 'no':
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.moveTo(x - r, y - r);
      ctx.lineTo(x + r, y + r);
      ctx.moveTo(x + r, y - r);
      ctx.lineTo(x - r, y + r);
      break;
    case 'proximo':
      ctx.moveTo(x - r, y - r);
      ctx.lineTo(x + r, y - r);
      ctx.lineTo(x - r, y + r);
      ctx.lineTo(x + r, y + r);
      ctx.closePath();
      break;
    default:
      ctx.rect(x - 5, y - 5, 10, 10);
  }
  ctx.stroke();
}

const cores = { linha: '', principal: '', ponto: '', cursor: '', previa: '', guia: '', osnap: '', janelaDentro: '', janelaToca: '' };

function lerCores(): void {
  cores.linha = corDoTema('--grade-linha');
  cores.principal = corDoTema('--grade-principal');
  cores.ponto = corDoTema('--grade-ponto');
  cores.cursor = corDoTema('--cursor');
  cores.previa = corDoTema('--previa');
  cores.guia = corDoTema('--guia');
  cores.osnap = corDoTema('--osnap');
  cores.janelaDentro = corDoTema('--janela-dentro');
  cores.janelaToca = corDoTema('--janela-toca');
}

/** O que se pinta de um quadro: preenchimentos, segmentos e textos (sem o fundo). */
interface ObjetosDoQuadro {
  xs: Float32Array;
  cores: Uint32Array;
  espessuras: Float32Array;
  textos: TextoDoQuadro[];
  preenchimentos: Preenchimento[];
}

export interface AreaDesenho {
  el: HTMLElement;
  /** O estado do motor mudou: pede o quadro de novo se preciso. */
  atualizar(e: Estado): void;
  /** A última resposta do cursor (para o rodapé mostrar as coordenadas). */
  ultimoCursor(): RespostaCursor | null;
  /** Aproxima/afasta em volta do centro, ou enquadra o desenho inteiro. */
  zoomMais(): void;
  zoomMenos(): void;
  ajustar(): void;
}

interface Contexto {
  /** O botão direito: onde (na janela) e o menu que o motor montou. */
  aoMenu: (x: number, y: number, itens: ItemDeMenu[]) => void;
  motor: Motor;
  t: Textos;
  /** Um gesto mudou o estado do motor (clique). */
  aoMudar: (e: Estado) => void;
  /** O cursor andou. */
  aoCursor: (r: RespostaCursor | null) => void;
  /** O zoom mudou: o texto pronto (`100%`) que o motor devolveu. */
  aoZoom: (texto: string) => void;
}

function tela2d(c: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = c.getContext('2d');
  if (!ctx) throw new Error('canvas 2D indisponível');
  return ctx;
}

function corCss(cor: number): string {
  return `#${cor.toString(16).padStart(6, '0')}`;
}

/** Cria a área de desenho. */
export function criarAreaDesenho(cx: Contexto): AreaDesenho {
  const { motor, t } = cx;
  const el = document.createElement('main');
  el.className = 'area';
  const telaDoFundo = document.createElement('canvas');
  telaDoFundo.className = 'desenho';
  const topo = document.createElement('canvas');
  topo.className = 'sobreposicao';
  const caixaXY = document.createElement('div');
  const caixaDA = document.createElement('div');
  caixaXY.className = caixaDA.className = 'caixa';
  caixaXY.hidden = caixaDA.hidden = true;
  el.append(telaDoFundo, topo, caixaXY, caixaDA);
  const zoomMais = (): void => {
    vista = motor.vistaZoomCentro(vista, FATOR_DO_ZOOM);
    vistaMudou();
  };
  const zoomMenos = (): void => {
    vista = motor.vistaZoomCentro(vista, 1 / FATOR_DO_ZOOM);
    vistaMudou();
  };
  const ajustar = (): void => {
    vista = motor.vistaAjustar(vista);
    vistaMudou();
  };
  el.append(criarVistaCubo(t), criarBarraNavegacao(t, zoomMais, zoomMenos, ajustar));

  let vista: Vista = motor.vistaInicial(800, 600);
  let quadro: { versao: number; xs: Float32Array; cores: Uint32Array; espessuras: Float32Array; textos: TextoDoQuadro[]; preenchimentos: Preenchimento[]; fundoX: MarcasDoFundo; fundoY: MarcasDoFundo } | null = null;
  let fundo: Fundo = 'nada';
  let vistaDoQuadro = '';
  let cursor: RespostaCursor | null = null;
  let mouse: { x: number; y: number } | null = null;
  /** Durante o "grab": os objetos arrastados, desenhados onde o cursor os põe (vêm do motor a cada movimento). */
  let arrastados: ObjetosDoQuadro | null = null;

  /** Refaz os dois canvases no tamanho da área (e da tela de alta densidade). */
  function ajustarTamanho(): void {
    const dpr = window.devicePixelRatio || 1;
    const { width, height } = el.getBoundingClientRect();
    if (width === 0 || height === 0) return;
    for (const c of [telaDoFundo, topo]) {
      c.width = Math.round(width * dpr);
      c.height = Math.round(height * dpr);
      tela2d(c).setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    vista = { ...vista, largura: width, altura: height };
    pedirQuadro();
    pintarTopo();
    cx.aoZoom(motor.vistaTexto(vista));
  }

  function pintarFundo(): void {
    const ctx = tela2d(telaDoFundo);
    ctx.clearRect(0, 0, vista.largura, vista.altura);
    if (!quadro) return;
    pintarGrade(ctx, quadro.fundoX, quadro.fundoY);
    pintarObjetos(ctx, quadro);
  }

  /** Preenchimentos, segmentos (um caminho por cor e espessura) e textos de um quadro. */
  function pintarObjetos(ctx: CanvasRenderingContext2D, q: ObjetosDoQuadro): void {
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
    ctx.lineWidth = 1;
    const { xs, cores, espessuras } = q;
    // Um caminho por cor e espessura (não um `stroke` por segmento): com dezenas de milhares de segmentos a diferença é enorme.
    const porCor = new Map<string, number[]>();
    for (let i = 0; i < cores.length; i++) {
      const chave = `${cores[i]!}:${espessuras[i]!}`;
      const lista = porCor.get(chave);
      if (lista) lista.push(i);
      else porCor.set(chave, [i]);
    }
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
    pintarTextos(ctx, q.textos);
  }

  /** Os textos, cada um na posição, tamanho e giro que o motor mandou. */
  function pintarTextos(ctx: CanvasRenderingContext2D, textos: TextoDoQuadro[]): void {
    for (const t of textos) {
      ctx.save();
      ctx.translate(t.x, t.y);
      ctx.rotate(t.giro);
      ctx.fillStyle = corCss(t.cor);
      ctx.font = `${t.altura}px system-ui, sans-serif`;
      ctx.textAlign = t.h;
      ctx.textBaseline = t.base;
      ctx.fillText(t.conteudo, 0, 0);
      ctx.restore();
    }
  }

  /** O fundo (linhas ou pontos) nas posições que o motor mandou. */
  function pintarGrade(ctx: CanvasRenderingContext2D, gx: MarcasDoFundo, gy: MarcasDoFundo): void {
    if (fundo === 'grade') {
      ctx.lineWidth = 1;
      for (const principal of [false, true]) {
        ctx.strokeStyle = principal ? cores.principal : cores.linha;
        ctx.beginPath();
        gx.pos.forEach((x, i) => {
          if ((gx.principal[i] === 1) === principal) {
            ctx.moveTo(Math.round(x) + 0.5, 0);
            ctx.lineTo(Math.round(x) + 0.5, vista.altura);
          }
        });
        gy.pos.forEach((y, i) => {
          if ((gy.principal[i] === 1) === principal) {
            ctx.moveTo(0, Math.round(y) + 0.5);
            ctx.lineTo(vista.largura, Math.round(y) + 0.5);
          }
        });
        ctx.stroke();
      }
    } else if (fundo === 'pontos') {
      ctx.fillStyle = cores.ponto;
      for (const x of gx.pos) for (const y of gy.pos) ctx.fillRect(Math.round(x), Math.round(y), 1.5, 1.5);
    }
  }

  /** Pede o quadro; se a vista não mudou e a versão vale, o motor responde "nada mudou". */
  function pedirQuadro(): void {
    const chaveDaVista = JSON.stringify(vista);
    const conhecida = quadro && chaveDaVista === vistaDoQuadro ? quadro.versao : null;
    const q: Quadro = medir('quadro', 'desenho', () => motor.quadro(vista, conhecida, temaVisivel() === 'light'), (r) => (r.tipo === 'novo' ? `${r.cores.length} segmentos` : 'igual'));
    if (q.tipo === 'novo') {
      quadro = { versao: q.versao, xs: q.xs, cores: q.cores, espessuras: q.espessuras, textos: q.textos, preenchimentos: q.preenchimentos, fundoX: q.fundoX, fundoY: q.fundoY };
      vistaDoQuadro = chaveDaVista;
      pintarFundo();
    }
  }

  function linha(ctx: CanvasRenderingContext2D, s: Segmento, cor: string, tracejada: boolean): void {
    ctx.strokeStyle = cor;
    ctx.setLineDash(tracejada ? [5, 4] : []);
    ctx.beginPath();
    ctx.moveTo(s.x1, s.y1);
    ctx.lineTo(s.x2, s.y2);
    ctx.stroke();
  }

  function posicionarCaixa(caixa: HTMLElement, x: number, y: number, texto: string): void {
    caixa.hidden = false;
    caixa.textContent = texto;
    caixa.style.left = `${x + 16}px`;
    caixa.style.top = `${y + 16}px`;
  }

  /** A janela de seleção: azul e cheia (pega o que cabe inteiro) ou verde e tracejada (pega o que toca). */
  function pintarJanelaDeSelecao(ctx: CanvasRenderingContext2D, lados: Segmento[], tipo: 'dentro' | 'toca'): void {
    const xs = lados.flatMap((s) => [s.x1, s.x2]);
    const ys = lados.flatMap((s) => [s.y1, s.y2]);
    const x = Math.min(...xs);
    const y = Math.min(...ys);
    const larg = Math.max(...xs) - x;
    const alt = Math.max(...ys) - y;
    const cor = tipo === 'dentro' ? cores.janelaDentro : cores.janelaToca;
    ctx.save();
    ctx.globalAlpha = 0.16;
    ctx.fillStyle = cor;
    ctx.fillRect(x, y, larg, alt);
    ctx.globalAlpha = 1;
    ctx.strokeStyle = cor;
    ctx.lineWidth = 1;
    ctx.setLineDash(tipo === 'toca' ? [6, 4] : []);
    ctx.strokeRect(x + 0.5, y + 0.5, larg, alt);
    ctx.restore();
  }

  function pintarTopo(): void {
    const ctx = tela2d(topo);
    ctx.clearRect(0, 0, vista.largura, vista.altura);
    if (!cursor) {
      caixaXY.hidden = caixaDA.hidden = true;
      return;
    }
    if (arrastados) pintarObjetos(ctx, arrastados);
    ctx.lineWidth = 1;
    if (cursor.janela && cursor.previa.length > 0) {
      pintarJanelaDeSelecao(ctx, cursor.previa, cursor.janela);
    } else {
      for (const trecho of cursor.previa) linha(ctx, trecho, cores.previa, false);
    }
    for (const g of cursor.guias) linha(ctx, g, cores.guia, true);
    ctx.setLineDash([]);
    ctx.strokeStyle = cores.cursor;
    ctx.beginPath();
    ctx.moveTo(cursor.x - TAMANHO_DA_MIRA, cursor.y);
    ctx.lineTo(cursor.x + TAMANHO_DA_MIRA, cursor.y);
    ctx.moveTo(cursor.x, cursor.y - TAMANHO_DA_MIRA);
    ctx.lineTo(cursor.x, cursor.y + TAMANHO_DA_MIRA);
    ctx.stroke();
    if (cursor.marca) {
      ctx.strokeStyle = cores.osnap;
      ctx.lineWidth = 1.5;
      marcaDeSnap(ctx, cursor.snap, cursor.x, cursor.y);
      ctx.lineWidth = 1;
    }
    posicionarCaixa(caixaXY, cursor.x, cursor.y, `${cursor.texto_x}  ${cursor.texto_y}`);
    if (cursor.texto_distancia !== null) {
      posicionarCaixa(caixaDA, cursor.x, cursor.y + 22, `${t.entrada.distancia} ${cursor.texto_distancia}  ${t.entrada.angulo} ${cursor.texto_angulo ?? ''}`);
    } else {
      caixaDA.hidden = true;
    }
  }

  /** A vista mudou de zoom: pede o quadro, o texto do zoom e refaz o cursor. */
  function vistaMudou(): void {
    pedirQuadro();
    cx.aoZoom(motor.vistaTexto(vista));
    if (mouse) moverCursor(mouse.x, mouse.y);
  }

  function moverCursor(x: number, y: number): void {
    mouse = { x, y };
    cursor = motor.cursorMover(vista, x, y);
    arrastados = null;
    if (pegou) {
      const q = motor.quadroArrasto(vista, x, y, temaVisivel() === 'light');
      if (q.tipo === 'novo') arrastados = q;
    }
    pintarTopo();
    cx.aoCursor(cursor);
  }

  function posicao(e: MouseEvent): { x: number; y: number } {
    const r = el.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  let arrastando: { x: number; y: number } | null = null;
  let pendente: { x: number; y: number } | null = null;
  let agendado = false;
  let ultimoCliqueDaRoda = 0;
  /** Sem comando ativo, apertar o mouse sobre um objeto o "pega" (grab): arrastar move, soltar sem andar é um clique. */
  let pegou: { x: number; y: number } | null = null;
  let ferramenta = 'nenhuma';
  let janelaPendente = false;
  /** O botão desceu no vazio e abriu a janela de seleção: se o mouse andar até soltar, a janela fecha ali (arrastar). */
  let janelaPorArrasto: { x: number; y: number; aditivo: boolean } | null = null;

  el.addEventListener('mousemove', (e) => {
    const p = posicao(e);
    if (arrastando) {
      vista = motor.vistaDeslocar(vista, p.x - arrastando.x, p.y - arrastando.y);
      arrastando = p;
      pedirQuadro();
    }
    // O mouse dispara várias vezes por quadro; o motor só é consultado uma vez por quadro de tela.
    pendente = p;
    if (!agendado) {
      agendado = true;
      requestAnimationFrame(() => {
        agendado = false;
        if (pendente) moverCursor(pendente.x, pendente.y);
        pendente = null;
      });
    }
  });
  el.addEventListener('mousedown', (e) => {
    const p = posicao(e);
    if (e.button === 1) {
      const agora = Date.now();
      if (agora - ultimoCliqueDaRoda < DUPLO_CLIQUE_MS) {
        vista = motor.vistaAjustar(vista);
        vistaMudou();
      }
      ultimoCliqueDaRoda = agora;
      arrastando = p;
      e.preventDefault();
    } else if (e.button === 0) {
      if (!e.shiftKey && ferramenta === 'nenhuma') {
        const estado = motor.arrastoIniciar(vista, p.x, p.y);
        if (estado.arrastando) {
          pegou = p;
          cx.aoMudar(estado);
          moverCursor(p.x, p.y);
          return;
        }
      }
      const antes = janelaPendente;
      const depois = motor.clique(vista, p.x, p.y, e.shiftKey);
      cx.aoMudar(depois);
      // Abriu a janela agora (não era o 2º clique de uma já aberta): arrastar até soltar a fecha.
      janelaPorArrasto = !antes && depois.janela_pendente ? { x: p.x, y: p.y, aditivo: e.shiftKey } : null;
      moverCursor(p.x, p.y);
    } else if (e.button === 2) {
      // Botão direito: o menu do momento (o motor monta; o Enter é o primeiro item quando há comando).
      const r = motor.menuContexto(vista, p.x, p.y);
      cx.aoMudar(r.estado);
      moverCursor(p.x, p.y);
      cx.aoMenu(e.clientX, e.clientY, r.itens);
    }
  });
  el.addEventListener('contextmenu', (e) => e.preventDefault());
  window.addEventListener('mouseup', (e) => {
    arrastando = null;
    if (janelaPorArrasto && e.button === 0) {
      const inicio = janelaPorArrasto;
      janelaPorArrasto = null;
      const p = posicao(e);
      // Andou: soltar fecha a janela (esquerda→direita azul, direita→esquerda verde). Não andou: segue o 2º clique.
      if (Math.hypot(p.x - inicio.x, p.y - inicio.y) >= LIMIAR_DO_ARRASTO_PX) {
        cx.aoMudar(motor.clique(vista, p.x, p.y, inicio.aditivo));
        moverCursor(p.x, p.y);
      }
    }
    if (!pegou || e.button !== 0) return;
    const inicio = pegou;
    pegou = null;
    const p = posicao(e);
    if (Math.hypot(p.x - inicio.x, p.y - inicio.y) >= LIMIAR_DO_ARRASTO_PX) {
      cx.aoMudar(motor.arrastoFim(vista, p.x, p.y));
    } else {
      // Quase não andou: era um clique.
      motor.arrastoCancelar();
      cx.aoMudar(motor.clique(vista, inicio.x, inicio.y, false));
    }
    moverCursor(p.x, p.y);
  });
  el.addEventListener('mouseleave', () => {
    cursor = null;
    pintarTopo();
    cx.aoCursor(null);
  });
  el.addEventListener(
    'wheel',
    (e) => {
      e.preventDefault();
      const p = posicao(e);
      vista = motor.vistaZoom(vista, p.x, p.y, e.deltaY < 0 ? FATOR_DO_ZOOM : 1 / FATOR_DO_ZOOM);
      mouse = p;
      vistaMudou();
    },
    { passive: false },
  );
  new ResizeObserver(ajustarTamanho).observe(el);
  // Trocou o tema: relê as cores, e o quadro guardado não vale (o branco do CAD inverte).
  lerCores();
  aoMudarTema(() => {
    lerCores();
    quadro = null;
    pedirQuadro();
    pintarTopo();
  });

  return {
    el,
    atualizar(e) {
      ferramenta = e.ferramenta;
      janelaPendente = e.janela_pendente;
      // Trocou o estilo do fundo: o quadro guardado não vale mais.
      if (e.fundo !== fundo) {
        fundo = e.fundo;
        quadro = null;
      }
      pedirQuadro();
      if (mouse) moverCursor(mouse.x, mouse.y);
    },
    ultimoCursor: () => cursor,
    zoomMais,
    zoomMenos,
    ajustar,
  };
}
