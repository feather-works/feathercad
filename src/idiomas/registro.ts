/**
 * As línguas do app. Cada uma são **dois** arquivos nesta pasta:
 *
 * - `<código>.json` — a configuração regional (bloco `idioma`) e todos os
 *   textos da interface (só o TypeScript lê);
 * - `comandos.<código>.json` — os nomes e apelidos de comando que o usuário
 *   digita (`LINHA`, `L`), que vão para o **Rust** interpretar a linha de
 *   comando (`idioma_definir`). Arquivo separado: o motor recebe só isso, sem
 *   o texto da interface inteiro. Os apelidos do AutoCAD estão em
 *   `docs/ATALHOS.md`; um comando só entra aqui quando o motor o tem.
 *
 * **Língua nova**: copie `pt-BR.json` e `comandos.pt-BR.json`, traduza, e
 * registre aqui (imports e uma linha em cada tabela). O que faltar num texto
 * aparece em português (`REFERENCIA`); `test/idiomas/idiomas.test.ts` acusa
 * chave faltando ou digitada errado.
 *
 * Variável no texto: `{nome}`, trocada por `formatar`.
 */

import ptBR from './pt-BR.json' with { type: 'json' };
import enUS from './en-US.json' with { type: 'json' };
import comandosPtBR from './comandos.pt-BR.json' with { type: 'json' };
import comandosEnUS from './comandos.en-US.json' with { type: 'json' };

/** A forma dos textos: a da língua de referência. */
export type Textos = typeof ptBR;

/** Uma língua pode estar incompleta: o que faltar vem da referência. */
export type TextosParciais<T = Textos> = {
  [K in keyof T]?: T[K] extends string ? string : TextosParciais<T[K]>;
};

/** A língua de referência: completa por definição, e reserva das outras. */
export const REFERENCIA = 'pt-BR';

/** Todas as línguas registradas, pelo código. */
export const IDIOMAS: Record<string, TextosParciais> = {
  'pt-BR': ptBR,
  'en-US': enUS,
};

/** Os nomes que o usuário digita: `comandos` (`LINHA`, `L`) e `opcoes` das ferramentas (`Fechar`). */
export interface TabelaDeComandos {
  comandos: Record<string, string[]>;
  opcoes: Record<string, string[]>;
}

/** Nome ou apelido de comando por língua. */
export const COMANDOS: Record<string, TabelaDeComandos> = {
  'pt-BR': comandosPtBR,
  'en-US': comandosEnUS,
};

/** As línguas que o app entrega: o teste exige que estejam completas. */
export const COMPLETAS = ['pt-BR', 'en-US'];
