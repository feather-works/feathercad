/**
 * As duas perguntas das folhas: "Inserir folha" (papel, escala e carimbo) e "Escala da folha" (trocar a escala de uma
 * folha ou pedir ao motor que ajuste ao desenho). Só juntam a escolha num pedido; quem valida, calcula a escala que
 * cabe e posiciona a folha é o Rust. Teclas: Enter confirma, Esc cancela.
 */

import type { Textos } from '../idiomas/index.ts';
import type { CamposFolha, FolhasDoMotor, PedidoFolhaNova } from '../ponte/tipos.ts';
import { listaSuspensa } from './caixa-exportar.ts';

const AUTOMATICA = 'auto';

const DIMENSOES_PADRAO_PAPEL: Record<string, string> = {
  A4: '297 × 210 mm',
  A3: '420 × 297 mm',
  A2: '594 × 420 mm',
  A1: '841 × 594 mm',
  A0: '1189 × 841 mm',
  '2A0': '1682 × 1189 mm',
  '4A0': '2378 × 1682 mm',
};

function formatarTamanhoPapel(papel: string, largura_mm?: number, altura_mm?: number): string {
  if (largura_mm && altura_mm) {
    return `${papel} (${largura_mm} × ${altura_mm} mm)`;
  }
  const dim = DIMENSOES_PADRAO_PAPEL[papel];
  return dim ? `${papel} (${dim})` : papel;
}

export const CAIXA_FOLHA_CSS = `
.caixa-folha-dialog {
  width: 450px;
  max-width: 94vw;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  background: var(--fundo-painel);
  border: 1px solid var(--borda);
  border-radius: 8px;
  box-shadow: var(--sombra);
  overflow: hidden;
  font-family: var(--fonte);
}

.caixa-folha-topo {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--fundo-faixa);
  border-bottom: 1px solid var(--borda);
  user-select: none;
}

.caixa-folha-topo h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--texto);
  letter-spacing: 0.2px;
}

.caixa-folha-fechar {
  background: none;
  border: 0;
  color: var(--texto-suave);
  font-size: 14px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  transition: color 0.15s, background 0.15s;
}

.caixa-folha-fechar:hover {
  color: var(--texto);
  background: var(--fundo-faixa);
}

.caixa-folha-resumo {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--texto-suave);
  background: var(--fundo-faixa);
  border-bottom: 1px solid var(--borda);
  line-height: 1.4;
}

.caixa-folha-corpo {
  padding: 0;
  overflow-y: auto;
  max-height: calc(90vh - 120px);
}

.caixa-folha-secao {
  margin-bottom: 4px;
}

.caixa-folha-secao-cabeca {
  padding: 4px 12px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--texto);
  background: var(--fundo-faixa);
  border-top: 1px solid var(--borda);
  border-bottom: 1px solid var(--borda);
}

.caixa-folha-secao:first-child .caixa-folha-secao-cabeca {
  border-top: 0;
}

.caixa-folha-linha {
  display: grid;
  grid-template-columns: 140px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  padding: 4px 12px;
  border-bottom: 1px solid var(--borda);
  transition: background 0.1s ease;
}

.caixa-folha-linha:hover {
  background: var(--fundo-faixa);
}

.caixa-folha-rotulo {
  font-size: 12px;
  color: var(--texto-suave);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.caixa-folha-valor-info {
  font-size: 12px;
  color: var(--texto);
  font-weight: 500;
  padding: 3px 0;
  user-select: text;
}

.caixa-folha-input,
.caixa-folha-seletor {
  width: 100%;
  height: 26px;
  box-sizing: border-box;
  padding: 0 8px;
  background: var(--fundo-area);
  color: var(--texto);
  border: 1px solid var(--borda);
  border-radius: 3px;
  font-size: 12px;
  font-family: inherit;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.caixa-folha-input:focus,
.caixa-folha-seletor:focus {
  border-color: var(--destaque);
  outline: none;
  box-shadow: 0 0 0 1px var(--destaque);
}

.caixa-folha-divisoria-label {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 12px;
  color: var(--texto);
  cursor: pointer;
}

.caixa-folha-rodape {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px 12px;
  background: var(--fundo-faixa);
  border-top: 1px solid var(--borda);
}

.caixa-folha-rodape button {
  padding: 5px 16px;
  font-size: 12px;
  font-weight: 500;
  background: var(--fundo-area);
  color: var(--texto);
  border: 1px solid var(--borda);
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.caixa-folha-rodape button:hover {
  border-color: var(--destaque);
}

.caixa-folha-rodape button.principal {
  background: var(--destaque);
  color: var(--fundo-area);
  border-color: var(--destaque);
  font-weight: 600;
}

.caixa-folha-rodape button.principal:hover {
  filter: brightness(1.1);
}
`;

