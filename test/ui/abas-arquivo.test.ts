import { test } from 'node:test';
import assert from 'node:assert/strict';
import { criarAbasArquivo, type ItemAba } from '../../src/ui/abas-arquivo.ts';
import { textos } from '../../src/idiomas/index.ts';

// Mock simples de DOM para o ambiente Node sem navegador
function mockDom() {
  class MockElement {
    className = '';
    textContent = '';
    title = '';
    listeners: Record<string, ((e: Event) => void)[]> = {};
    children: MockElement[] = [];

    addEventListener(type: string, fn: (e: Event) => void) {
      if (!this.listeners[type]) this.listeners[type] = [];
      this.listeners[type].push(fn);
    }
    append(...nodes: (MockElement | string)[]) {
      for (const n of nodes) {
        if (typeof n === 'string') {
          const textNode = new MockElement();
          textNode.textContent = n;
          this.children.push(textNode);
        } else {
          this.children.push(n);
        }
      }
    }
    replaceChildren(...nodes: (MockElement | string)[]) {
      this.children = [];
      this.append(...nodes);
    }
    classList = {
      contains: (cls: string) => this.className.split(' ').includes(cls),
    };
    querySelectorAll(selector: string): MockElement[] {
      const results: MockElement[] = [];
      const match = (el: MockElement) => {
        if (selector.startsWith('.') && el.className.split(' ').includes(selector.slice(1))) {
          results.push(el);
        }
        for (const child of el.children) match(child);
      };
      for (const child of this.children) match(child);
      return results;
    }
    querySelector(selector: string): MockElement | null {
      return this.querySelectorAll(selector)[0] ?? null;
    }
    click() {
      const ev = { stopPropagation: () => {} } as Event;
      for (const fn of this.listeners['click'] ?? []) fn(ev);
    }
  }

  (globalThis as unknown as Record<string, unknown>).document = {
    createElement: (tag: string) => {
      const el = new MockElement();
      if (tag === 'button') el.className = 'fechar';
      return el;
    },
  };
}

test('abas-arquivo: renderiza inicio, abas de desenho e botao de novo', () => {
  mockDom();
  const t = textos();
  const selecoes: string[] = [];
  const fechadas: string[] = [];
  let novoClicado = false;
  let inicioClicado = false;

  const abas = criarAbasArquivo(t, {
    aoInicio: () => { inicioClicado = true; },
    aoSelecionarAba: (id) => selecoes.push(id),
    aoFecharAba: (id) => fechadas.push(id),
    aoNovo: () => { novoClicado = true; },
  });

  const lista: ItemAba[] = [
    { id: 'tab1', nome: 'Planta.dxf', caminho: '/tmp/Planta.dxf', sujo: false },
    { id: 'tab2', nome: 'Eletrica.dxf', sujo: true },
  ];

  abas.atualizar(lista, 'tab1');

  const arquivos = (abas.el as unknown as { querySelectorAll: (s: string) => unknown[] }).querySelectorAll('.arquivo');
  assert.equal(arquivos.length, 3); // inicio + 2 desenhos

  // Testar selecao
  (arquivos[2] as { click: () => void }).click();
  assert.deepEqual(selecoes, ['tab2']);

  // Testar fechar
  const btnFechar = (arquivos[1] as { querySelector: (s: string) => { click: () => void } }).querySelector('.fechar');
  btnFechar.click();
  assert.deepEqual(fechadas, ['tab1']);

  // Testar botao novo
  const btnMais = (abas.el as unknown as { querySelector: (s: string) => { click: () => void } }).querySelector('.mais');
  btnMais.click();
  assert.equal(novoClicado, true);

  // Testar clique na aba inicio
  (arquivos[0] as { click: () => void }).click();
  assert.equal(inicioClicado, true);
});
