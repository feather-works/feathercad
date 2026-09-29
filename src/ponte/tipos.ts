/** As formas que o motor devolve (ver `crates/feathercad-ponte/src/lib.rs`). */

export interface Ponto {
  x: number;
  y: number;
}

/** A janela por onde se olha o desenho. Toda conta sobre ela é do Rust. */
/** O que o diálogo de exportação pede ao motor (o Rust valida e monta o PDF). */
export interface OpcoesPdf {
  papel: 'A4' | 'A3' | 'A2' | 'A1' | 'A0' | 'FOLHA';
  orientacao: 'automatica' | 'horizontal' | 'vertical';
  /** `true` mantém as cores da tela; `false` é o modo papel (preto sobre branco). */
  cores: boolean;
  /** Posições (a partir de 0) das folhas a exportar; ausente = todas. */
  folhas?: number[];
}

/** Os tipos de snap ao objeto (a ordem é a do menu). */
export const TIPOS_DE_SNAP = ['extremidade', 'meio', 'centro', 'quadrante', 'intersecao', 'perpendicular', 'tangente', 'no', 'proximo'] as const;
export type TipoDeSnap = (typeof TIPOS_DE_SNAP)[number];

export interface Vista {
  centro: Ponto;
  escala: number;
  largura: number;
  altura: number;
}

/** Texto para a interface traduzir: `chave` no arquivo de língua + `{variáveis}`. */
export interface Mensagem {
  chave: string;
  vars: Record<string, string>;
}

export type Entrada = { digitado: string } | { mensagem: Mensagem };

/** Onde o desenho mora no disco. */
export interface ArquivoDoDesenho {
  caminho: string;
  nome: string;
  formato: 'dxf' | 'fcdz';
}

export type Fundo = 'nada' | 'pontos' | 'grade';

/** A unidade em que o desenho é trabalhado. */
export type Unidade = 'mm' | 'cm' | 'm';

/** Uma linha do menu do botão direito, como o motor a monta. */
export interface ItemDeMenu {
  id: string;
  chave: string;
  atalho: string | null;
  /** Interruptor: `true`/`false` mostra o visto; `null` = não é interruptor. */
  marcado: boolean | null;
  ativo: boolean;
  separador: boolean;
  /** "Repetir X": o comando repetido. */
  repetido: string | null;
}

export interface RespostaDeMenu {
  estado: Estado;
  itens: ItemDeMenu[];
}

export interface Estado {
  versao: number;
  /** Nada = arquivo aberto sem essa informação. */
  unidade: Unidade | null;
  /** O tamanho da cota escolhido (na unidade do desenho); nada = automático. */
  cota_tamanho: number | null;
  /** O passo do Orto, em graus (90 = só horizontal e vertical). */
  orto_angulo: number;
  /** Um objeto está sendo arrastado com o mouse (o "grab"). */
  arrastando: boolean;
  /** O primeiro canto de uma janela de seleção já foi marcado. */
  janela_pendente: boolean;
  sujo: boolean;
  arquivo: ArquivoDoDesenho | null;
  /** O que ficou de fora ou foi simplificado no último abrir/salvar. */
  avisos_do_arquivo: Mensagem[];
  pode_desfazer: boolean;
  pode_refazer: boolean;
  camada_atual: string;
  /** O nome interno da ferramenta em andamento (o do comando que a abriu), ou `nenhuma`. */
  ferramenta: string;
  /** Quantos objetos estão selecionados. */
  selecionados: number;
  /** Muda a cada mudança da seleção (mesmo com a mesma contagem). */
  selecao_rev: number;
  fundo: Fundo;
  snap: boolean;
  orto: boolean;
  osnap: boolean;
  /** Que tipos de snap ao objeto estão ligados. */
  osnap_modos: Record<TipoDeSnap, boolean>;
  prompt: Mensagem;
  /** Enquanto o Inserir espera o nome: os blocos do desenho para escolher. */
  blocos: string[];
  historico: Entrada[];
}