function caixaBase(titulo: string, corpo: string): { fundo: HTMLElement; caixa: HTMLElement; fecharBtn: HTMLButtonElement } {
  const fundo = document.createElement('div');
  fundo.className = 'exportar-fundo';
  const caixa = document.createElement('div');
  caixa.className = 'caixa-folha-dialog';
  caixa.setAttribute('role', 'dialog');
  caixa.setAttribute('aria-modal', 'true');
  caixa.setAttribute('aria-label', titulo);

  const topo = document.createElement('div');
  topo.className = 'caixa-folha-topo';

  const h2 = document.createElement('h2');
  h2.textContent = titulo;

  const fecharBtn = document.createElement('button');
  fecharBtn.type = 'button';
  fecharBtn.className = 'caixa-folha-fechar';
  fecharBtn.textContent = '✕';
  fecharBtn.setAttribute('aria-label', 'Fechar');

  topo.append(h2, fecharBtn);

  const resumo = document.createElement('div');
  resumo.className = 'caixa-folha-resumo';
  resumo.textContent = corpo;

  caixa.append(topo, resumo);
  fundo.append(caixa);
  return { fundo, caixa, fecharBtn };
}

function botao(texto: string, principal: boolean, aoClicar: () => void): HTMLButtonElement {
  const b = Object.assign(document.createElement('button'), { type: 'button', textContent: texto, className: principal ? 'principal' : '' });
  b.addEventListener('click', aoClicar);
  return b;
}

function campoInfoLinha(rotulo: string, valor: string): HTMLElement {
  const linha = document.createElement('div');
  linha.className = 'caixa-folha-linha';

  const label = document.createElement('span');
  label.className = 'caixa-folha-rotulo';
  label.textContent = rotulo;
  label.title = rotulo;

  const valSpan = document.createElement('span');
  valSpan.className = 'caixa-folha-valor-info';
  valSpan.textContent = valor;

  linha.append(label, valSpan);
  return linha;
}

function campoTextoLinha(rotulo: string, inicial: string, aoMudar: (v: string) => void): HTMLElement {
  const linha = document.createElement('div');
  linha.className = 'caixa-folha-linha';

  const label = document.createElement('span');
  label.className = 'caixa-folha-rotulo';
  label.textContent = rotulo;
  label.title = rotulo;

  const entrada = Object.assign(document.createElement('input'), {
    type: 'text',
    className: 'caixa-folha-input',
    maxLength: 80,
    value: inicial,
  });
  entrada.addEventListener('input', () => aoMudar(entrada.value));

  linha.append(label, entrada);
  return linha;
}

function campoElementoLinha(rotulo: string, elemento: HTMLElement): HTMLElement {
  const linha = document.createElement('div');
  linha.className = 'caixa-folha-linha';

  const label = document.createElement('span');
  label.className = 'caixa-folha-rotulo';
  label.textContent = rotulo;
  label.title = rotulo;

  if (elemento instanceof HTMLSelectElement) {
    elemento.className = 'caixa-folha-seletor';
  }

  linha.append(label, elemento);
  return linha;
}

/** Abre a caixa: o resultado sai por `fechar` (Esc = `null`; Enter chama `aoEnter`). */
function abrir<R>(fundo: HTMLElement, foco: HTMLElement, aoEnter: (fechar: (r: R | null) => void) => void, fecharBtn?: HTMLButtonElement): { resultado: Promise<R | null>; fechar: (r: R | null) => void } {
  let fechar: (r: R | null) => void = () => undefined;
  const resultado = new Promise<R | null>((resolve) => {
    fechar = (r) => {
      window.removeEventListener('keydown', aoTeclar, true);
      fundo.remove();
      resolve(r);
    };
    const aoTeclar = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') fechar(null);
      else if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement)) aoEnter(fechar);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    if (fecharBtn) {
      fecharBtn.addEventListener('click', () => fechar(null));
    }
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    foco.focus();
  });
  return { resultado, fechar: (r) => fechar(r) };
}

