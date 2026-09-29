import { test } from 'node:test';
import assert from 'node:assert/strict';
import { motorNovo } from '../motor-wasm.ts';

const campos = { cliente: '', fase: '', disciplina: '', titulo: 'TÚNEL', subtitulo: '', numero: '', revisao: '', folha: '', divisoria: false };

function desenhoDe500m() {
  const m = motorNovo();
  m.documentoNovo();
  m.unidadeDefinir('m');
  m.vistaInicial(800, 600);
  m.digitar('L');
  for (const p of ['0;0', '500;250']) m.digitar(p);
  m.digitar('');
  return m;
}

test('folha: sem escala pedida o motor escolhe a menor em que o desenho cabe, e a lista devolve o texto da escala', () => {
  const m = desenhoDe500m();
  const r = m.folhaInserir({ papel: 'A1', campos });
  assert.equal(r.folha.papel, 'A1');
  assert.equal(r.folha.escala, 750);
  assert.equal(r.folha.escala_texto, '1:750');
  const lista = m.folhaListar();
  assert.equal(lista.folhas.length, 1);
  assert.ok(lista.papeis.includes('4A0') && lista.escalas.some(([v, t]) => v === 100 && t === '1:100'));
});

test('folha: trocar a escala e ajustar ao desenho voltam pelo Ctrl+Z de uma vez só', () => {
  const m = desenhoDe500m();
  m.folhaInserir({ papel: 'A1', escala: 2000, campos });
  const b = m.folhaEscala(0, 1000);
  assert.equal(b.folha.escala_texto, '1:1000');
  const c = m.folhaAjustar(0);
  assert.equal(c.folha.escala, 750);
  m.desfazer();
  assert.equal(m.folhaListar().folhas[0]?.escala, 1000);
});

test('folha: papel e escala inválidos são recusados pelo Rust', () => {
  const m = desenhoDe500m();
  assert.throws(() => m.folhaInserir({ papel: 'B7', campos }), /papel/);
  assert.throws(() => m.folhaInserir({ papel: 'A1', escala: -5, campos }), /escala/);
  assert.throws(() => m.folhaEscala(3, 100), /folha 4/);
});

test('folha: os campos do carimbo se leem, se reescrevem e o Ctrl+Z volta', () => {
  const m = desenhoDe500m();
  m.folhaInserir({ papel: 'A1', escala: 1000, campos });
  assert.equal(m.folhaCampos(0).titulo, 'TÚNEL');
  assert.equal(m.folhaCampos(0).cliente, '');
  m.folhaCamposDefinir(0, { ...campos, cliente: 'Prefeitura', titulo: 'PLANTA', revisao: 'B' });
  const c = m.folhaCampos(0);
  assert.deepEqual([c.cliente, c.titulo, c.revisao], ['Prefeitura', 'PLANTA', 'B']);
  m.desfazer();
  assert.equal(m.folhaCampos(0).titulo, 'TÚNEL');
});
