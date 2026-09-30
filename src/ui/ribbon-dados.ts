/**
 * A estrutura da Ribbon como **dado**: abas → painéis → botões. Sub-aba nova
 * entra aqui, sem tocar no componente nem nas outras abas. Botão com `comando`
 * precisa existir no motor (`test/ui/ribbon.test.ts` confere); botão com
 * `comando: null` é "em breve": aparece desabilitado, para a faixa já ter o
 * desenho final do AutoCAD e ir sendo preenchida. Os textos (`ribbon.abas.<id>`, `ribbon.paineis.<id>`,
 * `ribbon.botoes.<id>.rotulo/dica`) vêm do arquivo de língua.
 */

export interface BotaoDado {
  id: string;
  /** O nome interno do comando no motor; `null` = ainda não existe (botão desabilitado). */
  comando: string | null;
  /** Um símbolo curto (até haver ícone de verdade). */
  simbolo: string;
  /** Botão pequeno (ícone ao lado do texto, empilhado de 3 em 3); o padrão é grande. */
  pequeno?: boolean;
  /** Ação da **interface** (não do motor): `camadas` abre o gerenciador. O botão vale sem `comando`. */
  acao?: string;
  /** Não é botão: é um controle da faixa (lista, camada atual…), fabricado por `ribbon-widgets.ts`. */
  widget?: string;
}

export interface PainelDado {
  id: string;
  botoes: BotaoDado[];
  /** A ação da interface do "lançador" (↘) no canto do painel, se houver (`propriedades` abre a paleta). */
  lancador?: string;
}

export interface AbaDado {
  id: string;
  paineis: PainelDado[];
}

export const RIBBON: AbaDado[] = [
  {
    id: 'inicio',
    paineis: [
      { id: 'desenhar', botoes: [
        { id: 'linha', comando: 'linha', simbolo: '╱' },
        { id: 'polilinha', comando: 'polilinha', simbolo: '⌇' },
        { id: 'circulo', comando: 'circulo', simbolo: '◯' },
        { id: 'arco', comando: 'arco', simbolo: '◠' },
        { id: 'retangulo', comando: 'retangulo', simbolo: '▭', pequeno: true },
        { id: 'elipse', comando: 'elipse', simbolo: '⬭', pequeno: true },
        { id: 'hachura', comando: 'hachura', simbolo: '▨', pequeno: true },
        { id: 'hachura_padrao', comando: null, simbolo: '▩', pequeno: true, acao: 'hachura_padrao' },
      ] },
      { id: 'modificar', botoes: [
        { id: 'mover', comando: 'mover', simbolo: '✥', pequeno: true },
        { id: 'copiar', comando: 'copiar', simbolo: '⧉', pequeno: true },
        { id: 'esticar', comando: 'esticar', simbolo: '⇔', pequeno: true },
        { id: 'girar', comando: 'girar', simbolo: '↻', pequeno: true },
        { id: 'espelhar', comando: 'espelhar', simbolo: '⇋', pequeno: true },
        { id: 'escala', comando: 'escala', simbolo: '⤡', pequeno: true },
        { id: 'aparar', comando: 'aparar', simbolo: '✂', pequeno: true },
        { id: 'deslocar', comando: 'deslocar', simbolo: '⫽', pequeno: true },
        { id: 'estender', comando: 'estender', simbolo: '⇥', pequeno: true },
        { id: 'concordancia', comando: 'concordancia', simbolo: '⌒', pequeno: true },
        { id: 'matriz', comando: 'matriz', simbolo: '⁙', pequeno: true },
        { id: 'apagar', comando: 'apagar', simbolo: '✕', pequeno: true },
      ] },
      { id: 'anotacao', botoes: [
        { id: 'texto', comando: 'texto', simbolo: 'A' },
        { id: 'cota', comando: 'cota', simbolo: '⟷' },
        { id: 'cota_alinhada', comando: 'cota_alinhada', simbolo: '⤢', pequeno: true },
        { id: 'cota_raio', comando: 'cota_raio', simbolo: 'R', pequeno: true },
        { id: 'cota_diametro', comando: 'cota_diametro', simbolo: 'Ø', pequeno: true },
        { id: 'chamada', comando: 'chamada', simbolo: '↗', pequeno: true },
        { id: 'tabela', comando: 'tabela', simbolo: '▦', pequeno: true },
      ] },
      { id: 'camadas', botoes: [
        { id: 'propriedades_camada', comando: null, simbolo: '≣', acao: 'camadas' },
        { id: 'camadaAtual', comando: null, simbolo: '', pequeno: true, widget: 'camadaAtual' },
        { id: 'tornar_atual', comando: 'tornar_atual', simbolo: '✔', pequeno: true },
        { id: 'igualar_camada', comando: 'igualar_camada', simbolo: '⇉', pequeno: true },
      ] },
      { id: 'bloco', botoes: [
        { id: 'inserir', comando: 'inserir', simbolo: '⧉' },
        { id: 'criar_bloco', comando: 'criar_bloco', simbolo: '⊞', pequeno: true },
        { id: 'editar_bloco', comando: null, simbolo: '✎', pequeno: true },
      ] },
      { id: 'propriedades', lancador: 'propriedades', botoes: [
        { id: 'igualar_propriedades', comando: 'igualar_propriedades', simbolo: '🖌' },
        { id: 'propCor', comando: null, simbolo: '', pequeno: true, widget: 'propCor' },
        { id: 'propEspessura', comando: null, simbolo: '', pequeno: true, widget: 'propEspessura' },
        { id: 'propTipo', comando: null, simbolo: '', pequeno: true, widget: 'propTipo' },
      ] },
      { id: 'grupos', botoes: [
        { id: 'agrupar', comando: 'agrupar', simbolo: '▣' },
      ] },
      { id: 'utilitarios', botoes: [
        { id: 'medir', comando: 'medir', simbolo: '📏' },
        { id: 'contar', comando: 'contar', simbolo: '#' },
      ] },
      { id: 'area_transferencia', botoes: [
        { id: 'colar', comando: 'colar', simbolo: '📋' },
        { id: 'copiar_area', comando: 'copiar_area', simbolo: '⎘', pequeno: true },
        { id: 'recortar', comando: 'recortar', simbolo: '✂', pequeno: true },
      ] },
    ],
  },
  { id: 'inserir', paineis: [
    { id: 'folha', botoes: [
      { id: 'folha_inserir', comando: null, simbolo: '▭', acao: 'folha_inserir' },
      { id: 'folha_escala', comando: null, simbolo: '⤡', acao: 'folha_escala' },
      { id: 'folha_carimbo', comando: null, simbolo: '✎', acao: 'folha_carimbo' },
    ] },
  ] },
  { id: 'anotar', paineis: [] },
  { id: 'parametrico', paineis: [] },
  { id: 'exibir', paineis: [] },
  { id: 'gerenciar', paineis: [] },
  { id: 'saida', paineis: [] },
  { id: 'colaborar', paineis: [] },
];
