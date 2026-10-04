/**
 * A fiação: cria as peças, liga uma na outra e repassa cada estado novo do motor
 * a todas. Suporta múltiplas abas de desenho abertas simultaneamente.
 * Quem abre/salva arquivo é o Rust (ver `arquivos.ts`); aqui se gerencia o ciclo
 * de vida de cada aba e alternância entre elas.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import { medir } from '../ponte/metricas.ts';
import { comandosDe, regionalDe, texto, textos } from '../idiomas/index.ts';
import { criarAbasArquivo } from './abas-arquivo.ts';
import { registrarAvisos } from './avisos.ts';
import { perguntarExportacao } from './caixa-exportar.ts';
import { escolherHachura } from './caixa-hachura.ts';
import { perguntarCarimbo, perguntarEscalaDaFolha, perguntarFolhaNova } from './caixa-folha.ts';
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
import { abrirJanelaMonitor, desenhoVerificarDisco, mcpVivoVerificar, noApp } from '../ponte/nativo.ts';
import { devConsole } from './console-depuracao.ts';
import { criarTitulo } from './titulo.ts';

interface TabDoc {
  id: string;
  nome: string;
  caminho?: string;
  formato?: 'dxf' | 'fcdz';
  bytes?: Uint8Array;
  sujo: boolean;
}

/** Monta a janela dentro de `raiz`, ligada ao `motor`. */
export function iniciarApp(raiz: HTMLElement, motor: Motor): void {
  instalarEstilos();
  const t = textos();

  const abasDoc: TabDoc[] = [];
  let abaAtivaId: string = 'inicio';
  let contadorSemTitulo = 1;
  let atual: Estado = motor.estado();

  const temDesenho = (): boolean => abaAtivaId !== 'inicio';

  const aplicar = (e: Estado): void => {
    atual = e;
    const tabAtual = abasDoc.find((a) => a.id === abaAtivaId);
    if (tabAtual) {
      tabAtual.sujo = e.sujo;
      if (e.arquivo) {
        tabAtual.caminho = e.arquivo.caminho;
        tabAtual.nome = e.arquivo.nome;
      }
    }
    const emDesenho = temDesenho();
    titulo.atualizar(e, emDesenho);
    ribbon.atualizar(e);
    abas.atualizar(
      abasDoc.map((a) => ({ id: a.id, nome: a.nome, caminho: a.caminho, sujo: a.sujo })),
      abaAtivaId
    );
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

  function guardarEstadoDaAbaAtual(): void {
    if (abaAtivaId === 'inicio') return;
    const tab = abasDoc.find((a) => a.id === abaAtivaId);
    if (!tab) return;

    tab.sujo = atual.sujo;
    tab.caminho = atual.arquivo?.caminho;
    tab.nome = atual.arquivo?.nome ?? tab.nome;
    tab.formato = (atual.arquivo?.formato as 'dxf' | 'fcdz') ?? tab.formato ?? 'dxf';

    try {
      tab.bytes = motor.documentoGravar('fcdz');
    } catch (err) {
      console.error('Erro ao guardar estado da aba:', err);
    }
  }

  function ativarAba(id: string): void {
    if (id === 'inicio') {
      guardarEstadoDaAbaAtual();
      abaAtivaId = 'inicio';
      ribbon.el.hidden = true;
      desenhoWrap.hidden = true;
      inicio.el.hidden = false;
      aplicar(atual);
      void inicio.atualizar();
      return;
    }

    const tab = abasDoc.find((a) => a.id === id);
    if (!tab) return;

    if (abaAtivaId !== id && abaAtivaId !== 'inicio') {
      guardarEstadoDaAbaAtual();
    }

    abaAtivaId = id;

    if (tab.bytes && tab.bytes.length > 0) {
      motor.documentoAbrir(tab.bytes);
      if (tab.caminho && tab.formato) {
        motor.documentoArquivoDefinir(tab.caminho, tab.formato, !tab.sujo);
      }
      atual = motor.estado();
    }

    ribbon.el.hidden = false;
    desenhoWrap.hidden = false;
    inicio.el.hidden = true;

    aplicar(atual);
    area.ajustar();
  }

  const falhou = (chave: 'abrir' | 'salvar' | 'exportar' | 'folha', e: unknown): void => {
    const motivo = e instanceof Error ? e.message : String(e);
    console.error(`[erro] ${chave}: ${motivo}`);
    avisar(texto(`erro.${chave}`, { motivo }));
  };

  /** Salva (ou "salvar como"). `true` se gravou; `false` se cancelou ou deu erro. */
  async function salvar(como: boolean): Promise<boolean> {
    if (!temDesenho()) return false;
    const tab = abasDoc.find((a) => a.id === abaAtivaId);
    if (!tab) return false;

    try {
      const e = await salvarDesenho(motor, como);
      if (e) {
        tab.sujo = e.sujo;
        tab.caminho = e.arquivo?.caminho;
        tab.nome = e.arquivo?.nome ?? tab.nome;
        tab.formato = (e.arquivo?.formato as 'dxf' | 'fcdz') ?? 'dxf';
        tab.bytes = motor.documentoGravar('fcdz');

        aplicar(e);
        registrarAvisos(e, tab.nome);
        return true;
      }
      return false;
    } catch (e) {
      falhou('salvar', e);
      return false;
    }
  }

  /** Exporta as folhas em PDF. */
  async function exportar(): Promise<void> {
    if (!temDesenho()) return;
    try {
      const opcoes = await perguntarExportacao(t, motor.documentoFolhas(), atual.unidade ?? '');
      if (!opcoes) return;
      const onde = await exportarPdf(motor, opcoes);
      if (onde) avisar(texto('aviso.pdfGravado', { caminho: onde }));
    } catch (e) {
      falhou('exportar', e);
    }
  }

  async function inserirFolha(): Promise<void> {
    if (!temDesenho()) return;
    try {
      const temSelecao = atual.selecionados > 0;
      const pedido = await perguntarFolhaNova(t, motor.folhaListar(), temSelecao);
      if (!pedido) return;
      aplicar(medir('ação', 'folha_inserir', () => motor.folhaInserir(pedido)).estado);
      area.ajustar();
    } catch (e) {
      falhou('folha', e);
    }
  }

  async function escalaDaFolha(): Promise<void> {
    if (!temDesenho()) return;
    try {
      await perguntarEscalaDaFolha(t, motor.folhaListar(), (escolha) => {
        try {
          const r = 'ajustar' in escolha ? motor.folhaAjustar(escolha.indice) : motor.folhaEscala(escolha.indice, escolha.escala);
          aplicar(r.estado);
          area.ajustar();
        } catch (e) {
          falhou('folha', e);
        }
        return motor.folhaListar();
      });
    } catch (e) {
      falhou('folha', e);
    }
  }

  async function carimboDaFolha(): Promise<void> {
    if (!temDesenho()) return;
    try {
      const r = await perguntarCarimbo(t, motor.folhaListar(), (i) => motor.folhaCampos(i));
      if (r) aplicar(motor.folhaCamposDefinir(r.indice, r.campos).estado);
    } catch (e) {
      falhou('folha', e);
    }
  }

  async function novo(): Promise<void> {
    const unidade = await perguntarUnidade(t, unidadePreferida());
    if (!unidade) return;
    guardarUnidade(unidade);

    guardarEstadoDaAbaAtual();

    const id = `tab_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
    const nome = `${t.app.semTitulo} ${contadorSemTitulo++}`;

    novoDesenho(motor, unidade);
    const bytes = motor.documentoGravar('fcdz');

    const novaTab: TabDoc = {
      id,
      nome,
      sujo: false,
      bytes,
    };

    abasDoc.push(novaTab);
    ativarAba(id);
  }

  async function abrir(caminho?: string): Promise<void> {
    try {
      const e = await abrirDesenho(motor, caminho);
      if (!e) return;

      const caminhoFinal = e.arquivo?.caminho;
      if (caminhoFinal) {
        const existente = abasDoc.find((a) => a.caminho === caminhoFinal);
        if (existente) {
          ativarAba(existente.id);
          return;
        }
      }

      guardarEstadoDaAbaAtual();

      const id = `tab_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
      const nome = e.arquivo?.nome ?? t.app.semTitulo;
      const bytes = motor.documentoGravar('fcdz');

      const novaTab: TabDoc = {
        id,
        nome,
        caminho: e.arquivo?.caminho,
        formato: e.arquivo?.formato as 'dxf' | 'fcdz',
        sujo: e.sujo,
        bytes,
      };

      const primeiraTab = abasDoc[0];
      if (abasDoc.length === 1 && primeiraTab && !primeiraTab.sujo && !primeiraTab.caminho) {
        abasDoc[0] = novaTab;
      } else {
        abasDoc.push(novaTab);
      }

      ativarAba(id);
      registrarAvisos(e, nome);
    } catch (e) {
      falhou('abrir', e);
    }
  }

  async function fecharAba(id: string): Promise<void> {
    const tabIndex = abasDoc.findIndex((a) => a.id === id);
    if (tabIndex === -1) return;

    const tab = abasDoc[tabIndex];

    if (tab && tab.sujo) {
      if (abaAtivaId !== id) {
        ativarAba(id);
      }
      const r = await confirmarAlteracoes(t, tab.nome);
      if (r === 'cancelar') return;
      if (r === 'salvar') {
        const ok = await salvar(false);
        if (!ok) return;
      }
    }

    abasDoc.splice(tabIndex, 1);

    if (abaAtivaId === id) {
      if (abasDoc.length > 0) {
        const proximaIndex = Math.min(tabIndex, abasDoc.length - 1);
        const proximaTab = abasDoc[proximaIndex];
        if (proximaTab) {
          ativarAba(proximaTab.id);
        } else {
          ativarAba('inicio');
        }
      } else {
        ativarAba('inicio');
      }
    } else {
      abas.atualizar(
        abasDoc.map((a) => ({ id: a.id, nome: a.nome, caminho: a.caminho, sujo: a.sujo })),
        abaAtivaId
      );
    }
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
    temDesenho,
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
      else if (acao === 'folha_inserir') void inserirFolha();
      else if (acao === 'folha_escala') void escalaDaFolha();
      else if (acao === 'folha_carimbo') void carimboDaFolha();
      else if (acao === 'hachura_padrao') void escolherHachura(t, motor).catch((e) => falhou('folha', e));
      else if (acao === 'vista_ajustar') area.ajustar();
      else if (acao === 'zoom_mais') area.zoomMais();
      else if (acao === 'zoom_menos') area.zoomMenos();
      else if (acao === 'alternar_tema') {
        definirModo(proximoModo(modoSalvo()));
        titulo.modoDoTema(modoSalvo());
        config.atualizar();
      } else if (acao === 'configuracoes') config.alternar('aparencia');
      else if (acao === 'mcp') config.alternar('mcp');
      else if (acao === 'sobre') config.alternar('sobre');
      else if (acao === 'apelidos') config.alternar('apelidos');
      else if (acao === 'unidades') {
        void (async () => {
          const u = await perguntarUnidade(t, unidadePreferida());
          if (u) {
            guardarUnidade(u);
            aplicar(motor.unidadeDefinir(u));
          }
        })();
      } else if (acao === 'exportar_pdf') void exportar();
      else if (acao === 'salvar_dxf') void salvar(true);
      else if (acao === 'salvar_fcdz') void salvar(true);
      else if (acao === 'monitor') {
        if (noApp()) {
          void abrirJanelaMonitor();
        } else {
          devConsole.show();
        }
      }
    },
    criarWidgetsDaRibbon(t, motor, aplicar)
  );

  const abas = criarAbasArquivo(t, {
    aoInicio: () => ativarAba('inicio'),
    aoSelecionarAba: (id) => ativarAba(id),
    aoFecharAba: (id) => void fecharAba(id),
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
    (espaco) => gesto('layout alternar', () => { const e = motor.layoutAlternar(espaco); aplicar(e); return e; }),
    (nome, papel, w, h) => gesto('layout criar', () => { const e = motor.layoutCriar(nome, papel, w, h); aplicar(e); return e; }),
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
    const emCampo = alvo?.closest('input, select, textarea') != null;

    if (ev.ctrlKey && (ev.key === 'w' || ev.key === 'W' || ev.key === 'F4')) {
      ev.preventDefault();
      if (abaAtivaId !== 'inicio') {
        void fecharAba(abaAtivaId);
      }
      return;
    }

    if (ev.ctrlKey && ev.key === 'Tab') {
      ev.preventDefault();
      if (abasDoc.length > 0) {
        const currIdx = abasDoc.findIndex((a) => a.id === abaAtivaId);
        let nextIdx = ev.shiftKey ? currIdx - 1 : currIdx + 1;
        if (currIdx === -1) nextIdx = 0;
        else if (nextIdx >= abasDoc.length) nextIdx = 0;
        else if (nextIdx < 0) nextIdx = abasDoc.length - 1;
        const nextTab = abasDoc[nextIdx];
        if (nextTab) {
          ativarAba(nextTab.id);
        }
      }
      return;
    }

    const tecla = teclaDoEvento(ev);
    if (ev.ctrlKey && ev.key === ',') {
      ev.preventDefault();
      config.alternar();
      return;
    }
    if (tecla === 'Ctrl+1' && temDesenho()) {
      ev.preventDefault();
      painelProps.alternar();
      return;
    }
    if (tecla === null) {
      if (temDesenho() && !emCampo && ev.key === 'Enter') {
        ev.preventDefault();
        gesto('digitar', () => motor.digitar(''));
        return;
      }
      if (temDesenho() && !emCampo && ev.key.length === 1) comando.focar();
      return;
    }
    if (emCampo && (tecla === 'Ctrl+C' || tecla === 'Ctrl+X' || tecla === 'Ctrl+V')) return;
    if (tecla === 'Delete' && (emCampo || !temDesenho())) return;

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
    raiz.replaceChildren(Object.assign(document.createElement('pre'), { className: 'motor-antigo', textContent: `${t.erro.motorDesatualizado}\n\n${e instanceof Error ? e.message : String(e)}` }));
    return;
  }

  aplicar(motor.fundoDefinir(fundoPreferido()));
  ativarAba('inicio');
  vigiarArquivo();
  vigiarIa();

  /** A IA mexeu no desenho (MCP ao vivo): sem aba de desenho aberta, abre uma para ele; com aba, só se atualiza. */
  function receberDaIa(e: Estado): void {
    if (temDesenho()) {
      aplicar(e);
      return;
    }
    guardarEstadoDaAbaAtual();
    const id = `tab_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
    // sem `bytes`: o desenho já está no motor (com o histórico); recarregá-lo apagaria o Ctrl+Z
    abasDoc.push({ id, nome: e.arquivo?.nome ?? `${t.app.semTitulo} ${contadorSemTitulo++}`, caminho: e.arquivo?.caminho, formato: e.arquivo?.formato as 'dxf' | 'fcdz' | undefined, sujo: e.sujo });
    atual = e;
    ativarAba(id);
  }

  /** Pergunta ao Rust, várias vezes por segundo, se a IA mexeu no desenho e se está trabalhando. */
  function vigiarIa(): void {
    if (!noApp()) return;
    let ocupado = false;
    setInterval(() => {
      if (ocupado || atual.arrastando) return;
      ocupado = true;
      mcpVivoVerificar()
        .then((r) => {
          titulo.ia(r.ativa);
          if (r.estado) receberDaIa(r.estado);
        })
        .catch(() => undefined)
        .finally(() => {
          ocupado = false;
        });
    }, 300);
  }

  function vigiarArquivo(): void {
    if (!noApp()) return;
    let ocupado = false;
    setInterval(() => {
      if (ocupado || !temDesenho() || atual.ferramenta !== 'nenhuma' || atual.arrastando) return;
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
