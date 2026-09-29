/**
 * A página inicial (a aba "Início"), no estilo da tela de início do AutoCAD:
 * à esquerda o nome do programa, os botões Abrir e Novo e a navegação
 * (Recentes, Projetos); no meio, no modo Projetos, a lista de pastas; à direita
 * a tabela (ou miniaturas) dos desenhos.
 *
 * **Projeto aqui é uma pasta do disco** — sem conta, sem nuvem (regra 5 da suíte).
 * Quem lista pastas, guarda recentes e confere o que ainda existe é o Rust
 * (`feathercad-io::inicio`); esta peça só desenha o que ele devolve. A única
 * "conta" daqui é formatar a data pelo `Intl` do navegador, que conhece o fuso
 * e a língua (o Rust sem dependência não conhece o fuso).
 */

import { inicioEstado, inicioMiniatura, inicioPastaListar, inicioProjetoAdicionar, inicioProjetoRemover, inicioRecenteRemover, noApp } from '../ponte/nativo.ts';
import type { EstadoInicio, ItemDeArquivo } from '../ponte/nativo.ts';
import { codigoAtual, type Textos } from '../idiomas/index.ts';
import { guardarVisaoDoInicio, visaoDoInicio } from './preferencias.ts';

export const PAGINA_INICIAL_CSS = `
.inicio { display: grid; grid-template-columns: 220px 1fr; grid-template-rows: minmax(0, 1fr); min-height: 0; background: var(--fundo-area); }
.inicio .lateral { display: flex; flex-direction: column; gap: 10px; padding: 22px 0 0; background: var(--fundo); border-right: 1px solid var(--borda); min-height: 0; }
.inicio .produto { padding: 0 18px 10px; font-size: 14px; font-weight: 600; }
.inicio .acao { margin: 0 18px; padding: 8px 12px; text-align: left; background: none; border: 1px solid var(--borda); border-radius: 2px; cursor: pointer; }
.inicio .acao:hover { border-color: var(--destaque); }
.inicio nav { display: flex; flex-direction: column; margin-top: 18px; }
.inicio nav button { padding: 10px 18px; text-align: left; background: none; border: 0; border-left: 3px solid transparent; color: var(--texto-suave); cursor: pointer; }
.inicio nav button.ativo { background: var(--fundo-painel); border-left-color: var(--destaque); color: var(--texto); font-weight: 600; }
.inicio .projetos { width: 260px; padding: 22px 0 0; background: var(--fundo-faixa); border-right: 1px solid var(--borda); display: flex; flex-direction: column; min-height: 0; }
.inicio .projetos h2 { margin: 0; padding: 0 18px 12px; font-size: 18px; font-weight: 400; }
.inicio .projetos ul { list-style: none; margin: 0; padding: 0; overflow: auto; flex: 1; }
.inicio .projetos li { display: flex; align-items: center; gap: 4px; padding: 7px 8px 7px 18px; cursor: pointer; }
.inicio .projetos li span { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.inicio .projetos li:hover, .inicio .projetos li.ativo { background: var(--fundo-painel); }
.inicio .projetos li.ativo { border-left: 3px solid var(--destaque); padding-left: 15px; }
.inicio .projetos .mais { margin: 10px 18px; padding: 6px 10px; background: none; border: 1px dashed var(--borda); border-radius: 2px; cursor: pointer; color: var(--texto-suave); }
.inicio .minibotao { background: none; border: 0; cursor: pointer; color: var(--texto-suave); padding: 0 6px; visibility: hidden; }
.inicio li:hover .minibotao, .inicio tr:hover .minibotao { visibility: visible; }
.inicio .conteudo { display: flex; flex-direction: column; min-width: 0; min-height: 0; padding: 22px 28px 0; }
.inicio .cabeca { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
.inicio .cabeca h2 { margin: 0; font-size: 18px; font-weight: 400; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.inicio .visao button { padding: 4px 8px; background: none; border: 1px solid transparent; cursor: pointer; color: var(--texto-suave); font-size: 15px; }
.inicio .visao button.ativo { background: var(--destaque-suave); border-color: var(--destaque); color: var(--texto); }
.inicio .rolagem { flex: 1; min-height: 0; overflow: auto; }
.inicio table { width: 100%; border-collapse: collapse; }
.inicio th { position: sticky; top: 0; padding: 9px 10px; text-align: left; font-weight: 500; background: var(--fundo-painel); border-right: 1px solid var(--borda); }
.inicio td { padding: 8px 10px; border-bottom: 1px solid var(--borda); white-space: nowrap; max-width: 320px; overflow: hidden; text-overflow: ellipsis; }
.inicio tbody tr { cursor: pointer; }
.inicio tbody tr:hover, .inicio tbody tr:focus { background: var(--fundo-faixa); outline: none; }
.inicio td.nome { color: var(--texto); }
.inicio td.nome .icone { display: inline-block; width: 22px; color: var(--destaque); }
.inicio .cartoes { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 14px; }
.inicio .cartao { display: flex; flex-direction: column; gap: 4px; padding: 10px; background: var(--fundo-faixa); border: 1px solid var(--borda); border-radius: 3px; cursor: pointer; }
.inicio .cartao:hover, .inicio .cartao:focus { border-color: var(--destaque); outline: none; }
.inicio .cartao .miniatura { display: flex; align-items: center; justify-content: center; height: 100px; background: var(--fundo-area); color: var(--destaque); font-size: 38px; }
.inicio .cartao .miniatura img { width: 100%; height: 100%; object-fit: contain; }
.inicio .cartao .nome { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.inicio .cartao .data { color: var(--texto-suave); font-size: 11px; }
.inicio .vazio { padding: 30px 4px; color: var(--texto-suave); }
`;

