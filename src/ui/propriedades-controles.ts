/**
 * Os controles de propriedade dos objetos selecionados (cor, camada, tipo de linha, espessura), usados
 * na faixa (painel Propriedades) e na paleta lateral. **Só desenham** o que o motor resumiu
 * (`propriedades_selecao`) e mandam de volta o que o usuário escolheu (`propriedades_definir`).
 */

import type { Motor } from '../ponte/index.ts';
import { ESPESSURAS_MM, TIPOS_DE_LINHA, type CampoDePropriedade, type ResumoDePropriedades } from '../ponte/tipos.ts';
import type { Textos } from '../idiomas/index.ts';

export interface ControleDePropriedade {
  el: HTMLSelectElement;
  /** Mostra o resumo dos selecionados (`null` ou sem objetos: desabilitado). */
  mostrar(r: ResumoDePropriedades | null): void;
}

const POR_CAMADA = 'porcamada';
const VARIADO = '__variado';
const OUTRA = '__outra';

function opcao(valor: string, rotulo: string): HTMLOptionElement {
  const o = document.createElement('option');
  o.value = valor;
  o.textContent = rotulo;
  return o;
}

/** Põe o valor do campo no select: acrescenta uma opção temporária para "variado" ou para valor fora da lista. */
function refletir(sel: HTMLSelectElement, campo: CampoDePropriedade | undefined, rotuloVariado: string, rotuloDe: (valor: string) => string): void {
  sel.querySelectorAll('option[data-temp]').forEach((o) => o.remove());
  if (!campo || campo.estado === 'vazio') {
    sel.value = POR_CAMADA;
    return;
  }
  if (campo.estado === 'variado') {
    const o = opcao(VARIADO, rotuloVariado);
    o.dataset['temp'] = '1';
    sel.prepend(o);
    sel.value = VARIADO;
    return;
  }
  if (![...sel.options].some((o) => o.value.toLowerCase() === campo.valor.toLowerCase())) {
    const o = opcao(campo.valor, rotuloDe(campo.valor));
    o.dataset['temp'] = '1';
    sel.append(o);
  }
  sel.value = [...sel.options].find((o) => o.value.toLowerCase() === campo.valor.toLowerCase())?.value ?? POR_CAMADA;
}

function controle(sel: HTMLSelectElement, dica: string): void {
  sel.className = 'prop-controle';
  sel.title = dica;
  sel.disabled = true;
  sel.addEventListener('keydown', (e) => e.stopPropagation());
}

/** Cor: PorCamada, as sete básicas e "Personalizada…" (o seletor de cor do sistema). */
export function controleDeCor(t: Textos, aplicar: (valor: string) => void): ControleDePropriedade {
  const sel = document.createElement('select');
  controle(sel, t.propriedades.cor);
  const nomes = [t.propriedades.cores.vermelho, t.propriedades.cores.amarelo, t.propriedades.cores.verde, t.propriedades.cores.ciano, t.propriedades.cores.azul, t.propriedades.cores.magenta, t.propriedades.cores.branco];
  /** As opções vêm do motor (as cores básicas são dado dele): refeitas a cada resumo. */
  const montar = (basicas: string[]): void => {
    sel.replaceChildren(opcao(POR_CAMADA, t.propriedades.porCamada), ...basicas.map((hex, i) => opcao(hex, nomes[i] ?? hex)), opcao(OUTRA, t.propriedades.cores.personalizada));
  };
  montar([]);
  const seletor = document.createElement('input');
  seletor.type = 'color';
  // Fica no documento (escondido): o seletor de cor do sistema só abre de um campo que está na página.
  seletor.style.cssText = 'position:fixed;left:-100px;top:-100px;width:0;height:0;opacity:0;pointer-events:none';
  document.body.append(seletor);
  seletor.addEventListener('change', () => aplicar(seletor.value));
  let ultimo = POR_CAMADA;
  sel.addEventListener('change', () => {
    if (sel.value === OUTRA) {
      sel.value = ultimo;
      if (ultimo.startsWith('#')) seletor.value = ultimo;
      seletor.click();
    } else if (sel.value !== VARIADO) {
      aplicar(sel.value);
    }
  });
  return {
    el: sel,
    mostrar(r) {
      if (r) montar(r.cores_basicas);
      sel.disabled = !r || r.n === 0;
      refletir(sel, r?.cor, t.propriedades.variado, (v) => v);
      ultimo = sel.value;
    },
  };
}

