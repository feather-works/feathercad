/**
 * O `Motor`: os comandos do Rust como métodos tipados. É o **único** lugar do
 * TypeScript que fala com o motor. Aqui não há conta nem regra de desenho:
 * cada método manda o pedido e devolve a resposta.
 */

import { canalMedido } from './metricas.ts';
import { chamarJson, type Canal } from './canal.ts';
import type { CamposFolha, OpcoesDeHachura, ConsultaDeCamadas, FolhasDoMotor, OpcoesPdf, PedidoFolhaNova, RespostaFolha, TipoDeSnap, Estado, Fundo, RespostaDeMenu, Unidade, MarcasDoFundo, PedidoDeCamada, RespostaDeCamada, ResumoDePropriedades, TabelaDeCamadas, Ponto, Preenchimento, Quadro, Regional, RespostaCursor, TextoDoQuadro, Vista } from './tipos.ts';

export type { Canal } from './canal.ts';
export type * from './tipos.ts';

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
  return { tipo: 'novo', versao, xs, cores, espessuras, textos, preenchimentos, fundoX, fundoY };
}

/**
 * Todo comando que a ponte atende e que o `Motor` usa. Na largada a interface pergunta ao motor a
 * lista dele (`comandos_da_ponte`) e, se faltar algum, mostra "motor desatualizado" em vez de
 * deixar erros soltos. Um teste trava esta lista contra `COMANDOS_DA_PONTE` (Rust).
 */
export const COMANDOS_DA_PONTE_ESPERADOS: readonly string[] = [
  'estado',
  'comandos',
  'comandos_da_ponte',
  'idioma_definir',
  'interruptor',
  'orto_angulo_definir',
  'osnap_modos_definir',
  'atalho',
  'digitar',
  'executar',
  'cancelar',
  'clique',
  'cursor_mover',
  'linha_criar',
  'desfazer',
  'refazer',
  'camadas',
  'camada_acao',
  'propriedades_selecao',
  'propriedades_definir',
  'vista_inicial',
  'vista_deslocar',
  'vista_zoom',
  'vista_zoom_centro',
  'vista_ajustar',
  'vista_texto',
  'fundo_definir',
  'documento_novo',
  'documento_unidade_definir',
  'cota_tamanho_definir',
  'menu_contexto',
  'arrasto_iniciar',
  'arrasto_fim',
  'arrasto_cancelar',
  'documento_abrir',
  'documento_gravar',
  'documento_exportar_pdf',
  'documento_folhas',
  'folha_listar',
  'folha_inserir',
  'folha_escala',
  'folha_ajustar',
  'folha_campos',
  'folha_campos_definir',
  'hachura_opcoes',
  'hachura_definir',
  'hachura_previa',
  'documento_arquivo_definir',
  'documento_arquivo',
  'quadro',
  'quadro_arrasto',
];

/** O motor de um documento. */
export class Motor {
  private readonly canal: Canal;

  constructor(canal: Canal) {
    this.canal = canalMedido(canal);
  }

  private json<T>(comando: string, args?: unknown): T {
    return chamarJson<T>(this.canal, comando, args);
  }

  /** Os comandos que o motor em execução **não** tem (a interface é mais nova que ele); vazio = tudo certo. */
  comandosQueFaltam(): string[] {
    let tem: string[];
    try {
      tem = this.json<string[]>('comandos_da_ponte');
    } catch {
      // Motor tão antigo que nem sabe responder a isto.
      return [...COMANDOS_DA_PONTE_ESPERADOS];
    }
    return COMANDOS_DA_PONTE_ESPERADOS.filter((c) => !tem.includes(c));
  }

  /** Os comandos que o motor conhece (o nome interno). */
  comandos(): string[] {
    return this.json('comandos');
  }

  estado(): Estado {
    return this.json('estado');
  }

  /** Manda a configuração regional e a tabela de comandos da língua. */
  idioma(regional: Regional, tabela: { comandos: Record<string, string[]>; opcoes: Record<string, string[]> }): Estado {
    return this.json('idioma_definir', { regional, comandos: tabela.comandos, opcoes: tabela.opcoes });
  }

  interruptor(nome: 'grade' | 'snap' | 'orto' | 'osnap', ligado: boolean): Estado {
    return this.json('interruptor', { nome, ligado });
  }

  /** Uma tecla de atalho (`Ctrl+G`, `F8`, `Esc`). `tratado` diz se é de algum comando. */
  atalho(tecla: string): { tratado: boolean; janela: 'novo' | 'abrir' | 'salvar' | 'salvar_como' | null; estado: Estado } {
    return this.json('atalho', { tecla });
  }