/** Os campos do carimbo em seções de lista de propriedades. */
function gradeDeCampos(t: Textos, campos: CamposFolha): HTMLElement {
  const c = t.folha.inserir;
  const container = document.createElement('div');
  container.className = 'caixa-folha-corpo';

  // Seção 1: Projeto & Responsabilidade
  const secaoProjeto = document.createElement('div');
  secaoProjeto.className = 'caixa-folha-secao';
  const cabecaProjeto = document.createElement('div');
  cabecaProjeto.className = 'caixa-folha-secao-cabeca';
  cabecaProjeto.textContent = 'Projeto & Responsabilidade';
  secaoProjeto.append(
    cabecaProjeto,
    campoTextoLinha(c.cliente, campos.cliente ?? '', (v) => (campos.cliente = v)),
    campoTextoLinha(c.fase, campos.fase ?? '', (v) => (campos.fase = v)),
    campoTextoLinha(c.disciplina, campos.disciplina ?? '', (v) => (campos.disciplina = v)),
    campoTextoLinha(c.responsavel ?? 'Responsável técnico / Empresa', campos.responsavel ?? '', (v) => (campos.responsavel = v)),
  );

  // Seção 2: Desenho & Revisões
  const secaoDesenho = document.createElement('div');
  secaoDesenho.className = 'caixa-folha-secao';
  const cabecaDesenho = document.createElement('div');
  cabecaDesenho.className = 'caixa-folha-secao-cabeca';
  cabecaDesenho.textContent = 'Desenho & Revisões';
  secaoDesenho.append(
    cabecaDesenho,
    campoTextoLinha(c.tituloDesenho, campos.titulo ?? '', (v) => (campos.titulo = v)),
    campoTextoLinha(c.subtitulo, campos.subtitulo ?? '', (v) => (campos.subtitulo = v)),
    campoTextoLinha(c.numero, campos.numero ?? '', (v) => (campos.numero = v)),
    campoTextoLinha(c.revisao, campos.revisao ?? '', (v) => (campos.revisao = v)),
    campoTextoLinha(c.revisaoDescricao ?? 'Descrição da revisão', campos.revisao_descricao ?? '', (v) => (campos.revisao_descricao = v)),
    campoTextoLinha(c.numeroDaFolha, campos.folha ?? '', (v) => (campos.folha = v)),
  );

  container.append(secaoProjeto, secaoDesenho);
  return container;
}

/** Edita o carimbo de uma folha: escolhe a folha (se há mais de uma), mostra os campos de hoje e devolve os novos; `null` se desistiu. */
export function perguntarCarimbo(t: Textos, dados: FolhasDoMotor, ler: (indice: number) => CamposFolha): Promise<{ indice: number; campos: CamposFolha } | null> {
  const c = t.folha.carimbo;
  const { fundo, caixa, fecharBtn } = caixaBase(c.titulo, dados.folhas.length === 0 ? t.folha.escala.semFolhas : c.corpo);
  let indice = 0;
  let campos = dados.folhas.length > 0 ? ler(0) : ({} as CamposFolha);
  const meio = document.createElement('div');
  meio.className = 'caixa-folha-corpo';
  const pe = document.createElement('div');
  pe.className = 'caixa-folha-rodape';
  let sair: (r: { indice: number; campos: CamposFolha } | null) => void = () => undefined;
  const refazer = (): void => {
    const folhaAtual = dados.folhas[indice];
    const selectFolha = listaSuspensa(dados.folhas.map((f) => String(f.indice)), (v) => {
      const f = dados.folhas[Number(v)]!;
      return `Folha ${f.indice + 1} - ${formatarTamanhoPapel(f.papel, f.largura_mm, f.altura_mm)} - ${f.escala_texto}`;
    }, String(indice), (v) => {
      indice = Number(v);
      campos = ler(indice);
      refazer();
    });

    const secaoInfo = document.createElement('div');
    secaoInfo.className = 'caixa-folha-secao';
    const cabecaInfo = document.createElement('div');
    cabecaInfo.className = 'caixa-folha-secao-cabeca';
    cabecaInfo.textContent = 'Informações da Folha';
    secaoInfo.append(cabecaInfo);

    if (dados.folhas.length > 1) {
      secaoInfo.append(campoElementoLinha(t.folha.escala.qual, selectFolha));
    }

    if (folhaAtual) {
      secaoInfo.append(
        campoInfoLinha('Tamanho da folha', formatarTamanhoPapel(folhaAtual.papel, folhaAtual.largura_mm, folhaAtual.altura_mm)),
        campoInfoLinha('Escala', folhaAtual.escala_texto),
      );
    }

    meio.replaceChildren(secaoInfo, gradeDeCampos(t, campos));
  };
  const cancelar = botao(c.cancelar, false, () => sair(null));
  if (dados.folhas.length > 0) {
    refazer();
    caixa.append(meio);
    pe.append(cancelar, botao(c.salvar, true, () => sair({ indice, campos })));
  } else {
    pe.append(cancelar);
  }
  caixa.append(pe);
  const aberta = abrir<{ indice: number; campos: CamposFolha }>(fundo, cancelar, (fechar) => dados.folhas.length > 0 && fechar({ indice, campos }), fecharBtn);
  sair = aberta.fechar;
  return aberta.resultado;
}

