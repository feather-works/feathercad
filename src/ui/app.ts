/**
 * A fiação: cria as peças, liga uma na outra e repassa cada estado novo do motor
 * a todas. Duas "telas" dentro da janela: a página inicial (aba Início) e o
 * desenho (Ribbon, área, linha de comando e rodapé). Quem abre/salva arquivo é
 * o Rust (ver `arquivos.ts`); aqui só se decide **quando** perguntar sobre
 * alterações não salvas.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import { medir } from '../ponte/metricas.ts';
import { comandosDe, regionalDe, texto, textos } from '../idiomas/index.ts';
import { criarAbasArquivo, type AbaAtiva } from './abas-arquivo.ts';
import { registrarAvisos } from './avisos.ts';
import { perguntarExportacao } from './caixa-exportar.ts';
import { abrirDesenho, exportarPdf, novoDesenho, salvarDesenho } from './arquivos.ts';
import { criarAreaDesenho } from './area-desenho.ts';
import { confirmarAlteracoes } from './caixa-confirmar.ts';
import { criarMenuContexto } from './menu-contexto.ts';
import { criarConfiguracoes } from './configuracoes.ts';
import { criarPainelDeCamadas } from './camadas-painel.ts';
import { criarLinhaComando } from './linha-comando.ts';
import { criarPainelDePropriedades } from './propriedades-painel.ts';
import { criarPaginaInicial } from './pagina-inicial.ts';
import { perguntarUnidade } from './caixa-unidade.ts';
import { cotaPreferida, guardarOrtoAngulo, guardarSnapModos, ortoAnguloPreferido, snapModosPreferidos, fundoPreferido, guardarFundo, guardarUnidade, unidadePreferida } from './preferencias.ts';
import { criarRibbon } from './ribbon.ts';
import { criarWidgetsDaRibbon } from './ribbon-widgets.ts';
import { criarRodape } from './rodape.ts';
import { instalarEstilos } from './styles.ts';
import { teclaDoEvento } from './teclado.ts';
import { definirModo, modoSalvo, proximoModo } from './tema.ts';
import { avisar } from './toast.ts';
import { desenhoVerificarDisco, noApp } from '../ponte/nativo.ts';
import { criarTitulo } from './titulo.ts';

/** Monta a janela dentro de `raiz`, ligada ao `motor`. */
export function iniciarApp(raiz: HTMLElement, motor: Motor): void {
  instalarEstilos();
  const t = textos();
  let ativa: AbaAtiva = 'inicio';
  let temDesenho = false;
  let atual: Estado = motor.estado();

  const aplicar = (e: Estado): void => {
    atual = e;
    titulo.atualizar(e, temDesenho);
    ribbon.atualizar(e);
    abas.atualizar(e, ativa, temDesenho);
    comando.atualizar(e);
    rodape.atualizar(e);
    area.atualizar(e);
    painelCamadas.atualizar(e);
    painelProps.atualizar(e);
    config.atualizar();
  };
  const gesto = (nome: string, fn: () => Estado): void => {
    aplicar(medir('ação', nome, fn));
  };

  /** Mostra a página inicial ou o desenho. */
  const mostrar = (aba: AbaAtiva): void => {
    ativa = aba;
    const noDesenho = aba === 'desenho';
    ribbon.el.hidden = !noDesenho;
    desenhoWrap.hidden = !noDesenho;
    inicio.el.hidden = noDesenho;
    aplicar(atual);
    if (noDesenho) area.atualizar(atual);
    else void inicio.atualizar();
  };

  const falhou = (chave: 'abrir' | 'salvar' | 'exportar', e: unknown): void => {
    const motivo = e instanceof Error ? e.message : String(e);
    console.error(`[erro] ${chave}: ${motivo}`); // o console leva o erro ao Monitor
    avisar(texto(`erro.${chave}`, { motivo }));
  };

  /** Salva (ou "salvar como"). `true` se gravou; `false` se cancelou ou deu erro. */
  async function salvar(como: boolean): Promise<boolean> {
    if (!temDesenho) return false;
    try {
      const e = await salvarDesenho(motor, como);
      if (e) {
        aplicar(e);
        registrarAvisos(e, e.arquivo?.nome ?? t.app.semTitulo);
      }
      return e !== null;
    } catch (e) {
      falhou('salvar', e);
      return false;
    }
  }

  /** Antes de trocar ou fechar o desenho: se há alteração, pergunta. `false` = o usuário desistiu. */
  async function liberar(): Promise<boolean> {
    if (!temDesenho || !atual.sujo) return true;
    const r = await confirmarAlteracoes(t, atual.arquivo?.nome ?? t.app.semTitulo);
    if (r === 'cancelar') return false;
    return r === 'descartar' || (await salvar(false));
  }

  /** Exporta as folhas em PDF: pergunta papel, orientação e folhas; o Rust monta o arquivo. */
  async function exportar(): Promise<void> {
    if (!temDesenho) return;
    try {
      const opcoes = await perguntarExportacao(t, motor.documentoFolhas(), atual.unidade ?? '');
      if (!opcoes) return;
      const onde = await exportarPdf(motor, opcoes);
      if (onde) avisar(texto('aviso.pdfGravado', { caminho: onde }));
    } catch (e) {
      falhou('exportar', e);
    }
  }

  async function novo(): Promise<void> {
    if (!(await liberar())) return;
    // Todo desenho novo começa perguntando a unidade (mm, cm ou m); Esc desiste do Novo.
    const unidade = await perguntarUnidade(t, unidadePreferida());
    if (!unidade) return;
    guardarUnidade(unidade);
    temDesenho = true;
    aplicar(novoDesenho(motor, unidade));
    mostrar('desenho');
  }

  async function abrir(caminho?: string): Promise<void> {
    if (!(await liberar())) return;
    try {
      const e = await abrirDesenho(motor, caminho);
      if (!e) return;
      temDesenho = true;
      aplicar(e);
      registrarAvisos(e, e.arquivo?.nome ?? t.app.semTitulo);
      mostrar('desenho');
    } catch (e) {
      falhou('abrir', e);
    }
  }

  async function fechar(): Promise<void> {
    if (!(await liberar())) return;
    temDesenho = false;
    aplicar(motor.documentoNovo());
    mostrar('inicio');
  }

  const titulo = criarTitulo(t, {
    novo: () => void novo(),
    abrir: () => void abrir(),
    salvar: () => void salvar(false),
    exportar: () => void exportar(),
    desfazer: () => gesto('desfazer', () => motor.desfazer()),
    refazer: () => gesto('refazer', () => motor.refazer()),
    tema: () => {
      definirModo(proximoModo(modoSalvo()));
      titulo.modoDoTema(modoSalvo());
      config.atualizar();
    },
    configuracoes: () => config.alternar(),
  });
  titulo.modoDoTema(modoSalvo());
  const config = criarConfiguracoes(t, {
    motor,
    estado: () => atual,
    aplicar,
    temDesenho: () => temDesenho,
    salvar: (como) => void salvar(como),
  });
  const painelCamadas = criarPainelDeCamadas(t, motor, aplicar);
  const painelProps = criarPainelDePropriedades(t, motor, aplicar);
  const ribbon = criarRibbon(
    t,
    (c) => gesto(`botão ${c}`, () => motor.executar(c)),
    (acao) => {
      if (acao === 'camadas') painelCamadas.alternar();
      else if (acao === 'propriedades') painelProps.alternar();
    },
    criarWidgetsDaRibbon(t, motor, aplicar),
  );
  const abas = criarAbasArquivo(t, {
    aoInicio: () => mostrar('inicio'),
    aoDesenho: () => mostrar('desenho'),
    aoFechar: () => void fechar(),
    aoNovo: () => void novo(),
  });
  const inicio = criarPaginaInicial({ t, aoAbrir: (c) => void abrir(c), aoNovo: () => void novo() });
  const menu = criarMenuContexto((i) => {
    const [familia, nome] = i.id.split(':') as [string, string | undefined];
    if (i.id === 'enter') gesto('digitar', () => motor.digitar(''));
    else if (i.id === 'cancelar') gesto('cancelar', () => motor.cancelar());
    else if (i.id === 'fechar') gesto('digitar', () => motor.digitar(comandosDe().opcoes['fechar']?.[0] ?? ''));
    else if (familia === 'interruptor' && nome) gesto(`interruptor ${nome}`, () => motor.interruptor(nome as 'grade' | 'snap' | 'orto' | 'osnap', !i.marcado));
    else if (familia === 'janela') {
      if (nome === 'propriedades') painelProps.alternar();
      else if (nome === 'zoom_ajustar') area.ajustar();
      else config.alternar();
    } else gesto(i.id, () => motor.executar(i.id));
  });
  const area = criarAreaDesenho({
    motor,
    t,
    aoMenu: (x, y, itens) => menu.abrir(x, y, itens),
    aoMudar: aplicar,
    aoCursor: (r) => rodape.coordenadas(r),
    aoZoom: (z) => rodape.zoom(z),
  });
  const comando = criarLinhaComando(t, (texto) => gesto('digitar', () => motor.digitar(texto)));
  const rodape = criarRodape(
    t,
    (nome, ligado) => gesto(`interruptor ${nome}`, () => motor.interruptor(nome, ligado)),
    (fundo) => {
      guardarFundo(fundo);
      gesto('fundo', () => motor.fundoDefinir(fundo));
    },
    (graus) => {
      guardarOrtoAngulo(graus);
      gesto('orto angulo', () => motor.ortoAnguloDefinir(graus));
    },
    (modos) => {
      guardarSnapModos(modos);
      gesto('snap modos', () => motor.osnapModosDefinir(modos));
    },
  );

  const desenhoWrap = document.createElement('div');
  desenhoWrap.className = 'desenho-wrap';
  desenhoWrap.append(area.el, comando.el, rodape.el, painelCamadas.el, painelProps.el);
  const corpo = document.createElement('div');
  corpo.className = 'corpo';
  corpo.append(inicio.el, desenhoWrap);
  raiz.append(titulo.el, ribbon.el, abas.el, corpo, config.el, menu.el);

  const acoesDeJanela = { novo, abrir: () => abrir(), salvar: () => salvar(false), salvar_como: () => salvar(true) };

  window.addEventListener('keydown', (ev) => {
    const alvo = ev.target instanceof HTMLElement ? ev.target : null;
    // Digitando numa caixa (busca, nome de camada, linha de comando…): as teclas soltas são dela.
    const emCampo = alvo?.closest('input, select, textarea') != null;
    const tecla = teclaDoEvento(ev);
    if (ev.ctrlKey && ev.key === ',') {
      ev.preventDefault();
      config.alternar();
      return;
    }
    if (tecla === 'Ctrl+1' && ativa === 'desenho') {
      ev.preventDefault();
      painelProps.alternar();
      return;
    }
    if (tecla === null) {
      // Enter fora da caixa de comando confirma (seleção, fim do comando), como no AutoCAD.
      if (ativa === 'desenho' && !emCampo && ev.key === 'Enter') {
        ev.preventDefault();
        gesto('digitar', () => motor.digitar(''));
        return;
      }
      // Tecla comum: cai na linha de comando, como no AutoCAD (só no desenho).
      if (ativa === 'desenho' && !emCampo && ev.key.length === 1) comando.focar();
      return;
    }
    // Ctrl+C/X/V numa caixa de texto copiam texto, não objetos do desenho.
    if (emCampo && (tecla === 'Ctrl+C' || tecla === 'Ctrl+X' || tecla === 'Ctrl+V')) return;
    // Delete numa caixa de texto apaga texto, não o desenho.
    if (tecla === 'Delete' && (emCampo || ativa !== 'desenho')) return;
    const r = motor.atalho(tecla);
    if (!r.tratado) return;
    ev.preventDefault();
    aplicar(r.estado);
    if (r.janela) void acoesDeJanela[r.janela]();
  });

  try {
    const faltam = motor.comandosQueFaltam();
    if (faltam.length > 0) throw new Error(`comandos que faltam no motor: ${faltam.join(', ')}`);
    motor.idioma(regionalDe(), comandosDe());
    motor.cotaTamanhoDefinir(cotaPreferida());
    motor.ortoAnguloDefinir(ortoAnguloPreferido());
    motor.osnapModosDefinir(snapModosPreferidos());
  } catch (e) {
    // O motor (Rust) é mais antigo que a interface: acontece quando o `tauri dev` não recompilou.
    raiz.replaceChildren(Object.assign(document.createElement('pre'), { className: 'motor-antigo', textContent: `${t.erro.motorDesatualizado}\n\n${e instanceof Error ? e.message : String(e)}` }));
    return;
  }
  aplicar(motor.fundoDefinir(fundoPreferido()));
  mostrar('inicio');
  vigiarArquivo();

  /**
   * O desenho aberto pode ser regravado por fora (o servidor MCP, outro programa): a cada segundo
   * o Rust olha a data do arquivo e, se mudou, recarrega; aqui só se desenha o resultado. Fica em
   * silêncio enquanto um gesto está no meio (ferramenta ativa ou arrasto), para não atrapalhar.
   */
  function vigiarArquivo(): void {
    if (!noApp()) return;
    let ocupado = false;
    setInterval(() => {
      if (ocupado || !temDesenho || atual.ferramenta !== 'nenhuma' || atual.arrastando) return;
      ocupado = true;
      desenhoVerificarDisco()
        .then((m) => {
          if (m && 'recarregado' in m) aplicar(m.recarregado);
          else if (m) avisar(texto('erro.mudouNoDisco', { nome: m.conflito }));
        })
        .catch(() => undefined)
        .finally(() => {
          ocupado = false;
        });
    }, 1000);
  }
}
