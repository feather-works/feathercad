import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RIBBON } from '../../src/ui/ribbon-dados.ts';
import { COMPLETAS, IDIOMAS } from '../../src/idiomas/registro.ts';
import { motorNovo } from '../motor-wasm.ts';

const botoes = RIBBON.flatMap((a) => a.paineis.flatMap((p) => p.botoes)).filter((b) => !b.widget);

test('ribbon: nenhum botão morto — todo comando de botão existe no motor', () => {
  const conhecidos = new Set(motorNovo().comandos());
  for (const b of botoes.filter((x) => x.comando !== null)) assert.ok(conhecidos.has(b.comando!), `o botão ${b.id} chama \`${b.comando}\`, que o motor não tem`);
});

test('ribbon: toda aba, painel e botão tem texto em todas as línguas', () => {
  for (const codigo of COMPLETAS) {
    const r = (IDIOMAS[codigo] as { ribbon: Record<string, Record<string, unknown>> }).ribbon;
    for (const a of RIBBON) {
      assert.ok(r['abas']?.[a.id], `${codigo}: aba ${a.id}`);
      for (const p of a.paineis) assert.ok(r['paineis']?.[p.id], `${codigo}: painel ${p.id}`);
    }
    for (const b of botoes) assert.ok(r['botoes']?.[b.id], `${codigo}: botão ${b.id}`);
  }
});

test('ribbon: id de aba, painel e botão não se repetem', () => {
  const ids = (xs: { id: string }[]) => xs.map((x) => x.id);
  const unicos = (xs: string[]) => new Set(xs).size === xs.length;
  assert.ok(unicos(ids(RIBBON)));
  assert.ok(unicos(RIBBON.flatMap((a) => ids(a.paineis))));
  assert.ok(unicos(ids(botoes)));
});