/** Pergunta papel, escala e carimbo de uma folha nova; `null` se desistiu. */
export function perguntarFolhaNova(t: Textos, dados: FolhasDoMotor, temSelecao: boolean = false): Promise<PedidoFolhaNova | null> {
  const c = t.folha.inserir;
  const { fundo, caixa, fecharBtn } = caixaBase(c.titulo, c.corpo);
  const campos: CamposFolha = { cliente: '', fase: '', disciplina: '', responsavel: '', titulo: '', subtitulo: '', numero: '', revisao: '', revisao_descricao: '', folha: '', divisoria: false };
  let papel = dados.papeis.includes('A1') ? 'A1' : (dados.papeis[0] ?? 'A1');
  let escala: string = AUTOMATICA;
  let cobrirSelecao = temSelecao;
  const textoDaEscala = new Map(dados.escalas.map(([valor, texto]) => [String(valor), texto]));

  const corpo = document.createElement('div');
  corpo.className = 'caixa-folha-corpo';

  // Seção Configuração
  const secaoConfig = document.createElement('div');
  secaoConfig.className = 'caixa-folha-secao';
  const cabecaConfig = document.createElement('div');
  cabecaConfig.className = 'caixa-folha-secao-cabeca';
  cabecaConfig.textContent = 'Configuração da Folha';

  const selectPapel = listaSuspensa(dados.papeis, (p) => formatarTamanhoPapel(p), papel, (v) => {
    papel = v;
    atualizarTamanho();
  });
  const selectEscala = listaSuspensa([AUTOMATICA, ...dados.escalas.map(([v]) => String(v))], (v) => (v === AUTOMATICA ? c.automatica : (textoDaEscala.get(v) ?? v)), escala, (v) => (escala = v));

  const linhaTamanho = campoInfoLinha('Tamanho da folha', formatarTamanhoPapel(papel));
  const atualizarTamanho = () => {
    const span = linhaTamanho.querySelector('.caixa-folha-valor-info');
    if (span) span.textContent = formatarTamanhoPapel(papel);
  };

  secaoConfig.append(
    cabecaConfig,
    campoElementoLinha(c.papel, selectPapel),
    linhaTamanho,
    campoElementoLinha(c.escala, selectEscala),
  );

  const grade = gradeDeCampos(t, campos);

  const divisoria = document.createElement('label');
  divisoria.className = 'caixa-folha-divisoria-label';
  const caixaDivisoria = Object.assign(document.createElement('input'), { type: 'checkbox' });
  caixaDivisoria.addEventListener('change', () => (campos.divisoria = caixaDivisoria.checked));
  divisoria.append(caixaDivisoria, c.divisoria);

  corpo.append(secaoConfig, grade, divisoria);

  if (temSelecao) {
    const labelSelecao = document.createElement('label');
    labelSelecao.className = 'caixa-folha-divisoria-label';
    const caixaSelecao = Object.assign(document.createElement('input'), { type: 'checkbox', checked: true });
    caixaSelecao.addEventListener('change', () => (cobrirSelecao = caixaSelecao.checked));
    labelSelecao.append(caixaSelecao, 'Cobrir / enquadrar apenas a seleção atual');
    corpo.append(labelSelecao);
  }

  caixa.append(corpo);

  const pedido = (): PedidoFolhaNova => ({
    papel,
    ...(escala === AUTOMATICA ? {} : { escala: Number(escala) }),
    campos,
    ...(cobrirSelecao ? { cobrir_selecao: true } : {}),
  });
  const pe = document.createElement('div');
  pe.className = 'caixa-folha-rodape';
  let sair: (r: PedidoFolhaNova | null) => void = () => undefined;
  const ok = botao(c.inserir, true, () => sair(pedido()));
  pe.append(botao(c.cancelar, false, () => sair(null)), ok);
  caixa.append(pe);
  const caixaAberta = abrir<PedidoFolhaNova>(fundo, ok, (fechar) => fechar(pedido()), fecharBtn);
  sair = caixaAberta.fechar;
  return caixaAberta.resultado;
}

