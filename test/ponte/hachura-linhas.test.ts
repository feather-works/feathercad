import { test } from 'node:test';
import assert from 'node:assert/strict';
import { motorNovo } from '../motor-wasm.ts';

test('hachura: um quadrado feito de quatro linhas soltas também é preenchido (não só retângulo/polilinha)', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.digitar('L');
  for (const p of ['0;0', '100;0', '100;100', '0;100', '0;0']) m.digitar(p);
  m.digitar('');
  assert.equal(m.estado().ferramenta, 'nenhuma');
  // Sem entidade fechada nenhuma no desenho: só quatro linhas.
  m.executar('hachura');
  const antes = m.quadro(vista, null);
  assert.ok(antes.tipo === 'novo' && antes.preenchimentos.length === 0);
  m.clique(vista, 450, 250); // dentro do quadrado (centro = 50;50 → pixel 450,250)
  m.digitar('');
  const depois = m.quadro(vista, null);
  assert.ok(depois.tipo === 'novo' && (depois.preenchimentos.length > 0 || depois.cores.length > antes.cores.length), 'a hachura apareceu');
});

test('hachura: contorno de linhas com uma abertura não é preenchido', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.digitar('L');
  for (const p of ['0;0', '100;0', '100;100', '0;100', '0;10']) m.digitar(p); // falta fechar de 10 até 0
  m.digitar('');
  m.executar('hachura');
  const antes = m.estado().historico.length;
  m.clique(vista, 450, 250);
  const e = m.estado();
  assert.ok(e.historico.length > antes, 'o motor avisa que não há contorno');
  const q = m.quadro(vista, null);
  assert.ok(q.tipo === 'novo' && q.preenchimentos.length === 0);
});
