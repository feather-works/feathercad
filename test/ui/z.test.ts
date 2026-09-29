import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Z } from '../../src/ui/z.ts';

test('z-index: a escala é estritamente crescente, na ordem da pilha', () => {
  const valores = Object.values(Z);
  assert.deepEqual(valores, [...valores].sort((a, b) => a - b));
  assert.equal(new Set(valores).size, valores.length);
});