/** O que a caixa "Escala da folha" devolve: uma escala (denominador) ou "ajustar ao desenho". */
export type EscolhaDeEscala = { indice: number; escala: number } | { indice: number; ajustar: true };

/** Pergunta qual folha e qual escala; `null` se fechou sem escolher. Os botões agem na hora, pela função `aplicar`. */
export function perguntarEscalaDaFolha(t: Textos, dados: FolhasDoMotor, aplicar: (escolha: EscolhaDeEscala) => FolhasDoMotor): Promise<void> {
  const c = t.folha.escala;
  const { fundo, caixa, fecharBtn } = caixaBase(c.titulo, dados.folhas.length === 0 ? c.semFolhas : c.corpo);
  let atual = dados;
  let indice = 0;
  let escala = String(dados.folhas[0]?.escala ?? dados.escalas[0]?.[0] ?? 100);
  const cx = document.createElement('div');
  cx.className = 'caixa-folha-corpo';

  const refazer = (): void => {
    const folhaAtual = atual.folhas[indice];
    const secao = document.createElement('div');
    secao.className = 'caixa-folha-secao';
    const cabeca = document.createElement('div');
    cabeca.className = 'caixa-folha-secao-cabeca';
    cabeca.textContent = 'Ajuste de Escala';

    const selectQual = listaSuspensa(atual.folhas.map((f) => String(f.indice)), (v) => {
      const f = atual.folhas[Number(v)]!;
      return `Folha ${f.indice + 1} - ${formatarTamanhoPapel(f.papel, f.largura_mm, f.altura_mm)} - ${f.escala_texto}`;
    }, String(indice), (v) => {
      indice = Number(v);
      escala = String(atual.folhas[indice]?.escala ?? escala);
      refazer();
    });

    const selectEscala = listaSuspensa(atual.escalas.map(([v]) => String(v)), (v) => atual.escalas.find(([x]) => String(x) === v)?.[1] ?? v, escala, (v) => (escala = v));

    secao.append(
      cabeca,
      campoElementoLinha(c.qual, selectQual),
    );

    if (folhaAtual) {
      secao.append(campoInfoLinha('Tamanho da folha', formatarTamanhoPapel(folhaAtual.papel, folhaAtual.largura_mm, folhaAtual.altura_mm)));
    }

    secao.append(campoElementoLinha(c.escala, selectEscala));
    cx.replaceChildren(secao);
  };
  const pe = document.createElement('div');
  pe.className = 'caixa-folha-rodape';
  const depois = (novos: FolhasDoMotor): void => {
    atual = novos;
    escala = String(atual.folhas[indice]?.escala ?? escala);
    refazer();
  };
  let sair: (r: null) => void = () => undefined;
  const fechar = botao(c.fechar, false, () => sair(null));
  if (dados.folhas.length > 0) {
    refazer();
    caixa.append(cx);
    pe.append(
      botao(c.ajustar, false, () => depois(aplicar({ indice, ajustar: true }))),
      botao(c.aplicar, true, () => depois(aplicar({ indice, escala: Number(escala) }))),
    );
  }
  pe.append(fechar);
  caixa.append(pe);
  const caixaAberta = abrir<never>(fundo, fechar, (f) => f(null), fecharBtn);
  sair = caixaAberta.fechar;
  return caixaAberta.resultado.then(() => undefined);
}
