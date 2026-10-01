/**
 * A área de desenho: duas telas (o quadro do desenho e, por cima, cursor +
 * guias + prévia) e as caixinhas da entrada dinâmica. **Só desenha o que o
 * motor manda** — o ponto do mouse vai cru ao `cursor_mover` e volta resolvido
 * (snap, orto, grade), já com o texto das coordenadas.
 *
 * Zoom e pan também são contas do motor (`vista_zoom`, `vista_deslocar`): o
 * gesto vai e a vista nova volta. O TypeScript guarda a vista, não mexe nela.
 */

import { etapa, quadroDePerfil, registrar } from '../ponte/perfil.ts';
import { criarBarraNavegacao } from './barra-navegacao.ts';
import { criarMedidorFps } from './medidor-fps.ts';
import { pintarEmSegundoPlano, textoNoMovimento, zoomSuave } from './preferencias.ts';
import { criarPintor } from './pintor-quadro.ts';
import { pintarEixoUCS, pintarObjetos, type ObjetosDoQuadro } from './pintura.ts';
import { criarVistaCubo } from './vista-cubo.ts';
import { aoMudarTema, corDoTema, temaVisivel } from './tema.ts';
import type { Estado, Fundo, ItemDeMenu, Motor, RespostaCursor, Segmento, TipoDeSnap, Vista } from '../ponte/index.ts';
import type { Textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export const AREA_DESENHO_CSS = `
.area { position: relative; overflow: hidden; background: var(--fundo-area); cursor: none; }
.area canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
.area .desenho { z-index: ${Z.desenho}; transform-origin: 0 0; will-change: transform; }
.area .textos { z-index: ${Z.textos}; transform-origin: 0 0; will-change: transform; }
.area .sobreposicao { z-index: ${Z.sobreposicao}; }
.area .caixa { position: absolute; z-index: ${Z.entradaDinamica}; padding: 1px 5px; background: var(--caixa-dinamica); border: 1px solid var(--destaque); color: var(--texto); font: 11px var(--fonte); pointer-events: none; white-space: nowrap; }
`;

const TAMANHO_DA_MIRA = 10;
const FATOR_DO_ZOOM = 1.25;
/** Dois cliques da roda em menos que isto enquadram o desenho (como no AutoCAD). */
const DUPLO_CLIQUE_MS = 350;
/** Quanto o mouse precisa andar, em pixels, para o "grab" valer como arrasto e não como clique. */
const LIMIAR_DO_ARRASTO_PX = 4;
/** Quanto tempo depois do último movimento (zoom/pan) o texto real volta no lugar do traço. */
const ESPERA_DO_TEXTO_MS = 150;
/** Zoom suave: a cada quadro a vista anda esta fração do que falta (constante de tempo, em ms). Ao redor de 3× isto o gesto termina. */
const TEMPO_DO_ZOOM_SUAVE_MS = 70;
/** O que falta de zoom (em log) abaixo disto termina de uma vez. */
const RESTO_DO_ZOOM_SUAVE = 0.003;
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

/** Cria a área de desenho. */
export function criarAreaDesenho(cx: Contexto): AreaDesenho {
  const { motor, t } = cx;
  const el = document.createElement('main');
  el.className = 'area';
  const telaDoFundo = document.createElement('canvas');
  telaDoFundo.className = 'desenho';
  const telaDosTextos = document.createElement('canvas');
  telaDosTextos.className = 'textos';
  const topo = document.createElement('canvas');
  topo.className = 'sobreposicao';
  const caixaXY = document.createElement('div');
  const caixaDA = document.createElement('div');
  caixaXY.className = caixaDA.className = 'caixa';
  caixaXY.hidden = caixaDA.hidden = true;
  const fps = criarMedidorFps(t);
  // Quem pinta o quadro: um Worker (a thread principal fica livre) ou, se a webview não suporta, a própria thread principal.
  const pintor = criarPintor(telaDoFundo, telaDosTextos, pintarEmSegundoPlano());
  el.append(telaDoFundo, telaDosTextos, topo, caixaXY, caixaDA, fps.el);
  const zoomMais = (): void => pedirZoom(vista.largura / 2, vista.altura / 2, FATOR_DO_ZOOM);
  const zoomMenos = (): void => pedirZoom(vista.largura / 2, vista.altura / 2, 1 / FATOR_DO_ZOOM);
  const ajustar = (): void => {
    vista = motor.vistaAjustar(vista);
    vistaMudou();
  };
  const vistaCubo = criarVistaCubo(
    t,
    (rx, rz) => {
      vista = { ...vista, rotacao_x: rx, rotacao_z: rz };
      vistaMudou();
    },
    (perspectiva) => {
      vista = { ...vista, perspectiva };
      vistaMudou();
    }
  );
  const barraNav = criarBarraNavegacao(
    t,
    zoomMais,
    zoomMenos,
    ajustar,
    () => {
      // Toggle Órbita 3D
      vista = { ...vista, rotacao_x: (vista.rotacao_x ?? 0) + 15, rotacao_z: (vista.rotacao_z ?? 0) + 15 };
      vistaMudou();
    },
    undefined,
    () => {
      // Toggle Perspectiva / Ortográfica
      vista = { ...vista, perspectiva: !vista.perspectiva };
      vistaCubo.definirModoProjecao(Boolean(vista.perspectiva));
      vistaMudou();
    }
  );
  el.append(vistaCubo.el, barraNav);

  let vista: Vista = motor.vistaInicial(800, 600);
  /** A versão do último quadro pintado (o motor responde "nada mudou" se ela ainda vale e a vista é a mesma). */
  let versaoDoQuadro: number | null = null;
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
    topo.width = Math.round(width * dpr);
    topo.height = Math.round(height * dpr);
    tela2d(topo).setTransform(dpr, 0, 0, dpr, 0, 0);
    pintor.tamanho(width, height, dpr);
    vista = { ...vista, largura: width, altura: height };
    vistaDoFundo = vistaDosTextos = null;
    telaDoFundo.style.transform = telaDosTextos.style.transform = '';
    pedirQuadro();
    pintarTopo();
    cx.aoZoom(motor.vistaTexto(vista));
  }

  /** A vista com que cada camada foi pintada: até o quadro novo chegar, ela só é escalada/deslocada por CSS. */
  let vistaDoFundo: Vista | null = null;
  let vistaDosTextos: Vista | null = null;
  /** A vista está em movimento (zoom ou pan): o texto acompanha por CSS, sem ser redesenhado a cada quadro. */
  let emMovimentoAgora = false;
  let acabamento: ReturnType<typeof setTimeout> | null = null;

  const mesmaVista = (a: Vista, b: Vista): boolean =>
    a.escala === b.escala &&
    a.centro.x === b.centro.x &&
    a.centro.y === b.centro.y &&
    (a.centro.z ?? 0) === (b.centro.z ?? 0) &&
    a.largura === b.largura &&
    a.altura === b.altura &&
    (a.rotacao_x ?? 0) === (b.rotacao_x ?? 0) &&
    (a.rotacao_z ?? 0) === (b.rotacao_z ?? 0);

  /**
   * Leva uma camada já pintada com a vista `v0` até a vista de agora só com `transform` (a placa de vídeo escala a
   * imagem; nada é rasterizado de novo). A matriz é a composição das duas projeções mundo→tela (`Vista::para_tela`, no motor).
   */
  function acompanhar(tela: HTMLCanvasElement, v0: Vista | null): void {
    if (!v0) return;
    if (mesmaVista(v0, vista)) {
      tela.style.transform = '';
      return;
    }
    // Quando a rotação 3D muda, uma matriz afim 2D não se aplica e causaria deformação/salto da imagem antiga
    if (
      Math.abs((v0.rotacao_x ?? 0) - (vista.rotacao_x ?? 0)) > 1e-4 ||
      Math.abs((v0.rotacao_z ?? 0) - (vista.rotacao_z ?? 0)) > 1e-4
    ) {
      tela.style.transform = '';
      return;
    }
    const k = vista.escala / v0.escala;
    const radX = ((vista.rotacao_x ?? 0) * Math.PI) / 180;
    const radZ = ((vista.rotacao_z ?? 0) * Math.PI) / 180;
    const cosZ = Math.cos(radZ), sinZ = Math.sin(radZ);
    const cosX = Math.cos(radX), sinX = Math.sin(radX);
    const cx0 = v0.centro.x * cosZ + v0.centro.y * sinZ;
    const cy0 = (-v0.centro.x * sinZ + v0.centro.y * cosZ) * cosX + (v0.centro.z ?? 0) * sinX;
    const cx1 = vista.centro.x * cosZ + vista.centro.y * sinZ;
    const cy1 = (-vista.centro.x * sinZ + vista.centro.y * cosZ) * cosX + (vista.centro.z ?? 0) * sinX;

    const tx = (vista.largura / 2) * (1 - k) + (cx0 - cx1) * vista.escala;
    const ty = (vista.altura / 2) * (1 - k) - (cy0 - cy1) * vista.escala;
    tela.style.transform = `matrix(${k}, 0, 0, ${k}, ${tx}, ${ty})`;
  }

  /** A vista mudou: as duas camadas seguem o gesto na hora, sem esperar o quadro novo do motor. */
  function acompanharCamadas(): void {
    acompanhar(telaDoFundo, vistaDoFundo);
    acompanhar(telaDosTextos, vistaDosTextos);
  }

  /** Marca que a vista está em movimento e agenda o texto nítido para quando ele parar. */
  function emMovimento(): void {
    // com o texto completo ligado nas Configurações, o texto é redesenhado a cada quadro (mais lento no zoom)
    if (textoNoMovimento()) return;
    emMovimentoAgora = true;
    if (acabamento !== null) clearTimeout(acabamento);
    acabamento = setTimeout(() => {
      acabamento = null;
      emMovimentoAgora = false;
      const v = vistaDoFundo;
      if (v) {
        void pintor.pintarTextos(v).then(() => {
          vistaDosTextos = v;
          acompanhar(telaDosTextos, v);
        });
      }
    }, ESPERA_DO_TEXTO_MS);
  }

  /** Há um pedido de quadro no ar; só um por vez, e o último estado vale (ver `pedirQuadro`). */
  let emVoo = false;
  let querDeNovo = false;

  /**
   * Pede o quadro ao motor **sem travar a interface**: o Rust calcula em outra thread e os bytes voltam crus (IPC
   * binário). Só um pedido por vez; se a vista (ou o desenho) mudou enquanto ele voava, o quadro que chega ainda é
   * pintado (as camadas acompanham por CSS) e logo sai outro pedido, com o estado de agora. Se a vista não mudou e a
   * versão vale, o motor responde "nada mudou".
   */
  function pedirQuadro(): void {
    if (emVoo) {
      querDeNovo = true;
      return;
    }
    emVoo = true;
    querDeNovo = false;
    const vistaPedida = vista;
    const chave = JSON.stringify(vistaPedida);
    const conhecida = versaoDoQuadro !== null && chave === vistaDoQuadro ? versaoDoQuadro : null;
    const inicio = performance.now();
    let comTextos = true;
    motor
      .quadroBrutoAsync(vistaPedida, conhecida, temaVisivel() === 'light')
      .then((bytes) => {
        // Em movimento a camada de textos não é refeita (o CSS a acompanha); ao parar, `emMovimento` a redesenha.
        comTextos = !(emMovimentoAgora && vistaDosTextos);
        return pintor.pintar({ bytes, vista: vistaPedida, modo: fundo, cores: { linha: cores.linha, principal: cores.principal, ponto: cores.ponto }, comTextos });
      })
      .then((r) => {
        emVoo = false;
        quadroDePerfil('quadro-chegou', () => {
          registrar('quadro/latencia(pedido→resposta)', performance.now() - inicio);
          if (r.tipo === 'novo') {
            versaoDoQuadro = r.versao;
            vistaDoQuadro = chave;
            vistaDoFundo = vistaPedida;
            if (comTextos) vistaDosTextos = vistaPedida;
            acompanharCamadas();
            // O que o pintor gastou (no Worker, fora da thread principal): aparece como detalhe no perfil.
            registrar('pintor/decodificar', r.ms.decodificar);
            registrar('pintor/fundo', r.ms.fundo);
            registrar('pintor/textos', r.ms.textos);
            fps.info(`${r.segmentos} ${t.fps.segmentos} · ${r.textos} ${t.fps.textos}${pintor.emWorker ? '' : ' · sem Worker'}`);
          }
        });
        if (querDeNovo || JSON.stringify(vista) !== chave) pedirQuadro();
      })
      .catch((erro: unknown) => {
        emVoo = false;
        console.error('quadro:', erro);
      });
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
    const em3D = Math.abs(vista.rotacao_x ?? 0) > 1e-4 || Math.abs(vista.rotacao_z ?? 0) > 1e-4 || Math.abs(cursor.z ?? 0) > 1e-4;
    const coordsTexto = em3D ? `${cursor.texto_x}  ${cursor.texto_y}  ${cursor.texto_z}` : `${cursor.texto_x}  ${cursor.texto_y}`;
    posicionarCaixa(caixaXY, cursor.x, cursor.y, coordsTexto);
    if (cursor.texto_distancia !== null) {
      posicionarCaixa(caixaDA, cursor.x, cursor.y + 22, `${t.entrada.distancia} ${cursor.texto_distancia}  ${t.entrada.angulo} ${cursor.texto_angulo ?? ''}`);
    } else {
      caixaDA.hidden = true;
    }
    pintarEixoUCS(ctx, vista, vista.largura, vista.altura);
  }

  /** A vista mudou de zoom: pede o quadro, o texto do zoom e refaz o cursor. */
  function vistaMudou(): void {
    acompanharCamadas();
    pedirQuadro();
    cx.aoZoom(motor.vistaTexto(vista));
    vistaCubo.atualizar(vista.rotacao_x ?? 0, vista.rotacao_z ?? 0);
    if (mouse) moverCursor(mouse.x, mouse.y);
  }

  function moverCursor(x: number, y: number): void {
    mouse = { x, y };
    cursor = fps.medir('cursor', () => etapa('cursor', () => motor.cursorMover(vista, x, y)));
    arrastados = null;
    if (pegou) {
      const q = etapa('quadro_arrasto', () => motor.quadroArrasto(vista, x, y, temaVisivel() === 'light'));
      if (q.tipo === 'novo') arrastados = q;
    }
    fps.medir('pintura', () => etapa('pintar_topo', pintarTopo));
    const atual = cursor;
    if (atual) {
      vistaCubo.atualizarCoordenadas(atual.x, atual.y, atual.z ?? 0);
    }
    etapa('ao_cursor', () => cx.aoCursor(atual));
  }

  function posicao(e: MouseEvent): { x: number; y: number } {
    const r = el.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  let arrastando: { x: number; y: number } | null = null;
  let pendente: { x: number; y: number } | null = null;
  let agendado = false;
  // Pan e zoom também são agrupados por quadro de tela: o mouse (e a roda) disparam várias vezes por
  // quadro, e cada um custava um quadro inteiro do motor.
  let panPendente = { dx: 0, dy: 0 };
  let zoomPendente: { x: number; y: number; fator: number } | null = null;

  /** Aplica de uma vez o pan e o zoom acumulados desde o último quadro de tela. */
  function aplicarMovimentoPendente(): void {
    let mudou = false;
    if (panPendente.dx !== 0 || panPendente.dy !== 0) {
      const { dx, dy } = panPendente;
      vista = etapa('vista_deslocar', () => motor.vistaDeslocar(vista, dx, dy));
      panPendente = { dx: 0, dy: 0 };
      mudou = true;
    }
    if (zoomPendente) {
      const z = zoomPendente;
      vista = etapa('vista_zoom', () => motor.vistaZoom(vista, z.x, z.y, z.fator));
      mouse = { x: z.x, y: z.y };
      zoomPendente = null;
      cx.aoZoom(etapa('vista_texto', () => motor.vistaTexto(vista)));
      mudou = true;
    }
    if (mudou) {
      emMovimento();
      acompanharCamadas();
      pedirQuadro();
    }
  }

  /** Zoom suave em andamento: o que falta (fator) e o ponto fixo (o mouse, ou o centro nos botões). */
  let zoomAnimado: { x: number; y: number; falta: number; ultimo: number } | null = null;

  /** Pede um zoom de `fator` em volta de `(x, y)`: animado se a preferência está ligada, senão de uma vez. */
  function pedirZoom(x: number, y: number, fator: number): void {
    if (!zoomSuave()) {
      const acum = (zoomPendente?.fator ?? 1) * fator;
      zoomPendente = { x, y, fator: Math.max(0.1, Math.min(10.0, acum)) };
      agendarQuadro();
      return;
    }
    const rodando = zoomAnimado !== null;
    const acum = (zoomAnimado?.falta ?? 1) * fator;
    // Trava para evitar disparo descontrolado no mouse/trackpad
    const falta = Math.max(0.25, Math.min(4.0, acum));
    zoomAnimado = { x, y, falta, ultimo: zoomAnimado?.ultimo ?? performance.now() };
    if (!rodando) requestAnimationFrame(passoDoZoom);
  }

  /** Um quadro do zoom suave: aplica uma fração do que falta (o cálculo da vista continua sendo do motor). */
  function passoDoZoom(agora: number): void {
    const z = zoomAnimado;
    if (!z) return;
    const fracao = 1 - Math.exp(-Math.max(1, agora - z.ultimo) / TEMPO_DO_ZOOM_SUAVE_MS);
    const acabou = Math.abs(Math.log(z.falta)) < RESTO_DO_ZOOM_SUAVE;
    const passo = acabou ? z.falta : Math.pow(z.falta, fracao);
    zoomPendente = { x: z.x, y: z.y, fator: passo };
    fps.quadro();
    quadroDePerfil('zoom', () =>
      fps.medir('total', () => {
        aplicarMovimentoPendente();
        moverCursor(z.x, z.y);
      }),
    );
    if (acabou) {
      zoomAnimado = null;
    } else {
      z.falta /= passo;
      z.ultimo = agora;
      requestAnimationFrame(passoDoZoom);
    }
  }

  function agendarQuadro(): void {
    if (agendado) return;
    agendado = true;
    requestAnimationFrame(() => {
      agendado = false;
      fps.quadro();
      // o tipo do quadro no perfil: o que este quadro de tela vai fazer além de mexer o cursor
      const tipo = panPendente.dx !== 0 || panPendente.dy !== 0 ? 'pan' : zoomPendente ? 'zoom-seco' : 'mouse';
      quadroDePerfil(tipo, () =>
        fps.medir('total', () => {
          aplicarMovimentoPendente();
          if (pendente) moverCursor(pendente.x, pendente.y);
          else if (mouse) moverCursor(mouse.x, mouse.y);
          pendente = null;
        }),
      );
    });
  }
  let ultimoCliqueDaRoda = 0;
  /** Sem comando ativo, apertar o mouse sobre um objeto o "pega" (grab): arrastar move, soltar sem andar é um clique. */
  let pegou: { x: number; y: number } | null = null;
  let ferramenta = 'nenhuma';
  let janelaPendente = false;
  /** O botão desceu no vazio e abriu a janela de seleção: se o mouse andar até soltar, a janela fecha ali (arrastar). */
  let janelaPorArrasto: { x: number; y: number; aditivo: boolean } | null = null;

  let orbitando: { x: number; y: number } | null = null;

  el.addEventListener('mousemove', (e) => {
    const p = posicao(e);
    if (orbitando) {
      const dx = p.x - orbitando.x;
      const dy = p.y - orbitando.y;
      orbitando = p;
      let nRz = ((vista.rotacao_z ?? 0) + dx * 0.5) % 360;
      if (nRz < 0) nRz += 360;
      let nRx = Math.max(-180, Math.min(180, (vista.rotacao_x ?? 0) + dy * 0.5));
      vista = { ...vista, rotacao_x: nRx, rotacao_z: nRz };
      vistaMudou();
      return;
    }
    if (arrastando) {
      panPendente = { dx: panPendente.dx + p.x - arrastando.x, dy: panPendente.dy + p.y - arrastando.y };
      arrastando = p;
    }
    // O mouse dispara várias vezes por quadro; o motor só é consultado uma vez por quadro de tela.
    pendente = p;
    agendarQuadro();
  });
  el.addEventListener('mousedown', (e) => {
    const p = posicao(e);
    if (e.button === 1) {
      if (e.shiftKey) {
        orbitando = p;
        e.preventDefault();
        return;
      }
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
    orbitando = null;
    aplicarMovimentoPendente();
    if (janelaPorArrasto && e.button === 0) {
      const inicio = janelaPorArrasto;
      janelaPorArrasto = null;
      const p = posicao(e);
      // Andou: soltar fecha a janela (esquerda→direita azul, direita→esquerda verde). Não andou: segue o 2º clique.
      if (Math.hypot(p.x - inicio.x, p.y - inicio.y) >= LIMIAR_DO_ARRASTO_PX) {
        cx.aoMudar(motor.clique(vista, p.x, p.y, inicio.aditivo));
        moverCursor(p.x, p.y);
      } else {
        cx.aoMudar(motor.cancelar());
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
      let dy = e.deltaY;
      if (e.deltaMode === 1) dy *= 30; // DOM_DELTA_LINE
      else if (e.deltaMode === 2) dy *= 600; // DOM_DELTA_PAGE
      const dyTravado = Math.max(-120, Math.min(120, dy));
      // Proporcional ao deslocamento da roda (1 clique clássico ~100px dá FATOR_DO_ZOOM = 1.25)
      const fator = Math.pow(FATOR_DO_ZOOM, -dyTravado / 100);
      mouse = p;
      pedirZoom(p.x, p.y, fator);
    },
    { passive: false },
  );
  new ResizeObserver(ajustarTamanho).observe(el);
  // Trocou o tema: relê as cores, e o quadro guardado não vale (o branco do CAD inverte).
  lerCores();
  aoMudarTema(() => {
    lerCores();
    versaoDoQuadro = null;
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
        versaoDoQuadro = null;
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
