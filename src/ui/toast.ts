/** Um aviso rápido no pé da janela (erro ao abrir/salvar). Some sozinho. */

import { Z } from './z.ts';

export const TOAST_CSS = `
.toast { position: fixed; left: 50%; bottom: 60px; transform: translateX(-50%); z-index: ${Z.dica}; max-width: 80vw; padding: 9px 16px; background: #5a2323; border: 1px solid #c25a5a; border-radius: 4px; color: #fff; box-shadow: 0 4px 16px rgba(0,0,0,.5); }
`;

const DURACAO_MS = 7000;

/** Mostra `mensagem` por alguns segundos. */
export function avisar(mensagem: string): void {
  const t = document.createElement('div');
  t.className = 'toast';
  t.setAttribute('role', 'alert');
  t.textContent = mensagem;
  document.body.append(t);
  setTimeout(() => t.remove(), DURACAO_MS);
}
