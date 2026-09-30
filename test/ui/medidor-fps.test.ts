import { test } from 'node:test';
import assert from 'node:assert/strict';
import { criarConta } from '../../src/ui/medidor-fps.ts';

test('FPS: quadros a cada 16,67 ms dão 60 fps; o pior quadro e as etapas saem da mesma janela', () => {
  const c = criarConta();
  for (let i = 0; i <= 30; i++) c.quadro(i * (1000 / 60));
  c.quadro(30 * (1000 / 60) + 40); // um quadro lento
  c.tempo('motor', 4, 500);
  c.tempo('motor', 6, 510);
  c.tempo('pintura', 2, 510);
  const l = c.ler(540);
  assert.ok(l.fps > 45 && l.fps < 60, `${l.fps}`);
  assert.ok(Math.abs(l.pior - 40) < 1e-6);
  assert.deepEqual(l.etapas, { motor: 5, cursor: 0, pintura: 2, total: 0 });
});

test('FPS: o que sobra do intervalo depois do nosso trabalho é da webview', () => {
  const c = criarConta();
  for (let i = 0; i <= 10; i++) {
    c.quadro(i * 30); // 30 ms por quadro
    c.tempo('total', 2, i * 30);
  }
  const l = c.ler(300);
  assert.ok(Math.abs(l.webview - 28) < 1e-6 && Math.abs(l.etapas.total - 2) < 1e-6, `${l.webview}`);
});

test('FPS: mouse parado (pausa longa) não vira quadro lento, e o que saiu da janela some', () => {
  const c = criarConta();
  c.quadro(0);
  c.quadro(16);
  c.quadro(2000); // pausa de quase 2 s: fora da conta
  const l = c.ler(2000);
  assert.equal(l.pior, 0, 'só sobrou um quadro na janela');
  assert.equal(l.fps, 0);
  const vazio = criarConta().ler(0);
  assert.deepEqual(vazio, { fps: 0, pior: 0, etapas: { motor: 0, cursor: 0, pintura: 0, total: 0 }, webview: 0 });
});
