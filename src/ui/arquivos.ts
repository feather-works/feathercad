/**
 * Abrir, salvar e novo. No aplicativo quem abre a caixa e lê/grava o disco é o
 * Rust (`src-tauri/src/arquivos.rs`); no navegador (`npm run dev`) o mesmo fluxo
 * roda com `<input type=file>` e download — o motor é o mesmo e é ele que
 * entende DXF e `.fcdz`. Cada função devolve o estado novo do motor, ou `null`
 * se o usuário cancelou.
 */

import { desenhoAbrir, desenhoExportarPdf, desenhoSalvar, noApp } from '../ponte/nativo.ts';
import type { Estado, Motor, OpcoesPdf, Unidade } from '../ponte/index.ts';
import { formatoPadrao } from './preferencias.ts';

/** Pede um arquivo ao navegador. `null` se cancelou. */
function escolherArquivoNoNavegador(): Promise<File | null> {
  return new Promise((resolve) => {
    const entrada = document.createElement('input');
    entrada.type = 'file';
    entrada.accept = '.dxf,.fcdz';
    entrada.addEventListener('change', () => resolve(entrada.files?.[0] ?? null));
    entrada.addEventListener('cancel', () => resolve(null));
    entrada.click();
  });
}

/** Abre um desenho (`caminho` vem dos recentes/projetos; sem ele, abre a caixa). */
export async function abrirDesenho(motor: Motor, caminho?: string): Promise<Estado | null> {
  if (noApp()) return desenhoAbrir(caminho);
  const arquivo = await escolherArquivoNoNavegador();
  if (!arquivo) return null;
  motor.documentoAbrir(new Uint8Array(await arquivo.arrayBuffer()));
  const formato = arquivo.name.toLowerCase().endsWith('.fcdz') ? 'fcdz' : 'dxf';
  // No navegador não há caminho de verdade: o nome do arquivo faz as vezes.
  return motor.documentoArquivoDefinir(arquivo.name, formato, false);
}

/** Baixa os bytes como arquivo (navegador). */
function baixar(nome: string, bytes: Uint8Array): void {
  const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'application/octet-stream' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = nome;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Salva; `como` força escolher o destino de novo. */
export async function salvarDesenho(motor: Motor, como: boolean): Promise<Estado | null> {
  if (noApp()) return desenhoSalvar(como, formatoPadrao());
  const atual = motor.documentoArquivo();
  const formato = atual?.formato ?? formatoPadrao();
  const nome = atual?.nome ?? `desenho.${formato}`;
  baixar(nome, motor.documentoGravar(formato));
  return motor.documentoArquivoDefinir(nome, formato, true);
}

/** Exporta o PDF (uma página por folha). Devolve o nome/caminho gravado, ou `null` se o usuário cancelou. */
export async function exportarPdf(motor: Motor, opcoes: OpcoesPdf): Promise<string | null> {
  if (noApp()) return (await desenhoExportarPdf(opcoes))?.caminho ?? null;
  const nome = `${(motor.documentoArquivo()?.nome ?? 'desenho').replace(/\.[^.]+$/, '')}.pdf`;
  baixar(nome, motor.documentoExportarPdf(opcoes));
  return nome;
}

/** Desenho em branco. */
export function novoDesenho(motor: Motor, unidade: Unidade): Estado {
  return motor.documentoNovo(unidade);
}
