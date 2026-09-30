/**
 * A escala de `z-index` — a única. Nenhum componente escreve número solto:
 * `test/ui/z.test.ts` trava a ordem (cada camada acima da anterior).
 */
export const Z = {
  /** O quadro do desenho. */
  desenho: 1,
  /** Os textos do quadro, numa camada própria (escalada por CSS durante o zoom, redesenhada nítida ao parar). */
  textos: 2,
  /** Cursor, guias e prévia, por cima do quadro. */
  sobreposicao: 3,
  /** As caixinhas da entrada dinâmica, junto ao cursor. */
  entradaDinamica: 4,
  /** Paletas acopláveis/flutuantes. */
  paleta: 10,
  /** A Ribbon (abre lista por cima da área). */
  ribbon: 20,
  /** Menus, listas suspensas e caixas de confirmação. */
  menu: 30,
  /** Dicas de botão. */
  dica: 40,
  /** O console de depuração (F12 no navegador), por cima de tudo. */
  console: 50,
} as const;
