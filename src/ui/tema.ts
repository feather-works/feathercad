/**
 * O tema: **automático** (segue o sistema), **claro** ou **escuro** — o mesmo do
 * FeatherSheet. A escolha fica em `office.theme`, a chave da **suíte** (não do
 * app), para os aplicativos acompanharem uns aos outros. O tema é o atributo
 * `data-theme` do `<html>`; todo componente só usa as variáveis de cor de
 * `base.ts`, nunca uma cor solta. Vale também para as janelas à parte (Monitor):
 * elas ouvem o evento `storage` e trocam junto.
 */

export type ModoDoTema = 'auto' | 'light' | 'dark';

/** A ordem em que o botão gira. */
export const ORDEM_DOS_TEMAS: ModoDoTema[] = ['auto', 'light', 'dark'];

export const CHAVE_DO_TEMA = 'office.theme';

const CONSULTA_ESCURO = '(prefers-color-scheme: dark)';

/** O modo guardado (`auto` se nada, ou se o armazenamento falhar). */
export function modoSalvo(): ModoDoTema {
  try {
    const v = localStorage.getItem(CHAVE_DO_TEMA);
    return ORDEM_DOS_TEMAS.find((m) => m === v) ?? 'auto';
  } catch {
    return 'auto';
  }
}

/** O próximo modo na ordem (auto → claro → escuro → auto). */
export function proximoModo(atual: ModoDoTema): ModoDoTema {
  return ORDEM_DOS_TEMAS[(ORDEM_DOS_TEMAS.indexOf(atual) + 1) % ORDEM_DOS_TEMAS.length]!;
}

/** O tema é escuro? O automático pergunta ao sistema. */
export function ehEscuro(modo: ModoDoTema, sistemaEscuro: boolean): boolean {
  return modo === 'auto' ? sistemaEscuro : modo === 'dark';
}

type Ouvinte = () => void;
const ouvintes: Ouvinte[] = [];

/** Avisa `f` sempre que o tema visível mudar (quem desenha em canvas precisa reler as cores). */
export function aoMudarTema(f: Ouvinte): void {
  ouvintes.push(f);
}

const sistemaEscuro = (): boolean => window.matchMedia?.(CONSULTA_ESCURO).matches ?? true;

/** O tema visível agora. */
export function temaVisivel(): 'light' | 'dark' {
  return document.documentElement.dataset['theme'] === 'light' ? 'light' : 'dark';
}

/** Aplica o modo ao `<html>` e avisa quem desenha. */
export function aplicarModo(modo: ModoDoTema): void {
  const antes = document.documentElement.dataset['theme'];
  const depois = ehEscuro(modo, sistemaEscuro()) ? 'dark' : 'light';
  document.documentElement.dataset['theme'] = depois;
  if (antes !== depois) for (const f of ouvintes) f();
}

/** Escolhe e guarda o modo. */
export function definirModo(modo: ModoDoTema): void {
  try {
    localStorage.setItem(CHAVE_DO_TEMA, modo);
  } catch {
    // sem armazenamento: vale até fechar a janela
  }
  aplicarModo(modo);
}

/**
 * Liga o tema na página: aplica o guardado, segue o sistema no modo automático e
 * acompanha a troca feita em outra janela do mesmo app. Chamar uma vez por página.
 */
export function iniciarTema(): void {
  aplicarModo(modoSalvo());
  window.matchMedia?.(CONSULTA_ESCURO).addEventListener('change', () => aplicarModo(modoSalvo()));
  window.addEventListener('storage', (e) => {
    if (e.key === CHAVE_DO_TEMA) aplicarModo(modoSalvo());
  });
}

/** Lê uma variável de cor do tema (`--cursor`…) para desenhar em canvas. */
export function corDoTema(variavel: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(variavel).trim();
}