type Secao = 'recentes' | 'projetos';

interface Contexto {
  t: Textos;
  /** Abrir o desenho (com o caminho, ou a caixa "Abrir" sem ele). */
  aoAbrir: (caminho?: string) => void;
  aoNovo: () => void;
}

export interface PaginaInicial {
  el: HTMLElement;
  /** Relê recentes e projetos do disco (chamar ao mostrar a aba). */
  atualizar(): Promise<void>;
}

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, classe = '', texto = ''): HTMLElementTagNameMap[K] => {
  const e = document.createElement(tag);
  if (classe) e.className = classe;
  if (texto) e.textContent = texto;
  return e;
};

/** Cria a página inicial. */
export function criarPaginaInicial(cx: Contexto): PaginaInicial {
  const { t } = cx;
  const raiz = el('section', 'inicio');
  const data = new Intl.DateTimeFormat(codigoAtual(), { dateStyle: 'medium', timeStyle: 'short' });
  const quando = (ms: number | null): string => (ms ? data.format(new Date(ms)) : '—');

  let secao: Secao = 'recentes';
  let visao = visaoDoInicio();
  let estado: EstadoInicio = { recentes: [], projetos: [] };
  let projeto: string | null = null;
  let itens: ItemDeArquivo[] = [];
  let erro = '';

  async function carregarProjeto(): Promise<void> {
    itens = [];
    erro = '';
    if (projeto) {
      try {
        itens = await inicioPastaListar(projeto);
      } catch (e) {
        erro = e instanceof Error ? e.message : String(e);
      }
    }
  }

  const escolherSecao = (s: Secao): void => {
    secao = s;
    desenhar();
  };
  const aplicar = async (novo: EstadoInicio): Promise<void> => {
    estado = novo;
    if (projeto && !estado.projetos.includes(projeto)) projeto = null;
    await carregarProjeto();
    desenhar();
  };

  function lateral(): HTMLElement {
    const l = el('div', 'lateral');
    const abrir = el('button', 'acao', t.inicio.abrir);
    abrir.addEventListener('click', () => cx.aoAbrir());
    const novo = el('button', 'acao', t.inicio.novo);
    novo.addEventListener('click', () => cx.aoNovo());
    const nav = el('nav');
    for (const s of ['recentes', 'projetos'] as const) {
      const b = el('button', s === secao ? 'ativo' : '', t.inicio.navegacao[s]);
      b.addEventListener('click', () => escolherSecao(s));
      nav.append(b);
    }
    l.append(el('div', 'produto', t.app.nome), abrir, novo, nav);
    return l;
  }

  function colunaDeProjetos(): HTMLElement {
    const c = el('div', 'projetos');
    const lista = el('ul');
    for (const pasta of estado.projetos) {
      const li = el('li', pasta === projeto ? 'ativo' : '');
      li.title = pasta;
      const nome = el('span', '', pasta.split(/[\\/]/).filter(Boolean).pop() ?? pasta);
      const tirar = el('button', 'minibotao', '✕');
      tirar.title = t.inicio.projetos.remover;
      tirar.addEventListener('click', (ev) => {
        ev.stopPropagation();
        void inicioProjetoRemover(pasta).then(aplicar);
      });
      li.append(nome, tirar);
      li.addEventListener('click', () => {
        projeto = pasta;
        void carregarProjeto().then(desenhar);
      });
      lista.append(li);
    }
    const mais = el('button', 'mais', t.inicio.projetos.adicionar);
    mais.addEventListener('click', () => void inicioProjetoAdicionar().then(aplicar));
    c.append(el('h2', '', t.inicio.projetos.titulo), lista, mais);
    return c;
  }

  const linhaDoItem = (i: ItemDeArquivo, extra?: { aoTirar: () => void; data: number | null }): HTMLTableRowElement => {
    const tr = el('tr');
    tr.tabIndex = 0;
    tr.title = i.caminho;
    const nome = el('td', 'nome');
    nome.append(el('span', 'icone', '╱'), document.createTextNode(i.nome));
    tr.append(nome, el('td', '', quando(extra ? extra.data : i.criado_ms)), el('td', '', t.inicio.tipos[i.tipo]));
    if (extra) {
      tr.append(el('td', '', i.pasta));
      const tirar = el('button', 'minibotao', '✕');
      tirar.title = t.inicio.tirarDaLista;
      tirar.addEventListener('click', (ev) => {
        ev.stopPropagation();
        extra.aoTirar();
      });
      const celula = el('td');
      celula.append(tirar);
      tr.append(celula);
    }
    const abrir = (): void => cx.aoAbrir(i.caminho);
    tr.addEventListener('dblclick', abrir);
    tr.addEventListener('keydown', (ev) => ev.key === 'Enter' && abrir());
    return tr;
  };

  function tabela(): HTMLElement {
    const recentes = secao === 'recentes';
    const tabela = el('table');
    const cab = el('tr');
    const colunas = recentes ? [t.inicio.colunas.nome, t.inicio.colunas.aberto, t.inicio.colunas.tipo, t.inicio.colunas.local, ''] : [t.inicio.colunas.nome, t.inicio.colunas.criadoEm, t.inicio.colunas.tipo];
    for (const c of colunas) cab.append(el('th', '', c));
    const corpo = el('tbody');
    if (recentes) {
      for (const r of estado.recentes) corpo.append(linhaDoItem(r, { data: r.aberto_em_ms, aoTirar: () => void inicioRecenteRemover(r.caminho).then(aplicar) }));
    } else {
      for (const i of itens) corpo.append(linhaDoItem(i));
    }
    tabela.append(el('thead'), corpo);
    tabela.tHead?.append(cab);
    return tabela;
  }

  // Cada desenho da grade pede a sua miniatura ao Rust, um de cada vez (a lista aparece na hora, as imagens vão chegando).
  // Uma grade nova invalida a fila da anterior.
  let geracaoDasMiniaturas = 0;
  async function carregarMiniaturas(fila: { caminho: string; caixa: HTMLElement }[]): Promise<void> {
    const minha = ++geracaoDasMiniaturas;
    if (!noApp()) return;
    for (const { caminho, caixa } of fila) {
      if (minha !== geracaoDasMiniaturas) return;
      try {
        const svg = await inicioMiniatura(caminho);
        if (minha !== geracaoDasMiniaturas || !svg) continue;
        const img = document.createElement('img');
        img.alt = '';
        img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
        caixa.replaceChildren(img);
      } catch (e) {
        console.warn(`[início] sem miniatura de ${caminho}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
  }

  function cartoes(lista: { item: ItemDeArquivo; data: number | null }[]): HTMLElement {
    const g = el('div', 'cartoes');
    const fila: { caminho: string; caixa: HTMLElement }[] = [];
    for (const { item, data: quandoMs } of lista) {
      const c = el('div', 'cartao');
      c.tabIndex = 0;
      c.title = item.caminho;
      const miniatura = el('div', 'miniatura', '╱');
      fila.push({ caminho: item.caminho, caixa: miniatura });
      c.append(miniatura, el('div', 'nome', item.nome), el('div', 'data', quando(quandoMs)));
      c.addEventListener('dblclick', () => cx.aoAbrir(item.caminho));
      c.addEventListener('keydown', (ev) => ev.key === 'Enter' && cx.aoAbrir(item.caminho));
      g.append(c);
    }
    void carregarMiniaturas(fila);
    return g;
  }

  function conteudo(): HTMLElement {
    const c = el('div', 'conteudo');
    const cabeca = el('div', 'cabeca');
    const titulo = secao === 'recentes' ? t.inicio.recentes.titulo : projeto ? (projeto.split(/[\\/]/).filter(Boolean).pop() ?? projeto) : t.inicio.projetos.titulo;
    const alternador = el('div', 'visao');
    for (const [v, simbolo, dica] of [['lista', '☰', t.inicio.visao.lista], ['grade', '▦', t.inicio.visao.grade]] as const) {
      const b = el('button', v === visao ? 'ativo' : '', simbolo);
      b.title = dica;
      b.addEventListener('click', () => {
        visao = v;
        guardarVisaoDoInicio(v);
        desenhar();
      });
      alternador.append(b);
    }
    cabeca.append(el('h2', '', titulo), alternador);
    const rolagem = el('div', 'rolagem');
    const lista = secao === 'recentes' ? estado.recentes.map((r) => ({ item: r as ItemDeArquivo, data: r.aberto_em_ms })) : itens.map((i) => ({ item: i, data: i.criado_ms }));
    const vazio = secao === 'recentes' ? (noApp() ? t.inicio.recentes.vazio : t.inicio.somenteApp) : erro || (projeto ? t.inicio.projetos.semDesenhos : estado.projetos.length ? t.inicio.projetos.escolha : t.inicio.projetos.vazio);
    if (lista.length === 0) rolagem.append(el('div', 'vazio', vazio));
    else rolagem.append(visao === 'lista' ? tabela() : cartoes(lista));
    c.append(cabeca, rolagem);
    return c;
  }

  function desenhar(): void {
    raiz.style.gridTemplateColumns = secao === 'projetos' ? '220px 260px 1fr' : '220px 1fr';
    raiz.replaceChildren(lateral(), ...(secao === 'projetos' ? [colunaDeProjetos()] : []), conteudo());
  }

  desenhar();
  return {
    el: raiz,
    async atualizar() {
      if (noApp()) {
        try {
          await aplicar(await inicioEstado());
          return;
        } catch (e) {
          erro = e instanceof Error ? e.message : String(e);
        }
      }
      desenhar();
    },
  };
}
