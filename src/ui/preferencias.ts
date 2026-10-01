/**
 * Preferências do usuário guardadas no navegador (`localStorage`, prefixo
 * `feathercad.`). São conveniências: se o armazenamento falhar (janela privada,
 * dados bloqueados), vale o padrão e o app segue normalmente.
 */

import { TIPOS_DE_SNAP, type Fundo, type TipoDeSnap, type Unidade } from '../ponte/tipos.ts';

const FUNDOS: Fundo[] = ['nada', 'pontos', 'grade'];

function ler(chave: string): string | null {
  try {
    return localStorage.getItem(`feathercad.${chave}`);
  } catch {
    return null;
  }
}

function gravar(chave: string, valor: string): void {
  try {
    localStorage.setItem(`feathercad.${chave}`, valor);
  } catch {
    // sem armazenamento: só não lembra na próxima vez
  }
}

/** O medidor de FPS da área de desenho está ligado? (Padrão: desligado; é diagnóstico.) */
export function fpsLigado(): boolean {
  return ler('fps') === '1';
}

/** Liga/desliga o medidor de FPS: guarda a escolha e avisa a área de desenho na hora. */
export function guardarFps(ligado: boolean): void {
  gravar('fps', ligado ? '1' : '0');
  window.dispatchEvent(new CustomEvent('feathercad-fps', { detail: ligado }));
}

/** O zoom da roda é animado (suave)? Padrão: sim. */
export function zoomSuave(): boolean {
  return ler('zoomSuave') !== '0';
}

export function guardarZoomSuave(ligado: boolean): void {
  gravar('zoomSuave', ligado ? '1' : '0');
}

/** O desenho é pintado num Worker (segundo plano)? Padrão: sim. Vale ao reabrir (o tipo de contexto do canvas não muda depois). */
export function pintarEmSegundoPlano(): boolean {
  return ler('segundoPlano') !== '0';
}

export function guardarPintarEmSegundoPlano(ligado: boolean): void {
  gravar('segundoPlano', ligado ? '1' : '0');
}

/** O texto real é desenhado também durante o zoom e o pan? Padrão: não (vira traço até o movimento parar). */
export function textoNoMovimento(): boolean {
  return ler('textoNoMovimento') !== '0';
}

export function guardarTextoNoMovimento(ligado: boolean): void {
  gravar('textoNoMovimento', ligado ? '1' : '0');
}

/** O estilo de fundo escolhido (padrão: grade). */
export function fundoPreferido(): Fundo {
  const v = ler('fundo');
  return FUNDOS.find((f) => f === v) ?? 'grade';
}

export function guardarFundo(f: Fundo): void {
  gravar('fundo', f);
}

/**
 * O formato em que um arquivo **novo** é salvo. Decisão de 29/09/2026: o padrão de
 * fábrica é `.dxf` (universal); o `.fcdz` é opção de quem quer o formato próprio.
 * A tela de Configurações (`configuracoes.ts`) escolhe; aqui só se lê e se guarda.
 */
export function formatoPadrao(): 'dxf' | 'fcdz' {
  return ler('formatoPadrao') === 'fcdz' ? 'fcdz' : 'dxf';
}

export function guardarFormatoPadrao(f: 'dxf' | 'fcdz'): void {
  gravar('formatoPadrao', f);
}

/** A última unidade escolhida ao criar um desenho (já vem marcada na próxima pergunta). */
export function unidadePreferida(): Unidade {
  const v = ler('unidade');
  return v === 'cm' || v === 'm' ? v : 'mm';
}

export function guardarUnidade(u: Unidade): void {
  gravar('unidade', u);
}

/** Os passos de ângulo do Orto que aparecem prontos (o campo aceita qualquer número). */
export const ANGULOS_DO_ORTO = [90, 60, 45, 30, 15] as const;

/** O passo do Orto guardado (padrão 90: só horizontal e vertical). */
export function ortoAnguloPreferido(): number {
  const n = Number(ler('ortoAngulo'));
  return Number.isFinite(n) && n > 0 && n <= 180 ? n : 90;
}

export function guardarOrtoAngulo(graus: number): void {
  gravar('ortoAngulo', String(graus));
}

/** Os tipos de snap ao objeto que o usuário ligou/desligou (só os que ele mexeu; o resto fica no padrão do motor). */
export function snapModosPreferidos(): Partial<Record<TipoDeSnap, boolean>> {
  try {
    const bruto: unknown = JSON.parse(ler('snapModos') ?? '{}');
    if (typeof bruto !== 'object' || bruto === null) return {};
    return Object.fromEntries(TIPOS_DE_SNAP.flatMap((t) => (typeof (bruto as Record<string, unknown>)[t] === 'boolean' ? [[t, (bruto as Record<string, boolean>)[t]]] : [])));
  } catch {
    return {};
  }
}

export function guardarSnapModos(modos: Partial<Record<TipoDeSnap, boolean>>): void {
  gravar('snapModos', JSON.stringify({ ...snapModosPreferidos(), ...modos }));
}

/** O tamanho da cota escolhido em Configurações (na unidade do desenho); `null` = automático. */
export function cotaPreferida(): number | null {
  const n = Number(ler('cota'));
  return Number.isFinite(n) && n > 0 ? n : null;
}

export function guardarCota(tamanho: number | null): void {
  gravar('cota', tamanho === null ? '' : String(tamanho));
}

/** O modo de exibição da página inicial. */
export function visaoDoInicio(): 'lista' | 'grade' {
  return ler('visaoInicio') === 'grade' ? 'grade' : 'lista';
}

export function guardarVisaoDoInicio(v: 'lista' | 'grade'): void {
  gravar('visaoInicio', v);
}

/** As cores de destaque (`azul` é a do tema, sem atributo). As cores em si estão em `base.ts`. */
export const ACENTOS = ['azul', 'verde', 'roxo', 'laranja', 'rosa', 'grafite'] as const;
export type Acento = (typeof ACENTOS)[number];
export const FONTES = ['sans', 'serif', 'mono'] as const;
export type Fonte = (typeof FONTES)[number];
export const CORPOS = [11, 12, 13, 14, 16] as const;

export function acentoPreferido(): Acento {
  const v = ler('acento');
  return ACENTOS.find((a) => a === v) ?? 'azul';
}

export function fontePreferida(): Fonte {
  const v = ler('fonte');
  return FONTES.find((f) => f === v) ?? 'sans';
}

export function corpoPreferido(): number {
  const n = Number(ler('corpo'));
  return CORPOS.find((c) => c === n) ?? 12;
}

export function guardarAparencia(campo: 'acento' | 'fonte' | 'corpo', valor: string | number): void {
  gravar(campo, String(valor));
}

/** Põe acento, fonte e corpo da letra no `<html>` (as variáveis de `base.ts` fazem o resto). */
export function aplicarAparencia(): void {
  const raiz = document.documentElement;
  const acento = acentoPreferido();
  if (acento === 'azul') delete raiz.dataset['acento'];
  else raiz.dataset['acento'] = acento;
  const fonte = fontePreferida();
  if (fonte === 'sans') delete raiz.dataset['fonte'];
  else raiz.dataset['fonte'] = fonte;
  raiz.style.setProperty('--corpo', `${corpoPreferido()}px`);
}
