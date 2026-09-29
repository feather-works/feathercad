import { test } from 'node:test';
import assert from 'node:assert/strict';
import { lerMetrica } from '../../src/monitor/metricas-texto.ts';
import { ehAtalhoDoMonitor } from '../../src/ui/monitor-atalhos.ts';
import { teclaDoEvento } from '../../src/ui/teclado.ts';
import { acaoDoMotorParaF12 } from './util.ts';

const ev = (key: string, m: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean }> = {}) => ({ key, ctrlKey: false, shiftKey: false, altKey: false, ...m });

test('monitor: lê linha de métrica do motor e da interface', () => {
  assert.deepEqual(lerMetrica('[motor] cursor_mover — 31 ms (espera 0 ms) · 120 B -> 12 B'), {
    origem: 'motor',
    texto: 'cursor_mover — 31 ms (espera 0 ms) · 120 B -> 12 B',
    ms: 31,
  });
  assert.equal(lerMetrica('[quadro] desenho: 22 ms (1 comando(s), motor 3 ms)')?.origem, 'quadro');
  assert.equal(lerMetrica('uma linha qualquer'), null);
});

test('monitor: F12, Alt+C, Ctrl+Shift+J e Ctrl+Shift+M abrem o Monitor; o resto não', () => {
  assert.ok(ehAtalhoDoMonitor(ev('F12')));
  assert.ok(ehAtalhoDoMonitor(ev('c', { altKey: true })));
  assert.ok(ehAtalhoDoMonitor(ev('J', { ctrlKey: true, shiftKey: true })));
  assert.ok(ehAtalhoDoMonitor(ev('M', { ctrlKey: true, shiftKey: true })));
  assert.equal(ehAtalhoDoMonitor(ev('F8')), false);
  assert.equal(ehAtalhoDoMonitor(ev('m', { ctrlKey: true })), false);
});

test('monitor: o F12 não é comando do motor (o AutoCAD usa para entrada dinâmica; aqui é do Monitor)', () => {
  assert.equal(teclaDoEvento({ ...ev('F12'), metaKey: false }), 'F12');
  assert.equal(acaoDoMotorParaF12(), false);
});
