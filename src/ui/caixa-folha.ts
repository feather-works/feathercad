/**
 * As duas perguntas das folhas: "Inserir folha" (papel, escala e carimbo) e "Escala da folha" (trocar a escala de uma
 * folha ou pedir ao motor que ajuste ao desenho). Só juntam a escolha num pedido; quem valida, calcula a escala que
 * cabe e posiciona a folha é o Rust. Teclas: Enter confirma, Esc cancela.
 */

import type { Textos } from '../idiomas/index.ts';
import type { CamposFolha, FolhasDoMotor, PedidoFolhaNova } from '../ponte/tipos.ts';
import { campo, listaSuspensa } from './caixa-exportar.ts';

const AUTOMATICA = 'auto';

function caixaBase(titulo: string, corpo: string): { fundo: HTMLElement; caixa: HTMLElement } {
  const fundo = document.createElement('div');
  fundo.className = 'exportar-fundo';
  const caixa = document.createElement('div');
  caixa.className = 'exportar';
  caixa.setAttribute('role', 'dialog');
  caixa.setAttribute('aria-modal', 'true');
  caixa.setAttribute('aria-label', titulo);
  caixa.append(Object.assign(document.createElement('h2'), { textContent: titulo }), Object.assign(document.createElement('p'), { textContent: corpo }));
  fundo.append(caixa);
  return { fundo, caixa };
}

function botao(texto: string, principal: boolean, aoClicar: () => void): HTMLButtonElement {
  const b = Object.assign(document.createElement('button'), { type: 'button', textContent: texto, className: principal ? 'principal' : '' });
  b.addEventListener('click', aoClicar);
  return b;
}

function campoTexto(titulo: string, inicial: string, aoMudar: (v: string) => void): HTMLFieldSetElement {
  const entrada = Object.assign(document.createElement('input'), { type: 'text', maxLength: 80, value: inicial });
  entrada.addEventListener('input', () => aoMudar(entrada.value));
  return campo(titulo, entrada);
}

/** Abre a caixa: o resultado sai por `fechar` (Esc = `null`; Enter chama `aoEnter`). */
function abrir<R>(fundo: HTMLElement, foco: HTMLElement, aoEnter: (fechar: (r: R | null) => void) => void): { resultado: Promise<R | null>; fechar: (r: R | null) => void } {
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
    document.body.append(fundo);
    window.addEventListener('keydown', aoTeclar, true);
    foco.focus();
  });
  return { resultado, fechar: (r) => fechar(r) };
}

/** Os campos do carimbo em duas colunas; o que se digita vai direto para `campos`. */
function gradeDeCampos(t: Textos, campos: CamposFolha): HTMLElement {
  const c = t.folha.inserir;
  const grade = document.createElement('div');
  grade.className = 'duas';
  grade.append(
    campoTexto(c.cliente, campos.cliente, (v) => (campos.cliente = v)),
    campoTexto(c.fase, campos.fase, (v) => (campos.fase = v)),
    campoTexto(c.disciplina, campos.disciplina, (v) => (campos.disciplina = v)),
    campoTexto(c.tituloDesenho, campos.titulo, (v) => (campos.titulo = v)),
    campoTexto(c.subtitulo, campos.subtitulo, (v) => (campos.subtitulo = v)),
    campoTexto(c.numero, campos.numero, (v) => (campos.numero = v)),
    campoTexto(c.revisao, campos.revisao, (v) => (campos.revisao = v)),
    campoTexto(c.numeroDaFolha, campos.folha, (v) => (campos.folha = v)),
  );
  return grade;
}

