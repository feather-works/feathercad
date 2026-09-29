import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { CHAVE_DO_TEMA, ehEscuro, ORDEM_DOS_TEMAS, proximoModo } from '../../src/ui/tema.ts';
import { COMPLETAS, IDIOMAS } from '../../src/idiomas/registro.ts';
import { motorNovo } from '../motor-wasm.ts';

test('tema: gira automático → claro → escuro → automático, e o automático segue o sistema', () => {
  assert.deepEqual(ORDEM_DOS_TEMAS, ['auto', 'light', 'dark']);
  assert.equal(proximoModo('auto'), 'light');
  assert.equal(proximoModo('light'), 'dark');
  assert.equal(proximoModo('dark'), 'auto');
  assert.equal(ehEscuro('auto', true), true);
  assert.equal(ehEscuro('auto', false), false);
  assert.equal(ehEscuro('light', true), false);
  assert.equal(ehEscuro('dark', false), true);
});

test('tema: a chave é a da suíte (o FeatherSheet usa a mesma), não a do app', () => {
  assert.equal(CHAVE_DO_TEMA, 'office.theme');
});

test('tema: todo modo tem texto em todas as línguas', () => {
  for (const codigo of COMPLETAS) {
    const t = (IDIOMAS[codigo] as { tema: { dica: string; modos: Record<string, string> } }).tema;
    assert.match(t.dica, /\{modo\}/);
    for (const m of ORDEM_DOS_TEMAS) assert.ok(t.modos[m], `${codigo}: ${m}`);
  }
});

test('tema: nenhum componente escreve cor solta — só as variáveis de base.ts (exceções: console copiado e toast de erro)', () => {
  const pasta = new URL('../../src/ui/', import.meta.url);
  const livres = new Set(['base.ts', 'console-depuracao.ts', 'toast.ts']);
  for (const arq of readdirSync(pasta).filter((f) => f.endsWith('.ts') && !livres.has(f))) {
    const cores = readFileSync(new URL(arq, pasta), 'utf8').match(/#[0-9a-fA-F]{3,8}\b|rgba?\(/g);
    assert.equal(cores, null, `${arq} tem cor solta: ${cores?.join(', ')}`);
  }
});

test('tema: claro e escuro definem exatamente as mesmas variáveis', () => {
  const css = readFileSync(new URL('../../src/ui/base.ts', import.meta.url), 'utf8');
  const bloco = (inicio: string): Set<string> => {
    const a = css.indexOf(inicio);
    const fim = css.indexOf('\n}', a);
    return new Set([...css.slice(a, fim).matchAll(/(--[a-z-]+):/g)].map((m) => m[1]!));
  };
  const claro = bloco(':root {');
  const escuro = bloco("[data-theme='dark'] {");
  const soClaro = [...claro].filter((v) => !escuro.has(v) && v !== '--fonte');
  assert.deepEqual(soClaro, [], 'variável definida só no claro: o escuro herdaria a cor errada');
  assert.deepEqual([...escuro].filter((v) => !claro.has(v)), []);
});

test('tema: o branco do CAD vira preto no fundo claro (conta do motor, pela ponte)', () => {
  const m = motorNovo();
  m.linhaCriar({ x: 0, y: 0 }, { x: 10, y: 0 });
  const v = m.vistaInicial(800, 600);
  const cor = (claro: boolean): number => {
    const q = m.quadro(v, null, claro);
    return q.tipo === 'novo' ? q.cores[0]! : -1;
  };
  assert.equal(cor(false), 0xffffff);
  assert.equal(cor(true), 0x000000);
});
