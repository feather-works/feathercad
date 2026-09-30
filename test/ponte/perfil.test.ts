import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LENTO_MS, etapa, ligarPerfil, quadroDePerfil } from '../../src/ponte/perfil.ts';

/** Espera ocupada: o perfil mede tempo de parede, então o teste precisa gastar tempo de verdade. */
function gastar(ms: number): void {
  const fim = performance.now() + ms;
  while (performance.now() < fim) {
    // ocupado
  }
}

/** Captura o que o perfil imprime no console durante `f`. */
function capturar(f: () => void): string[] {
  const linhas: string[] = [];
  const original = console.log;
  console.log = (...args: unknown[]) => {
    linhas.push(args.join(' '));
  };
  try {
    f();
  } finally {
    console.log = original;
  }
  return linhas;
}

test('perfil desligado: etapa e quadro só devolvem o resultado, sem imprimir nada', () => {
  ligarPerfil(false);
  const linhas = capturar(() => {
    assert.equal(etapa('a', () => 7), 7);
    assert.equal(quadroDePerfil('zoom', () => { gastar(LENTO_MS + 5); return 'ok'; }), 'ok');
  });
  assert.deepEqual(linhas, []);
});

test('perfil ligado: um quadro lento sai na hora, com as etapas em ordem do maior para o menor e o resto', () => {
  ligarPerfil(true);
  try {
    const linhas = capturar(() => {
      quadroDePerfil('zoom', () => {
        etapa('pintar_fundo', () => gastar(LENTO_MS));
        etapa('quadro', () => {
          gastar(3);
          etapa('quadro/canal', () => gastar(1));
        });
        gastar(2); // vai para o "resto"
      });
    });
    assert.equal(linhas.length, 1, linhas.join('\n'));
    const l = linhas[0]!;
    assert.match(l, /^\[perfil\] quadro lento \(zoom\): /);
    const etapas = l.split('→ ')[1] ?? '';
    assert.ok(etapas.indexOf('pintar_fundo') < etapas.indexOf('quadro '), 'a etapa maior vem primeiro');
    assert.match(l, /quadro\/canal/);
    assert.match(l, /resto/);
  } finally {
    ligarPerfil(false);
  }
});

test('perfil ligado: quadro rápido não imprime sozinho; etapa fora de quadro não quebra', () => {
  ligarPerfil(true);
  try {
    const linhas = capturar(() => {
      assert.equal(etapa('solta', () => 3), 3);
      quadroDePerfil('mouse', () => etapa('cursor', () => gastar(1)));
    });
    assert.deepEqual(linhas, []);
  } finally {
    ligarPerfil(false);
  }
});
