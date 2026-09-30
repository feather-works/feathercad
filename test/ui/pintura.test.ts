import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pintarObjetos, pintarTracos, type ObjetosDoQuadro } from '../../src/ui/pintura.ts';

/** Um contexto de mentira que guarda com que fonte, alinhamento e linha de base cada texto foi desenhado. */
function contextoFalso(): { ctx: CanvasRenderingContext2D; textos: { conteudo: string; font: string; align: string; base: string }[]; chamadas: string[] } {
  const textos: { conteudo: string; font: string; align: string; base: string }[] = [];
  const chamadas: string[] = [];
  const estado = { font: '10px sans-serif', textAlign: 'start', textBaseline: 'alphabetic' };
  const ctx = new Proxy(estado as unknown as Record<string, unknown>, {
    get(alvo, nome: string) {
      if (nome in alvo) return alvo[nome];
      if (nome === 'getTransform') return () => ({ tag: 'base' });
      if (nome === 'fillText') {
        return (conteudo: string) => textos.push({ conteudo, font: estado.font, align: estado.textAlign, base: estado.textBaseline });
      }
      // `save`/`restore` devolveriam a fonte ao padrão no meio da lista: o teste os proíbe
      return (..._args: unknown[]) => {
        chamadas.push(nome);
        if (nome === 'save' || nome === 'restore') throw new Error(`o desenho de texto não pode usar ${nome}`);
      };
    },
    set(alvo, nome: string, valor) {
      alvo[nome] = valor;
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
  return { ctx, textos, chamadas };
}

const vazio = { xs: new Float32Array(0), cores: new Uint32Array(0), espessuras: new Float32Array(0), preenchimentos: [] };

test('texto: todos os textos saem com a própria fonte, alinhamento e linha de base (regressão do cache de fonte)', () => {
  const { ctx, textos } = contextoFalso();
  const q: ObjetosDoQuadro = {
    ...vazio,
    textos: [
      { x: 10, y: 10, altura: 20, giro: 0, h: 'left', base: 'alphabetic', cor: 0xffffff, conteudo: 'a' },
      { x: 10, y: 40, altura: 20, giro: 0, h: 'left', base: 'alphabetic', cor: 0xffffff, conteudo: 'b' },
      { x: 10, y: 70, altura: 20, giro: 0, h: 'center', base: 'middle', cor: 0xffffff, conteudo: 'c' },
      { x: 10, y: 90, altura: 35, giro: 0.5, h: 'center', base: 'middle', cor: 0xff0000, conteudo: 'd' },
    ],
  };
  pintarObjetos(ctx, q);
  assert.deepEqual(
    textos.map((t) => [t.conteudo, t.font, t.align, t.base]),
    [
      ['a', '20px system-ui, sans-serif', 'left', 'alphabetic'],
      ['b', '20px system-ui, sans-serif', 'left', 'alphabetic'],
      ['c', '20px system-ui, sans-serif', 'center', 'middle'],
      ['d', '35px system-ui, sans-serif', 'center', 'middle'],
    ],
  );
});

test('pintarTracos não desenha texto (o texto tem camada própria)', () => {
  const { ctx, textos } = contextoFalso();
  const q: ObjetosDoQuadro = { ...vazio, textos: [{ x: 10, y: 10, altura: 20, giro: 0, h: 'left', base: 'alphabetic', cor: 0xffffff, conteudo: 'texto' }] };
  pintarTracos(ctx, q);
  assert.deepEqual(textos, []);
});
