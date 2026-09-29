/**
 * O gerenciador de propriedades de camadas (a janela do AutoCAD): tabela com Status, Nome, Ativada, Congelar,
 * Bloquear, Plotar, Cor, Tipo de linha, Espessura e Transparência; árvore de filtros; busca; contagem.
 * **Só desenha**: filtrar, buscar, ordenar, validar nome e aplicar cada mudança é do motor (`camadas`,
 * `camada_acao`); aqui ficam o que se seleciona na tabela, a coluna clicada e a caixa de edição aberta.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import { ESPESSURAS_MM, TIPOS_DE_LINHA, TIPO_PARA_DXF, type ConsultaDeCamadas, type LinhaDeCamada, type OrdemDeCamadas, type PedidoDeCamada, type TabelaDeCamadas } from '../ponte/tipos.ts';
import { texto, type Textos } from '../idiomas/index.ts';
import { iconeCadeado, iconeImpressora, iconeLampada, iconeSolOuNeve, iconeStatus } from './camadas-icones.ts';
import { Z } from './z.ts';

export const CAMADAS_PAINEL_CSS = `
.camadas-painel { position: absolute; left: 16px; top: 16px; width: min(900px, calc(100% - 32px)); height: min(430px, calc(100% - 32px)); min-width: 520px; min-height: 240px; z-index: ${Z.paleta}; display: flex; flex-direction: column; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 4px; box-shadow: var(--sombra); resize: both; overflow: hidden; }
.camadas-painel .titulo { display: flex; align-items: center; gap: 8px; padding: 5px 8px; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); cursor: move; user-select: none; font-weight: 600; }
.camadas-painel .titulo .espaco { flex: 1; }
.camadas-painel button.fechar { background: none; border: 0; cursor: pointer; font-size: 14px; color: var(--texto-suave); }
.camadas-painel .barra { display: flex; align-items: center; gap: 4px; padding: 5px 8px; border-bottom: 1px solid var(--borda); }
.camadas-painel .barra .atual { margin-left: 10px; color: var(--texto-suave); }
.camadas-painel .barra .espaco { flex: 1; }
.camadas-painel .barra button { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 8px; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 3px; cursor: pointer; color: var(--texto); }
.camadas-painel .barra button:hover:not(:disabled) { border-color: var(--destaque); }
.camadas-painel .barra button:disabled { opacity: 0.45; cursor: default; }
.camadas-painel .barra input { width: 190px; height: 24px; padding: 0 6px; background: var(--fundo-area); color: var(--texto); border: 1px solid var(--borda); border-radius: 3px; }
.camadas-painel .corpo-painel { flex: 1; min-height: 0; display: flex; }
.camadas-painel .filtros { width: 150px; flex: none; padding: 6px; border-right: 1px solid var(--borda); overflow: auto; }
.camadas-painel .filtros .rotulo { color: var(--texto-suave); margin-bottom: 4px; }
.camadas-painel .filtros button { display: block; width: 100%; text-align: left; padding: 3px 8px; background: none; border: 0; border-radius: 3px; cursor: pointer; color: var(--texto); }
.camadas-painel .filtros button.ativo { background: var(--destaque-suave); }
.camadas-painel .tabela-rolagem { flex: 1; min-width: 0; overflow: auto; background: var(--fundo-area); }
.camadas-painel table { border-collapse: collapse; width: 100%; }
.camadas-painel th { position: sticky; top: 0; z-index: 1; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); border-right: 1px solid var(--borda); padding: 3px 6px; text-align: left; font-weight: 500; cursor: pointer; user-select: none; white-space: nowrap; }
.camadas-painel th .seta { color: var(--texto-suave); margin-left: 3px; }
.camadas-painel td { padding: 2px 6px; border-bottom: 1px solid var(--borda); white-space: nowrap; }
.camadas-painel td.centro { text-align: center; }
.camadas-painel tr.selecionada td { background: var(--destaque-suave); }
.camadas-painel tr.atual td.nome { font-weight: 600; }
.camadas-painel td button.chave { background: none; border: 0; padding: 0 2px; cursor: pointer; line-height: 0; }
.camadas-painel label.cor { display: inline-flex; align-items: center; gap: 6px; cursor: pointer; }
.camadas-painel label.cor .amostra { width: 14px; height: 14px; border: 1px solid var(--borda); border-radius: 2px; }
.camadas-painel label.cor input { position: absolute; width: 0; height: 0; opacity: 0; pointer-events: none; }
.camadas-painel td select, .camadas-painel td input.numero, .camadas-painel td input.nome-edicao { background: var(--fundo-area); color: var(--texto); border: 1px solid var(--borda); border-radius: 2px; height: 20px; font: inherit; }
.camadas-painel td input.numero { width: 48px; }
.camadas-painel td input.nome-edicao { width: 100%; min-width: 110px; }
.camadas-painel .rodape-painel { display: flex; gap: 12px; padding: 4px 8px; border-top: 1px solid var(--borda); color: var(--texto-suave); }
.camadas-painel .rodape-painel .erro { color: var(--texto); font-weight: 600; }
`;

export interface PainelDeCamadas {
  el: HTMLElement;
  abrir(): void;
  fechar(): void;
  alternar(): void;
  /** O motor mudou (por fora do painel: desfazer, comando…): atualiza a tabela se o painel está aberto. */
  atualizar(e: Estado): void;
}