export interface Segmento {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/** Tudo do cursor num pacote só: ponto resolvido, guias, prévia e o texto da entrada dinâmica. */
export interface RespostaCursor {
  x: number;
  y: number;
  texto_x: string;
  texto_y: string;
  texto_distancia: string | null;
  texto_angulo: string | null;
  guias: Segmento[];
  /** A prévia do que está sendo desenhado (trechos já marcados + último até o cursor). */
  previa: Segmento[];
  marca: boolean;
  /** O tipo da marca de snap, se há marca. */
  snap: TipoDeSnap | null;
  /** Janela de seleção aberta: `dentro` (só o que cabe inteiro) ou `toca` (basta tocar); nada = sem janela. */
  janela: 'dentro' | 'toca' | null;
}

/** Os tipos de linha (o nome que o motor usa é o do DXF). */
export const TIPOS_DE_LINHA = ['CONTINUOUS', 'DASHED', 'HIDDEN', 'CENTER', 'DOT', 'PHANTOM'] as const;
export type TipoDeLinha = (typeof TIPOS_DE_LINHA)[number];

/** O nome do tipo de linha que o motor devolve na tabela → o nome do DXF (o que se manda de volta). */
export const TIPO_PARA_DXF = { continua: 'CONTINUOUS', tracejada: 'DASHED', oculta: 'HIDDEN', eixo: 'CENTER', pontilhada: 'DOT', fantasma: 'PHANTOM' } as const;

/** As espessuras de linha padrão do AutoCAD, em mm. */
export const ESPESSURAS_MM = [0, 0.05, 0.09, 0.13, 0.15, 0.18, 0.2, 0.25, 0.3, 0.35, 0.4, 0.5, 0.53, 0.6, 0.7, 0.8, 0.9, 1, 1.06, 1.2, 1.4, 1.58, 2, 2.11] as const;

/** Uma linha da tabela do gerenciador de camadas (do motor: já filtrada e ordenada). */
export interface LinhaDeCamada {
  /** O índice da camada no documento: com ele se pede uma mudança. */
  indice: number;
  nome: string;
  atual: boolean;
  /** Tem objetos. */
  usada: boolean;
  ligada: boolean;
  congelada: boolean;
  bloqueada: boolean;
  plotar: boolean;
  /** `#rrggbb`. */
  cor: string;
  /** `continua`… (serde snake_case do motor). */
  tipo_linha: 'continua' | 'tracejada' | 'oculta' | 'eixo' | 'pontilhada' | 'fantasma';
  /** Em mm; `null` = padrão. */
  espessura: number | null;
  transparencia: number;
}

export interface TabelaDeCamadas {
  linhas: LinhaDeCamada[];
  exibidas: number;
  totais: number;
  atual: string;
}

export type OrdemDeCamadas = 'status' | 'nome' | 'ligada' | 'congelada' | 'bloqueada' | 'plotar' | 'cor' | 'tipo_linha' | 'espessura' | 'transparencia';

/** O que a tabela pede ao motor (filtro, busca, coluna e sentido). */
export interface ConsultaDeCamadas {
  filtro: 'todas' | 'usadas';
  busca: string;
  ordem: OrdemDeCamadas;
  crescente: boolean;
}

export type AcaoDeCamada = 'nova' | 'renomear' | 'propriedade' | 'atual' | 'excluir';

export interface PedidoDeCamada {
  acao: AcaoDeCamada;
  indices?: number[];
  nome?: string;
  propriedade?: 'ligada' | 'congelada' | 'bloqueada' | 'plotar' | 'cor' | 'tipo_linha' | 'espessura' | 'transparencia';
  valor?: boolean | string | number | null;
  consulta: ConsultaDeCamadas;
}

export interface RespostaDeCamada {
  /** A chave da mensagem de erro (`camadas.erro.…`), se a operação (ou parte dela) não aconteceu. */
  erro: string | null;
  /** Quantas camadas saíram (excluir). */
  afetadas: number;
  tabela: TabelaDeCamadas;
  estado: Estado;
}

/** Um campo da paleta de propriedades: sem objetos, igual em todos ou variado. */
export type CampoDePropriedade = { estado: 'vazio' } | { estado: 'variado' } | { estado: 'igual'; valor: string };

export interface ResumoDePropriedades {
  n: number;
  camada: CampoDePropriedade;
  /** `porcamada` ou `#rrggbb`. */
  cor: CampoDePropriedade;
  cor_efetiva: CampoDePropriedade;
  /** `porcamada` ou o nome do DXF. */
  tipo_linha: CampoDePropriedade;
  /** `porcamada` ou milímetros. */
  espessura: CampoDePropriedade;
  /** As cores básicas (`#rrggbb`): vermelho, amarelo, verde, ciano, azul, magenta, branco. */
  cores_basicas: string[];
}

/** O que o Rust entende por regional (do arquivo de língua). */
export interface Regional {
  decimal: string;
  separador_de_coordenadas: string;
  casas: number;
}

/** O quadro do Rust já lido: segmentos em pixels, cor final, ou "nada mudou". */
/** Um texto do desenho, já em pixels e com a cor final. */
export interface TextoDoQuadro {
  x: number;
  y: number;
  /** Altura da letra em pixels. */
  altura: number;
  /** Giro em radianos, no sentido do canvas. */
  giro: number;
  h: 'left' | 'center' | 'right';
  base: 'alphabetic' | 'bottom' | 'middle' | 'top';
  cor: number;
  conteudo: string;
}

/** Uma área cheia (hachura sólida): laços em pixels, preenchidos par-ímpar. */
export interface Preenchimento {
  cor: number;
  /** Cada laço: x, y, x, y… */
  lacos: Float32Array[];
}

/** Uma linha (ou ponto) do fundo: posição em pixels e se é principal. */
export interface MarcasDoFundo {
  pos: Float32Array;
  principal: Uint8Array;
}

export type Quadro =
  | { tipo: 'igual' }
  | { tipo: 'novo'; versao: number; xs: Float32Array; cores: Uint32Array; espessuras: Float32Array; textos: TextoDoQuadro[]; preenchimentos: Preenchimento[]; fundoX: MarcasDoFundo; fundoY: MarcasDoFundo };