  /** Texto digitado na linha de comando (Enter). */
  digitar(texto: string): Estado {
    return this.json('digitar', { texto });
  }

  /** Executa um comando pelo nome interno (botão da Ribbon). */
  executar(comando: string): Estado {
    return this.json('executar', { comando });
  }

  cancelar(): Estado {
    return this.json('cancelar');
  }

  desfazer(): Estado {
    return this.json('desfazer');
  }

  refazer(): Estado {
    return this.json('refazer');
  }

  clique(vista: Vista, x: number, y: number, aditivo = false): Estado {
    return this.json('clique', { vista, x, y, aditivo });
  }

  cursorMover(vista: Vista, x: number, y: number): RespostaCursor {
    return this.json('cursor_mover', { vista, x, y });
  }

  linhaCriar(a: Ponto, b: Ponto): Estado {
    return this.json('linha_criar', { a, b });
  }

  /** A tabela do gerenciador de camadas: filtrada, buscada e ordenada pelo motor. */
  camadas(consulta?: ConsultaDeCamadas): TabelaDeCamadas {
    return this.json('camadas', consulta ?? {});
  }

  /** Uma ação sobre camadas (nova, renomear, propriedade, atual, excluir). */
  camadaAcao(pedido: PedidoDeCamada): RespostaDeCamada {
    return this.json('camada_acao', pedido);
  }

  /** O que os objetos selecionados têm em comum. */
  propriedadesSelecao(): ResumoDePropriedades {
    return this.json('propriedades_selecao');
  }

  /** Põe `valor` no `campo` de todos os objetos selecionados. */
  propriedadesDefinir(campo: 'camada' | 'cor' | 'tipo_linha' | 'espessura', valor: string): Estado {
    return this.json('propriedades_definir', { campo, valor });
  }

  vistaInicial(largura: number, altura: number): Vista {
    return this.json('vista_inicial', { largura, altura });
  }

  vistaDeslocar(vista: Vista, dx: number, dy: number): Vista {
    return this.json('vista_deslocar', { vista, dx, dy });
  }

  vistaZoom(vista: Vista, x: number, y: number, fator: number): Vista {
    return this.json('vista_zoom', { vista, x, y, fator });
  }

  /** Multiplica o zoom por `fator` em volta do centro da área. */
  vistaZoomCentro(vista: Vista, fator: number): Vista {
    return this.json('vista_zoom_centro', { vista, fator });
  }

  /** Enquadra o desenho inteiro ("zoom extensão"). */
  vistaAjustar(vista: Vista): Vista {
    return this.json('vista_ajustar', { vista });
  }

  /** O zoom como texto pronto (`100%`). */
  vistaTexto(vista: Vista): string {
    return this.json<{ zoom: string }>('vista_texto', { vista }).zoom;
  }

  fundoDefinir(modo: Fundo): Estado {
    return this.json('fundo_definir', { modo });
  }

  /** Desenho em branco. */
  documentoNovo(unidade?: Unidade): Estado {
    return this.json('documento_novo', { unidade: unidade ?? null });
  }

  /** O botão direito em (x, y): o objeto ali vira a seleção, e volta o menu que cabe ao momento. */
  menuContexto(vista: Vista, x: number, y: number): RespostaDeMenu {
    return this.json('menu_contexto', { vista, x, y, aditivo: false });
  }

  /** Mouse apertado: se há objeto ali, começa o "grab" (`arrastando` no estado diz se pegou). */
  arrastoIniciar(vista: Vista, x: number, y: number): Estado {
    return this.json('arrasto_iniciar', { vista, x, y, aditivo: false });
  }

  /** Mouse solto: move os objetos arrastados para onde o cursor está. */
  arrastoFim(vista: Vista, x: number, y: number): Estado {
    return this.json('arrasto_fim', { vista, x, y, aditivo: false });
  }

  /** O arrasto era só um clique: desiste sem mover. */
  arrastoCancelar(): Estado {
    return this.json('arrasto_cancelar');
  }

  /** Liga ou desliga tipos de snap ao objeto (só os citados mudam). */
  osnapModosDefinir(modos: Partial<Record<TipoDeSnap, boolean>>): Estado {
    return this.json('osnap_modos_definir', { modos });
  }

  /** De quantos em quantos graus o Orto trava a direção. */
  ortoAnguloDefinir(graus: number): Estado {
    return this.json('orto_angulo_definir', { graus });
  }