/** Edita o carimbo de uma folha: escolhe a folha (se há mais de uma), mostra os campos de hoje e devolve os novos; `null` se desistiu. */
export function perguntarCarimbo(t: Textos, dados: FolhasDoMotor, ler: (indice: number) => CamposFolha): Promise<{ indice: number; campos: CamposFolha } | null> {
  const c = t.folha.carimbo;
  const { fundo, caixa } = caixaBase(c.titulo, dados.folhas.length === 0 ? t.folha.escala.semFolhas : c.corpo);
  let indice = 0;
  let campos = dados.folhas.length > 0 ? ler(0) : ({} as CamposFolha);
  const meio = document.createElement('div');
  const pe = document.createElement('div');
  pe.className = 'pe';
  let sair: (r: { indice: number; campos: CamposFolha } | null) => void = () => undefined;
  const refazer = (): void => {
    const seletor = campo(t.folha.escala.qual, listaSuspensa(dados.folhas.map((f) => String(f.indice)), (v) => {
      const f = dados.folhas[Number(v)]!;
      return t.folha.item.replace('{n}', String(f.indice + 1)).replace('{papel}', f.papel).replace('{escala}', f.escala_texto);
    }, String(indice), (v) => {
      indice = Number(v);
      campos = ler(indice);
      refazer();
    }));
    meio.replaceChildren(...(dados.folhas.length > 1 ? [seletor] : []), gradeDeCampos(t, campos));
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
  const aberta = abrir<{ indice: number; campos: CamposFolha }>(fundo, cancelar, (fechar) => dados.folhas.length > 0 && fechar({ indice, campos }));
  sair = aberta.fechar;
  return aberta.resultado;
}

/** Pergunta papel, escala e carimbo de uma folha nova; `null` se desistiu. */
export function perguntarFolhaNova(t: Textos, dados: FolhasDoMotor): Promise<PedidoFolhaNova | null> {
  const c = t.folha.inserir;
  const { fundo, caixa } = caixaBase(c.titulo, c.corpo);
  const campos: CamposFolha = { cliente: '', fase: '', disciplina: '', titulo: '', subtitulo: '', numero: '', revisao: '', folha: '', divisoria: false };
  let papel = dados.papeis.includes('A1') ? 'A1' : (dados.papeis[0] ?? 'A1');
  let escala: string = AUTOMATICA;
  const textoDaEscala = new Map(dados.escalas.map(([valor, texto]) => [String(valor), texto]));

  const linha1 = document.createElement('div');
  linha1.className = 'duas';
  linha1.append(
    campo(c.papel, listaSuspensa(dados.papeis, (p) => p, papel, (v) => (papel = v))),
    campo(c.escala, listaSuspensa([AUTOMATICA, ...dados.escalas.map(([v]) => String(v))], (v) => (v === AUTOMATICA ? c.automatica : (textoDaEscala.get(v) ?? v)), escala, (v) => (escala = v))),
  );
  caixa.append(linha1, Object.assign(document.createElement('p'), { textContent: c.carimbo }));
  const grade = gradeDeCampos(t, campos);
  caixa.append(grade);
  const divisoria = document.createElement('label');
  const caixaDivisoria = Object.assign(document.createElement('input'), { type: 'checkbox' });
  caixaDivisoria.addEventListener('change', () => (campos.divisoria = caixaDivisoria.checked));
  divisoria.append(caixaDivisoria, c.divisoria);
  caixa.append(divisoria);

  const pedido = (): PedidoFolhaNova => ({ papel, ...(escala === AUTOMATICA ? {} : { escala: Number(escala) }), campos });
  const pe = document.createElement('div');
  pe.className = 'pe';
  let sair: (r: PedidoFolhaNova | null) => void = () => undefined;
  const ok = botao(c.inserir, true, () => sair(pedido()));
  pe.append(botao(c.cancelar, false, () => sair(null)), ok);
  caixa.append(pe);
  const caixaAberta = abrir<PedidoFolhaNova>(fundo, ok, (fechar) => fechar(pedido()));
  sair = caixaAberta.fechar;
  return caixaAberta.resultado;
}

/** O que a caixa "Escala da folha" devolve: uma escala (denominador) ou "ajustar ao desenho". */
export type EscolhaDeEscala = { indice: number; escala: number } | { indice: number; ajustar: true };

/** Pergunta qual folha e qual escala; `null` se fechou sem escolher. Os botões agem na hora, pela função `aplicar`. */
export function perguntarEscalaDaFolha(t: Textos, dados: FolhasDoMotor, aplicar: (escolha: EscolhaDeEscala) => FolhasDoMotor): Promise<void> {
  const c = t.folha.escala;
  const { fundo, caixa } = caixaBase(c.titulo, dados.folhas.length === 0 ? c.semFolhas : c.corpo);
  let atual = dados;
  let indice = 0;
  let escala = String(dados.folhas[0]?.escala ?? dados.escalas[0]?.[0] ?? 100);
  const cx = document.createElement('div');
  cx.className = 'duas';
  const refazer = (): void => {
    cx.replaceChildren(
      campo(c.qual, listaSuspensa(atual.folhas.map((f) => String(f.indice)), (v) => {
        const f = atual.folhas[Number(v)]!;
        return t.folha.item.replace('{n}', String(f.indice + 1)).replace('{papel}', f.papel).replace('{escala}', f.escala_texto);
      }, String(indice), (v) => {
        indice = Number(v);
        escala = String(atual.folhas[indice]?.escala ?? escala);
        refazer();
      })),
      campo(c.escala, listaSuspensa(atual.escalas.map(([v]) => String(v)), (v) => atual.escalas.find(([x]) => String(x) === v)?.[1] ?? v, escala, (v) => (escala = v))),
    );
  };
  const pe = document.createElement('div');
  pe.className = 'pe';
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
  const caixaAberta = abrir<never>(fundo, fechar, (f) => f(null));
  sair = caixaAberta.fechar;
  return caixaAberta.resultado.then(() => undefined);
}