/** Tipo de linha: PorCamada e os seis tipos. */
export function controleDeTipoDeLinha(t: Textos, aplicar: (valor: string) => void): ControleDePropriedade {
  const sel = document.createElement('select');
  controle(sel, t.propriedades.tipoLinha);
  sel.append(opcao(POR_CAMADA, t.propriedades.porCamada));
  for (const dxf of TIPOS_DE_LINHA) sel.append(opcao(dxf, t.camadas.tipos[dxf]));
  sel.addEventListener('change', () => sel.value !== VARIADO && aplicar(sel.value));
  return {
    el: sel,
    mostrar(r) {
      sel.disabled = !r || r.n === 0;
      refletir(sel, r?.tipo_linha, t.propriedades.variado, (v) => v);
    },
  };
}

/** Espessura de linha: PorCamada e a lista padrão em mm (o número é formatado pelo `Intl`, na língua do sistema). */
export function controleDeEspessura(t: Textos, aplicar: (valor: string) => void): ControleDePropriedade {
  const sel = document.createElement('select');
  controle(sel, t.propriedades.espessura);
  const numero = new Intl.NumberFormat(document.documentElement.lang || undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  sel.append(opcao(POR_CAMADA, t.propriedades.porCamada));
  for (const mm of ESPESSURAS_MM) sel.append(opcao(String(mm), `${numero.format(mm)} mm`));
  sel.addEventListener('change', () => sel.value !== VARIADO && aplicar(sel.value));
  return {
    el: sel,
    mostrar(r) {
      sel.disabled = !r || r.n === 0;
      refletir(sel, r?.espessura, t.propriedades.variado, (v) => `${numero.format(Number(v))} mm`);
    },
  };
}

/** Camada: uma opção por camada do desenho (a lista vem do motor a cada `mostrar`). */
export function controleDeCamada(t: Textos, motor: Motor, aplicar: (valor: string) => void): ControleDePropriedade {
  const sel = document.createElement('select');
  controle(sel, t.propriedades.camada);
  sel.addEventListener('change', () => sel.value !== VARIADO && aplicar(sel.value));
  return {
    el: sel,
    mostrar(r) {
      const nomes = motor.camadas().linhas.map((l) => l.nome);
      sel.replaceChildren(...nomes.map((n) => opcao(n, n)));
      sel.disabled = !r || r.n === 0;
      refletir(sel, r?.camada, t.propriedades.variado, (v) => v);
    },
  };
}

/** Quem pede o resumo dos selecionados ao motor **uma vez por estado** (a faixa e a paleta dividem o resultado). */
export function fonteDePropriedades(motor: Motor): (e: { versao: number; selecionados: number; selecao_rev: number }) => ResumoDePropriedades {
  let chave = '';
  let resumo: ResumoDePropriedades | null = null;
  return (e) => {
    const nova = `${e.versao}:${e.selecionados}:${e.selecao_rev}`;
    if (resumo === null || nova !== chave) {
      resumo = motor.propriedadesSelecao();
      chave = nova;
    }
    return resumo;
  };
}

export interface ControleDeTexto {
  el: HTMLInputElement;
  mostrar(campo?: CampoDePropriedade, rotuloVariado?: string): void;
}

export function controleDeTexto(dica: string, aplicar?: (valor: string) => void): ControleDeTexto {
  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'prop-controle prop-controle-input';
  input.title = dica;
  input.disabled = true;
  let ultimoValor = '';

  const enviar = () => {
    if (!input.disabled && input.value !== ultimoValor && aplicar) {
      ultimoValor = input.value;
      aplicar(input.value);
    }
  };

  input.addEventListener('change', enviar);
  input.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter') {
      enviar();
      input.blur();
    } else if (ev.key === 'Escape') {
      input.value = ultimoValor;
      input.blur();
    }
    ev.stopPropagation();
  });

  return {
    el: input,
    mostrar(campo, rotuloVariado = '*VARIADO*') {
      if (!campo || campo.estado === 'vazio') {
        input.value = '';
        input.disabled = true;
        ultimoValor = '';
      } else if (campo.estado === 'variado') {
        input.value = rotuloVariado;
        input.disabled = false;
        ultimoValor = rotuloVariado;
      } else {
        input.value = campo.valor;
        input.disabled = false;
        ultimoValor = campo.valor;
      }
    },
  };
}