  /** O tamanho da cota (altura do texto, na unidade do desenho); `null` = automático. */
  cotaTamanhoDefinir(tamanho: number | null): Estado {
    return this.json('cota_tamanho_definir', { tamanho });
  }

  /** Troca a unidade do desenho que já existe (conta como alteração). */
  unidadeDefinir(unidade: Unidade | null): Estado {
    return this.json('documento_unidade_definir', { unidade });
  }

  /** Abre os bytes de um arquivo (DXF ou `.fcdz`, o motor reconhece pelo conteúdo). */
  documentoAbrir(bytes: Uint8Array): Estado {
    return this.json('documento_abrir', bytes);
  }

  /** Os bytes do desenho no formato pedido. */
  documentoGravar(formato: 'dxf' | 'fcdz'): Uint8Array {
    return this.canal.chamar('documento_gravar', { formato });
  }

  /** O PDF do desenho, uma página por folha (camada FOLHA). `cores` mantém as cores da tela em vez do modo papel. */
  documentoExportarPdf(opcoes: OpcoesPdf): Uint8Array {
    return this.canal.chamar('documento_exportar_pdf', opcoes);
  }

  /** As folhas do desenho (largura × altura, em unidades do desenho), na ordem em que saem no PDF. */
  documentoFolhas(): Array<[number, number]> {
    return this.json('documento_folhas');
  }

  /** As folhas inseridas, os papéis e as escalas oferecidas. */
  folhaListar(): FolhasDoMotor {
    return this.json('folha_listar');
  }

  /** Insere uma folha (o Rust valida papel e escala e escolhe onde ela fica). */
  folhaInserir(pedido: PedidoFolhaNova): RespostaFolha {
    return this.json('folha_inserir', pedido);
  }

  /** Troca a escala de uma folha (o centro da área útil fica no lugar). */
  folhaEscala(indice: number, escala: number): RespostaFolha {
    return this.json('folha_escala', { indice, escala });
  }

  /** Escolhe a menor escala em que o desenho todo cabe na folha e a centra nele. */
  folhaAjustar(indice: number): RespostaFolha {
    return this.json('folha_ajustar', { indice });
  }

  /** Os campos do carimbo de uma folha, como estão (vazio = o texto-guia do modelo). */
  folhaCampos(indice: number): CamposFolha {
    return this.json('folha_campos', { indice });
  }

  /** Reescreve os campos do carimbo (vazio volta ao texto-guia). */
  folhaCamposDefinir(indice: number, campos: CamposFolha): RespostaFolha {
    return this.json('folha_campos_definir', { indice, campos });
  }

  /** Os padrões de hachura, o escolhido e as escalas oferecidas. */
  hachuraOpcoes(): OpcoesDeHachura {
    return this.json('hachura_opcoes');
  }

  /** Escolhe o padrão (e a escala) da próxima hachura; o Rust valida o nome. */
  hachuraDefinir(padrao: string, escala?: number): OpcoesDeHachura {
    return this.json('hachura_definir', { padrao, ...(escala === undefined ? {} : { escala }) });
  }

  /** Uma amostra do padrão em SVG, desenhada pelo motor. */
  hachuraPrevia(padrao: string, escala: number): string {
    return this.json('hachura_previa', { padrao, escala });
  }

  /** Onde o desenho está no disco (`null` se ainda não foi salvo). */
  documentoArquivo(): Estado['arquivo'] {
    return this.json('documento_arquivo');
  }

  /** Diz ao motor onde o desenho está no disco; `salvo` = acabou de gravar. */
  documentoArquivoDefinir(caminho: string, formato: 'dxf' | 'fcdz', salvo: boolean): Estado {
    return this.json('documento_arquivo_definir', { caminho, formato, salvo });
  }

  /** Durante o "grab": só os objetos arrastados, desenhados onde o cursor os põe. */
  quadroArrasto(vista: Vista, x: number, y: number, fundoClaro = false): Quadro {
    return lerQuadro(this.canal.chamar('quadro_arrasto', { vista, x, y, fundo_claro: fundoClaro }));
  }

  /** O que está na tela. Passe `versaoConhecida` só se a vista e o tema não mudaram; `fundoClaro` diz se o tema é claro. */
  quadro(vista: Vista, versaoConhecida: number | null, fundoClaro = false): Quadro {
    return lerQuadro(this.canal.chamar('quadro', { vista, versao_conhecida: versaoConhecida, fundo_claro: fundoClaro }));
  }
}
