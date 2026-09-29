import { test } from 'node:test';
import assert from 'node:assert/strict';
import { teclaDoEvento } from '../../src/ui/teclado.ts';

const ev = (key: string, m: Partial<{ ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean }> = {}) => ({
  key,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  ...m,
});

test('teclado: traduz o evento no texto do atalho (só traduz, não decide)', () => {
  assert.equal(teclaDoEvento(ev('g', { ctrlKey: true })), 'Ctrl+G');
  assert.equal(teclaDoEvento(ev('H', { ctrlKey: true, shiftKey: true })), 'Ctrl+Shift+H');
  assert.equal(teclaDoEvento(ev('F8')), 'F8');
  assert.equal(teclaDoEvento(ev('Escape')), 'Esc');
  assert.equal(teclaDoEvento(ev('[', { ctrlKey: true })), 'Ctrl+[');
});

test('teclado: letra solta vai para a linha de comando, e modificador sozinho não é atalho', () => {
  assert.equal(teclaDoEvento(ev('l')), null);
  assert.equal(teclaDoEvento(ev('Control', { ctrlKey: true })), null);
});