type Coluna = { ordem: OrdemDeCamadas; rotulo: string };

/** Cria o gerenciador. `aoMudar` recebe o estado novo do motor depois de cada ação (para o quadro refazer). */
export function criarPainelDeCamadas(t: Textos, motor: Motor, aoMudar: (e: Estado) => void): PainelDeCamadas {
  const el = document.createElement('div');
  el.className = 'camadas-painel';
  el.hidden = true;

  let consulta: ConsultaDeCamadas = { filtro: 'todas', busca: '', ordem: 'nome', crescente: true };
  let tabela: TabelaDeCamadas | null = null;
  let selecionadas = new Set<number>();
  let ancora: number | null = null;
  let ultimaVersao = -1;

  const c = t.camadas;
  const colunas: Coluna[] = [
    { ordem: 'status', rotulo: c.colunas.status },
    { ordem: 'nome', rotulo: c.colunas.nome },
    { ordem: 'ligada', rotulo: c.colunas.ligada },
    { ordem: 'congelada', rotulo: c.colunas.congelada },
    { ordem: 'bloqueada', rotulo: c.colunas.bloqueada },
    { ordem: 'plotar', rotulo: c.colunas.plotar },
    { ordem: 'cor', rotulo: c.colunas.cor },
    { ordem: 'tipo_linha', rotulo: c.colunas.tipoLinha },
    { ordem: 'espessura', rotulo: c.colunas.espessura },
    { ordem: 'transparencia', rotulo: c.colunas.transparencia },
  ];

  // ---- esqueleto
  const titulo = document.createElement('div');
  titulo.className = 'titulo';
  const nomeTitulo = document.createElement('span');
  nomeTitulo.textContent = c.titulo;
  const espacoTitulo = document.createElement('span');
  espacoTitulo.className = 'espaco';
  const botaoFechar = document.createElement('button');
  botaoFechar.className = 'fechar';
  botaoFechar.textContent = '✕';
  botaoFechar.title = c.fechar;
  titulo.append(nomeTitulo, espacoTitulo, botaoFechar);

  const barra = document.createElement('div');
  barra.className = 'barra';
  const botao = (rotulo: string, dica: string, acao: () => void): HTMLButtonElement => {
    const b = document.createElement('button');
    b.textContent = rotulo;
    b.title = dica;
    b.addEventListener('click', acao);
    return b;
  };
  const botaoNova = botao('＋', c.nova, () => agir({ acao: 'nova', indices: alvoUnico() }));
  const botaoExcluir = botao('✕', c.excluir, () => agir({ acao: 'excluir', indices: [...selecionadas] }));
  const botaoAtual = botao('✔', c.tornarAtual, () => agir({ acao: 'atual', indices: alvoUnico() }));
  const rotuloAtual = document.createElement('span');
  rotuloAtual.className = 'atual';
  const espacoBarra = document.createElement('span');
  espacoBarra.className = 'espaco';
  const busca = document.createElement('input');
  busca.type = 'search';
  busca.placeholder = c.buscar;
  busca.spellcheck = false;
  barra.append(botaoNova, botaoExcluir, botaoAtual, rotuloAtual, espacoBarra, busca);

  const corpo = document.createElement('div');
  corpo.className = 'corpo-painel';
  const filtros = document.createElement('div');
  filtros.className = 'filtros';
  const rotuloFiltros = document.createElement('div');
  rotuloFiltros.className = 'rotulo';
  rotuloFiltros.textContent = c.filtros;
  const botaoTodas = document.createElement('button');
  botaoTodas.textContent = c.filtroTodas;
  botaoTodas.addEventListener('click', () => filtrar('todas'));
  const botaoUsadas = document.createElement('button');
  botaoUsadas.textContent = c.filtroUsadas;
  botaoUsadas.addEventListener('click', () => filtrar('usadas'));
  filtros.append(rotuloFiltros, botaoTodas, botaoUsadas);
  const rolagem = document.createElement('div');
  rolagem.className = 'tabela-rolagem';
  const tab = document.createElement('table');
  const cabeca = document.createElement('thead');
  const linhaCabeca = document.createElement('tr');
  for (const col of colunas) {
    const th = document.createElement('th');
    th.addEventListener('click', () => ordenarPor(col.ordem));
    linhaCabeca.append(th);
  }
  cabeca.append(linhaCabeca);
  const tbody = document.createElement('tbody');
  tab.append(cabeca, tbody);
  rolagem.append(tab);
  corpo.append(filtros, rolagem);

  const rodape = document.createElement('div');
  rodape.className = 'rodape-painel';
  const contagem = document.createElement('span');
  const mensagem = document.createElement('span');
  mensagem.className = 'erro';
  rodape.append(contagem, mensagem);
  el.append(titulo, barra, corpo, rodape);

  // ---- ações (o motor decide; aqui só se manda)
  function agir(pedido: Omit<PedidoDeCamada, 'consulta'>): void {
    const r = motor.camadaAcao({ ...pedido, consulta });
    ultimaVersao = r.estado.versao;
    mensagem.textContent = r.erro ? texto(r.erro) : '';
    // O que não existe mais some da seleção.
    aoMudar(r.estado);
    mostrar(r.tabela);
  }

  /** Só uma linha (a selecionada, ou a atual) serve de modelo para "nova" e alvo de "tornar atual". */
  function alvoUnico(): number[] {
    const escolhida = [...selecionadas][0] ?? tabela?.linhas.find((l) => l.atual)?.indice;
    return escolhida === undefined ? [] : [escolhida];
  }

  /** Aplica uma propriedade à linha — ou a todas as selecionadas, se ela é uma delas. */
  function propriedade(l: LinhaDeCamada, nome: NonNullable<PedidoDeCamada['propriedade']>, valor: boolean | string | number | null): void {
    const indices = selecionadas.has(l.indice) ? [...selecionadas] : [l.indice];
    agir({ acao: 'propriedade', indices, propriedade: nome, valor });
  }

  function carregar(): void {
    mensagem.textContent = '';
    mostrar(motor.camadas(consulta));
  }

  function filtrar(f: ConsultaDeCamadas['filtro']): void {
    consulta = { ...consulta, filtro: f };
    carregar();
  }

  function ordenarPor(ordem: OrdemDeCamadas): void {
    consulta = { ...consulta, ordem, crescente: consulta.ordem === ordem ? !consulta.crescente : true };
    carregar();
  }

  busca.addEventListener('input', () => {
    consulta = { ...consulta, busca: busca.value };
    carregar();
  });

  // ---- desenho da tabela
  function chave(l: LinhaDeCamada, dica: string, icone: HTMLElement, nome: 'ligada' | 'congelada' | 'bloqueada' | 'plotar', novo: boolean): HTMLTableCellElement {
    const td = document.createElement('td');
    td.className = 'centro';
    const b = document.createElement('button');
    b.className = 'chave';
    b.title = dica;
    b.append(icone);
    b.addEventListener('click', (ev) => {
      ev.stopPropagation();
      propriedade(l, nome, novo);
    });
    td.append(b);
    return td;
  }

  function celulaNome(l: LinhaDeCamada): HTMLTableCellElement {
    const td = document.createElement('td');
    td.className = 'nome';
    td.title = c.dicaNome;
    td.textContent = l.nome;
    td.addEventListener('dblclick', () => {
      if (l.nome === '0') return;
      const entrada = document.createElement('input');
      entrada.className = 'nome-edicao';
      entrada.value = l.nome;
      td.replaceChildren(entrada);
      entrada.focus();
      entrada.select();
      let feito = false;
      const terminar = (aplicar: boolean): void => {
        if (feito) return;
        feito = true;
        if (aplicar && entrada.value !== l.nome) agir({ acao: 'renomear', indices: [l.indice], nome: entrada.value });
        else td.textContent = l.nome;
      };
      entrada.addEventListener('keydown', (ev) => {
        if (ev.key === 'Enter') terminar(true);
        else if (ev.key === 'Escape') terminar(false);
        ev.stopPropagation();
      });
      entrada.addEventListener('blur', () => terminar(true));
    });
    return td;
  }

  function celulaCor(l: LinhaDeCamada): HTMLTableCellElement {
    const td = document.createElement('td');
    const rotulo = document.createElement('label');
    rotulo.className = 'cor';
    rotulo.title = c.dicaCor;
    const amostra = document.createElement('span');
    amostra.className = 'amostra';
    amostra.style.background = l.cor;
    const seletor = document.createElement('input');
    seletor.type = 'color';
    seletor.value = l.cor;
    seletor.addEventListener('change', () => propriedade(l, 'cor', seletor.value));
    const nome = document.createElement('span');
    nome.textContent = l.cor;
    rotulo.append(amostra, nome, seletor);
    td.append(rotulo);
    return td;
  }

  function celulaTipo(l: LinhaDeCamada): HTMLTableCellElement {
    const td = document.createElement('td');
    const sel = document.createElement('select');
    for (const dxf of TIPOS_DE_LINHA) {
      const o = document.createElement('option');
      o.value = dxf;
      o.textContent = c.tipos[dxf];
      sel.append(o);
    }
    sel.value = TIPO_PARA_DXF[l.tipo_linha];
    sel.addEventListener('change', () => propriedade(l, 'tipo_linha', sel.value));
    td.append(sel);
    return td;
  }

  function celulaEspessura(l: LinhaDeCamada): HTMLTableCellElement {
    const td = document.createElement('td');
    const sel = document.createElement('select');
    const numero = new Intl.NumberFormat(document.documentElement.lang || undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    const padrao = document.createElement('option');
    padrao.value = '';
    padrao.textContent = c.padrao;
    sel.append(padrao);
    for (const mm of ESPESSURAS_MM) {
      const o = document.createElement('option');
      o.value = String(mm);
      o.textContent = `${numero.format(mm)} mm`;
      sel.append(o);
    }
    sel.value = l.espessura === null ? '' : String(ESPESSURAS_MM.find((mm) => Math.abs(mm - (l.espessura ?? 0)) < 1e-3) ?? '');
    sel.addEventListener('change', () => propriedade(l, 'espessura', sel.value === '' ? null : Number(sel.value)));
    td.append(sel);
    return td;
  }

  function celulaTransparencia(l: LinhaDeCamada): HTMLTableCellElement {
    const td = document.createElement('td');
    const entrada = document.createElement('input');
    entrada.className = 'numero';
    entrada.type = 'number';
    entrada.min = '0';
    entrada.max = '90';
    entrada.step = '10';
    entrada.value = String(l.transparencia);
    entrada.addEventListener('change', () => propriedade(l, 'transparencia', Number(entrada.value) || 0));
    entrada.addEventListener('keydown', (ev) => ev.stopPropagation());
    td.append(entrada);
    return td;
  }

  function linha(l: LinhaDeCamada): HTMLTableRowElement {
    const tr = document.createElement('tr');
    tr.classList.toggle('selecionada', selecionadas.has(l.indice));
    tr.classList.toggle('atual', l.atual);
    const status = document.createElement('td');
    status.className = 'centro';
    status.append(iconeStatus(l.atual, l.usada, l.atual ? c.dicaAtual : l.usada ? c.dicaUsada : c.dicaVazia));
    tr.append(
      status,
      celulaNome(l),
      chave(l, c.ligada, iconeLampada(l.ligada), 'ligada', !l.ligada),
      chave(l, c.congelada, iconeSolOuNeve(l.congelada), 'congelada', !l.congelada),
      chave(l, c.bloqueada, iconeCadeado(l.bloqueada), 'bloqueada', !l.bloqueada),
      chave(l, c.plotar, iconeImpressora(l.plotar), 'plotar', !l.plotar),
      celulaCor(l),
      celulaTipo(l),
      celulaEspessura(l),
      celulaTransparencia(l),
    );
    tr.addEventListener('click', (ev) => escolher(l.indice, ev.ctrlKey || ev.metaKey, ev.shiftKey));
    tr.addEventListener('dblclick', (ev) => {
      // Duplo clique no status torna atual (como no AutoCAD); no nome renomeia (tratado na célula).
      if ((ev.target as HTMLElement).closest('td')?.classList.contains('centro') && !(ev.target as HTMLElement).closest('button')) agir({ acao: 'atual', indices: [l.indice] });
    });
    return tr;
  }

  function escolher(indice: number, alternar: boolean, faixa: boolean): void {
    const ordem = tabela?.linhas.map((l) => l.indice) ?? [];
    if (faixa && ancora !== null && ordem.includes(ancora)) {
      const [a, b] = [ordem.indexOf(ancora), ordem.indexOf(indice)].sort((x, y) => x - y) as [number, number];
      selecionadas = new Set(ordem.slice(a, b + 1));
    } else if (alternar) {
      if (!selecionadas.delete(indice)) selecionadas.add(indice);
      ancora = indice;
    } else {
      selecionadas = new Set([indice]);
      ancora = indice;
    }
    mostrar(tabela);
  }

  function mostrar(nova: TabelaDeCamadas | null): void {
    if (!nova) return;
    tabela = nova;
    const existentes = new Set(nova.linhas.map((l) => l.indice));
    // A seleção acompanha só as camadas que ainda aparecem (excluir, filtrar).
    selecionadas = new Set([...selecionadas].filter((i) => existentes.has(i)));
    rotuloAtual.textContent = texto('camadas.atual', { nome: nova.atual });
    ([...linhaCabeca.children] as HTMLElement[]).forEach((th, i) => {
      const col = colunas[i]!;
      th.textContent = col.rotulo;
      if (consulta.ordem === col.ordem) {
        const seta = document.createElement('span');
        seta.className = 'seta';
        seta.textContent = consulta.crescente ? '▲' : '▼';
        th.append(seta);
      }
    });
    botaoTodas.classList.toggle('ativo', consulta.filtro === 'todas');
    botaoUsadas.classList.toggle('ativo', consulta.filtro === 'usadas');
    botaoExcluir.disabled = selecionadas.size === 0;
    tbody.replaceChildren(...nova.linhas.map(linha));
    const filtrado = consulta.filtro !== 'todas' || consulta.busca.trim() !== '';
    contagem.textContent = texto(filtrado ? 'camadas.contagemFiltrada' : 'camadas.contagem', { exibidas: String(nova.exibidas), totais: String(nova.totais) });
  }

  // ---- arrastar pela barra de título
  titulo.addEventListener('mousedown', (ev) => {
    if ((ev.target as HTMLElement).closest('button')) return;
    const pai = el.offsetParent as HTMLElement | null;
    const [x0, y0, esq, topo] = [ev.clientX, ev.clientY, el.offsetLeft, el.offsetTop];
    const mover = (m: MouseEvent): void => {
      const maxX = (pai?.clientWidth ?? 0) - 60;
      const maxY = (pai?.clientHeight ?? 0) - 30;
      el.style.left = `${Math.max(0, Math.min(maxX, esq + m.clientX - x0))}px`;
      el.style.top = `${Math.max(0, Math.min(maxY, topo + m.clientY - y0))}px`;
    };
    const soltar = (): void => {
      window.removeEventListener('mousemove', mover);
      window.removeEventListener('mouseup', soltar);
    };
    window.addEventListener('mousemove', mover);
    window.addEventListener('mouseup', soltar);
    ev.preventDefault();
  });
  botaoFechar.addEventListener('click', () => (el.hidden = true));
  // Delete na tabela exclui as camadas selecionadas (as que puderem).
  el.addEventListener('keydown', (ev) => {
    if (ev.key === 'Delete' && !(ev.target instanceof HTMLInputElement) && selecionadas.size > 0) agir({ acao: 'excluir', indices: [...selecionadas] });
  });
  el.tabIndex = -1;
  // O mouse sobre o painel não chega ao desenho.
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick', 'contextmenu'] as const) el.addEventListener(ev, (e) => e.stopPropagation());

  return {
    el,
    abrir() {
      el.hidden = false;
      ultimaVersao = -1;
      carregar();
    },
    fechar() {
      el.hidden = true;
    },
    alternar() {
      if (el.hidden) this.abrir();
      else this.fechar();
    },
    atualizar(e) {
      if (el.hidden || e.versao === ultimaVersao) return;
      ultimaVersao = e.versao;
      carregar();
    },
  };
}
