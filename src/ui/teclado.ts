/**
 * Traduz o evento do teclado no texto de atalho que o motor entende
 * (`Ctrl+G`, `Ctrl+Shift+H`, `F8`, `Esc`). **Só traduz**: o que a tecla faz é do
 * Rust (`atalhos.rs`, a tabela do AutoCAD em `docs/ATALHOS.md`).
 */

/** O texto do atalho, ou `null` para tecla solta comum (essa vai para a linha de comando). */
export function teclaDoEvento(e: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>): string | null {
  const modificada = e.ctrlKey || e.metaKey || e.altKey;
  const especial = e.key === 'Escape' || e.key === 'Delete' || /^F\d{1,2}$/.test(e.key);
  if (!modificada && !especial) return null;
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;
  const nome = e.key === 'Escape' ? 'Esc' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const partes = [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', e.shiftKey ? 'Shift' : '', nome];
  return partes.filter(Boolean).join('+');
}
