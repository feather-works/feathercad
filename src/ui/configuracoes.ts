/**
 * A janela de Configurações — a mesma do FeatherSheet: modal central e arrastável, seções
 * numa coluna à esquerda, uma linha por opção (rótulo, dica, controle). Não bloqueia a
 * tela: dá para ver o efeito no desenho enquanto se mexe. **Nada aqui decide coisa do
 * desenho**: as opções do motor (snap, orto, fundo, apelidos) só são repassadas a ele; o
 * resto é preferência da interface, guardada em `preferencias.ts`.
 */

import type { Estado, Motor } from '../ponte/index.ts';
import { apelidosDoUsuario, codigoAtual, comandosPadraoDe, guardarApelidosDoUsuario, guardarIdioma, IDIOMAS, regionalDe, comandosDe, formatar, type Textos } from '../idiomas/index.ts';
import { abrirJanelaMonitor, mcpEstado, mcpTestar, noApp, type EstadoDoMcp } from '../ponte/nativo.ts';
import { devConsole } from './console-depuracao.ts';
import { ACENTOS, ANGULOS_DO_ORTO, CORPOS, FONTES, acentoPreferido, aplicarAparencia, corpoPreferido, fontePreferida, formatoPadrao, guardarAparencia, guardarCota, guardarOrtoAngulo, guardarFormatoPadrao, guardarFps, guardarFundo, fpsLigado, guardarZoomSuave, zoomSuave, guardarTextoNoMovimento, textoNoMovimento, guardarPintarEmSegundoPlano, pintarEmSegundoPlano } from './preferencias.ts';
import { definirModo, modoSalvo, type ModoDoTema } from './tema.ts';
import type { Fundo, Unidade } from '../ponte/tipos.ts';
import { Z } from './z.ts';

export const CONFIGURACOES_CSS = `
.cfg-modal { position: fixed; top: 70px; left: calc(50vw - 410px); z-index: ${Z.menu}; display: flex; width: min(820px, calc(100vw - 40px)); height: min(600px, calc(100vh - 100px)); border-radius: 14px; background: var(--fundo-painel); border: 1px solid var(--borda); box-shadow: var(--sombra); overflow: hidden; }
.cfg-nav { flex: none; width: 196px; padding: 18px 10px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid var(--borda); background: var(--fundo-faixa); }
.cfg-nav-titulo { padding: 0 10px 14px; font-size: 16px; font-weight: 500; cursor: grab; user-select: none; }
.cfg-nav button { display: flex; align-items: center; gap: 10px; padding: 9px 11px; border: 0; border-radius: 8px; background: none; color: var(--texto-suave); font-size: 13.5px; text-align: left; cursor: pointer; }
.cfg-nav button:hover { background: var(--fundo-painel); }
.cfg-nav button[aria-current='true'] { background: var(--destaque-suave); color: var(--texto); font-weight: 500; }
.cfg-principal { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.cfg-cabeca { display: flex; align-items: center; gap: 10px; padding: 16px 18px 12px; border-bottom: 1px solid var(--borda); cursor: grab; user-select: none; }
.cfg-cabeca strong { flex: 1; font-size: 15px; font-weight: 500; }
.cfg-fechar { width: 30px; height: 30px; border: 0; border-radius: 50%; background: none; color: var(--texto-suave); font-size: 15px; cursor: pointer; }
.cfg-fechar:hover { background: var(--fundo-faixa); color: var(--texto); }
.cfg-corpo { flex: 1; overflow-y: auto; padding: 6px 18px 22px; scrollbar-width: thin; }
.cfg-linha { display: flex; align-items: center; gap: 16px; padding: 13px 0; border-bottom: 1px solid var(--borda); }
.cfg-linha:last-child { border-bottom: 0; }
.cfg-texto { flex: 1; min-width: 0; }
.cfg-rotulo { font-size: 13.5px; }
.cfg-dica { margin-top: 3px; color: var(--texto-suave); font-size: 12px; line-height: 1.5; }
.cfg-seg { display: inline-flex; flex: none; padding: 3px; border: 1px solid var(--borda); border-radius: 18px; }
.cfg-seg button { padding: 5px 11px; border: 0; border-radius: 14px; background: none; color: var(--texto-suave); font-size: 12.5px; white-space: nowrap; cursor: pointer; }
.cfg-seg button[aria-pressed='true'] { background: var(--destaque); color: var(--fundo-area); }
.cfg-interruptor { flex: none; position: relative; width: 40px; height: 22px; }
.cfg-interruptor input { position: absolute; opacity: 0; width: 100%; height: 100%; margin: 0; cursor: pointer; }
.cfg-interruptor span { position: absolute; inset: 0; border-radius: 11px; background: var(--borda); transition: background 120ms; pointer-events: none; }
.cfg-interruptor span::after { content: ''; position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: var(--fundo-area); transition: transform 120ms; }
.cfg-interruptor input:checked + span { background: var(--destaque); }
.cfg-interruptor input:checked + span::after { transform: translateX(18px); }
.cfg-interruptor input:focus-visible + span { outline: 2px solid var(--destaque); outline-offset: 2px; }
.cfg-cores { display: flex; gap: 8px; flex: none; }
.cfg-cor { width: 24px; height: 24px; padding: 0; border-radius: 50%; border: 2px solid transparent; background: var(--destaque); cursor: pointer; }
.cfg-cor[aria-pressed='true'] { border-color: var(--texto); box-shadow: 0 0 0 2px var(--fundo-painel) inset; }
.cfg-botao { flex: none; padding: 6px 14px; border: 1px solid var(--borda); border-radius: 8px; background: var(--fundo-faixa); color: var(--texto); font-size: 12.5px; cursor: pointer; }
.cfg-botao:hover { border-color: var(--destaque); }
.cfg-botao.principal { background: var(--destaque); border-color: var(--destaque); color: var(--fundo-area); }
.cfg-duplo { display: flex; gap: 8px; flex: none; }
.cfg-campo { flex: none; width: 150px; padding: 5px 8px; border: 1px solid var(--borda); border-radius: 6px; background: var(--fundo-area); color: var(--texto); font: inherit; }
.cfg-campo:focus { outline: 1px solid var(--destaque); }
.cfg-campo.erro { border-color: var(--osnap); }
.cfg-nota { margin: 14px 0 0; color: var(--texto-suave); font-size: 12.5px; line-height: 1.6; }
.cfg-codigo { margin-top: 8px; padding: 8px 10px; background: var(--fundo-area); border: 1px solid var(--borda); border-radius: 8px; font: 11.5px ui-monospace, Consolas, monospace; overflow-wrap: anywhere; white-space: pre-wrap; user-select: all; }
.cfg-estado { display: inline-block; margin-left: 8px; padding: 1px 8px; border-radius: 999px; font-size: 11.5px; background: var(--fundo-faixa); border: 1px solid var(--borda); }
.cfg-estado.ok { color: var(--icone-ok); border-color: var(--icone-ok); }
.cfg-estado.falta { color: var(--osnap); border-color: var(--osnap); }
.cfg-caminho { margin-top: 6px; color: var(--texto-suave); font: 11.5px ui-monospace, Consolas, monospace; overflow-wrap: anywhere; }
`;

type Secao = 'aparencia' | 'desenho' | 'salvamento' | 'idioma' | 'apelidos' | 'mcp' | 'sobre';
const SECOES: Secao[] = ['aparencia', 'desenho', 'salvamento', 'idioma', 'apelidos', 'mcp', 'sobre'];

/** O que a janela precisa do resto do app (ela não conhece o `app.ts`). */
export interface AcoesDeConfiguracao {
  motor: Motor;
  estado: () => Estado;
  /** Repassa um estado novo do motor a todas as peças. */
  aplicar: (e: Estado) => void;
  temDesenho: () => boolean;
  salvar: (como: boolean) => void;
}

export interface Configuracoes {
  el: HTMLElement;
  alternar(secao?: Secao): void;
  visivel(): boolean;
  /** O estado do motor mudou (atalho F8 etc.): redesenha se estiver aberta. */
  atualizar(): void;
}

function linha(rotulo: string, dica: string, controle?: HTMLElement): HTMLElement {
  const el = document.createElement('div');
  el.className = 'cfg-linha';
  const texto = document.createElement('div');
  texto.className = 'cfg-texto';
  const r = document.createElement('div');
  r.className = 'cfg-rotulo';
  r.textContent = rotulo;
  const d = document.createElement('div');
  d.className = 'cfg-dica';
  d.textContent = dica;
  texto.append(r, d);
  el.append(texto);
  if (controle) el.append(controle);
  return el;
}

function seg(opcoes: [string, string][], atual: string, aoEscolher: (v: string) => void): HTMLElement {
  const el = document.createElement('div');
  el.className = 'cfg-seg';
  for (const [valor, rotulo] of opcoes) {
    const b = document.createElement('button');
    b.textContent = rotulo;
    b.setAttribute('aria-pressed', valor === atual ? 'true' : 'false');
    b.addEventListener('click', () => aoEscolher(valor));
    el.append(b);
  }
  return el;
}

function interruptor(ligado: boolean, aoMudar: (v: boolean) => void): HTMLElement {
  const el = document.createElement('label');
  el.className = 'cfg-interruptor';
  const caixa = document.createElement('input');
  caixa.type = 'checkbox';
  caixa.checked = ligado;
  caixa.addEventListener('change', () => aoMudar(caixa.checked));
  el.append(caixa, document.createElement('span'));
  return el;
}

function botao(rotulo: string, aoClicar: () => void, principal = false): HTMLButtonElement {
  const b = document.createElement('button');
  b.className = principal ? 'cfg-botao principal' : 'cfg-botao';
  b.textContent = rotulo;
  b.addEventListener('click', aoClicar);
  return b;
}

/** Deixa `janela` arrastável pelos `puxadores` (fica onde soltar, dentro da tela). */
function arrastavel(janela: HTMLElement, puxadores: HTMLElement[]): void {
  for (const p of puxadores) {
    p.addEventListener('pointerdown', (ev) => {
      if ((ev.target as HTMLElement).closest('button')) return;
      const r = janela.getBoundingClientRect();
      const dx = ev.clientX - r.left;
      const dy = ev.clientY - r.top;
      const mover = (e: PointerEvent): void => {
        janela.style.left = `${Math.min(Math.max(0, e.clientX - dx), window.innerWidth - 80)}px`;
        janela.style.top = `${Math.min(Math.max(0, e.clientY - dy), window.innerHeight - 40)}px`;
      };
      const soltar = (): void => {
        window.removeEventListener('pointermove', mover);
        window.removeEventListener('pointerup', soltar);
      };
      window.addEventListener('pointermove', mover);
      window.addEventListener('pointerup', soltar);
    });
  }
}

/** Cria a janela (escondida); `alternar()` abre e fecha. */
export function criarConfiguracoes(t: Textos, a: AcoesDeConfiguracao): Configuracoes {
  const c = t.config;
  const modal = document.createElement('div');
  modal.className = 'cfg-modal';
  modal.hidden = true;
  modal.setAttribute('role', 'dialog');
  modal.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Escape') esconder();
  });

  const nav = document.createElement('nav');
  nav.className = 'cfg-nav';
  const navTitulo = document.createElement('div');
  navTitulo.className = 'cfg-nav-titulo';
  navTitulo.textContent = c.titulo;
  nav.append(navTitulo);
  let secao: Secao = 'aparencia';
  for (const id of SECOES) {
    const b = document.createElement('button');
    b.dataset['secao'] = id;
    b.textContent = c.secoes[id];
    b.addEventListener('click', () => {
      secao = id;
      desenhar();
    });
    nav.append(b);
  }

  const principal = document.createElement('div');
  principal.className = 'cfg-principal';
  const cabeca = document.createElement('div');
  cabeca.className = 'cfg-cabeca';
  const titulo = document.createElement('strong');
  const fechar = document.createElement('button');
  fechar.className = 'cfg-fechar';
  fechar.textContent = '✕';
  fechar.title = c.fechar;
  fechar.setAttribute('aria-label', c.fechar);
  fechar.addEventListener('click', () => esconder());
  cabeca.append(titulo, fechar);
  const corpo = document.createElement('div');
  corpo.className = 'cfg-corpo';
  principal.append(cabeca, corpo);
  modal.append(nav, principal);
  arrastavel(modal, [navTitulo, cabeca]);

  const esconder = (): void => {
    modal.hidden = true;
  };
  const motor = (e: Estado): void => a.aplicar(e);

  const aparencia = (): void => {
    const cores = document.createElement('div');
    cores.className = 'cfg-cores';
    for (const id of ACENTOS) {
      const b = document.createElement('button');
      b.className = 'cfg-cor';
      if (id !== 'azul') b.dataset['acento'] = id;
      b.title = c.cores[id];
      b.setAttribute('aria-label', c.cores[id]);
      b.setAttribute('aria-pressed', acentoPreferido() === id ? 'true' : 'false');
      b.addEventListener('click', () => {
        guardarAparencia('acento', id);
        aplicarAparencia();
        desenhar();
      });
      cores.append(b);
    }
    corpo.append(
      linha(c.tema.rotulo, c.tema.dica, seg([['auto', c.tema.auto], ['light', c.tema.claro], ['dark', c.tema.escuro]], modoSalvo(), (v) => { definirModo(v as ModoDoTema); desenhar(); })),
      linha(c.destaque.rotulo, c.destaque.dica, cores),
      linha(c.fonte.rotulo, c.fonte.dica, seg(FONTES.map((f) => [f, c.fonte[f]]), fontePreferida(), (v) => { guardarAparencia('fonte', v); aplicarAparencia(); desenhar(); })),
      linha(c.corpo.rotulo, c.corpo.dica, seg(CORPOS.map((n) => [String(n), String(n)]), String(corpoPreferido()), (v) => { guardarAparencia('corpo', v); aplicarAparencia(); desenhar(); })),
      linha(c.fundo.rotulo, c.fundo.dica, seg([['nada', c.fundo.nada], ['pontos', c.fundo.pontos], ['grade', c.fundo.grade]], a.estado().fundo, (v) => { guardarFundo(v as Fundo); motor(a.motor.fundoDefinir(v as Fundo)); desenhar(); })),
    );
  };

  /** Automático ou um número: o motor guarda; a preferência lembra na próxima largada. */
  const tamanhoDaCota = (atual: number | null): HTMLElement => {
    const caixa = document.createElement('div');
    caixa.className = 'cfg-duplo';
    const campo = document.createElement('input');
    campo.className = 'cfg-campo';
    campo.style.width = '90px';
    campo.type = 'text';
    campo.inputMode = 'decimal';
    campo.placeholder = c.cota.automatico;
    campo.value = atual === null ? '' : String(atual).replace('.', regionalDe().decimal);
    const definir = (tamanho: number | null): void => {
      guardarCota(tamanho);
      motor(a.motor.cotaTamanhoDefinir(tamanho));
    };
    campo.addEventListener('change', () => {
      const n = Number(campo.value.trim().replace(regionalDe().decimal, '.'));
      if (campo.value.trim() === '') definir(null);
      else if (Number.isFinite(n) && n > 0) definir(n);
      else campo.classList.add('erro');
      if (campo.value.trim() === '' || (Number.isFinite(n) && n > 0)) campo.classList.remove('erro');
    });
    const auto = botao(c.cota.automatico, () => { definir(null); desenhar(); });
    caixa.append(campo, auto);
    return caixa;
  };

  /** Passos prontos (90, 60, 45, 30, 15) e um campo para qualquer outro ângulo. */
  const anguloDoOrto = (atual: number): HTMLElement => {
    const caixa = document.createElement('div');
    caixa.className = 'cfg-duplo';
    const definir = (g: number): void => {
      guardarOrtoAngulo(g);
      motor(a.motor.ortoAnguloDefinir(g));
      desenhar();
    };
    const campo = document.createElement('input');
    campo.className = 'cfg-campo';
    campo.style.width = '64px';
    campo.type = 'text';
    campo.inputMode = 'decimal';
    campo.placeholder = c.orto.outro;
    campo.title = c.orto.outro;
    campo.value = (ANGULOS_DO_ORTO as readonly number[]).includes(atual) ? '' : String(atual).replace('.', regionalDe().decimal);
    campo.addEventListener('change', () => {
      const n = Number(campo.value.trim().replace(regionalDe().decimal, '.'));
      if (Number.isFinite(n) && n > 0 && n <= 180) definir(n);
      else campo.classList.add('erro');
    });
    caixa.append(seg(ANGULOS_DO_ORTO.map((g) => [String(g), `${g}°`]), String(atual), (v) => definir(Number(v))), campo);
    return caixa;
  };

  const desenho = (): void => {
    const e = a.estado();
    if (a.temDesenho()) {
      corpo.append(linha(t.unidade.rotulo, e.unidade ? t.unidade.dica : t.unidade.semUnidade, seg((['mm', 'cm', 'm'] as const).map((u) => [u, u]), e.unidade ?? '', (v) => { motor(a.motor.unidadeDefinir(v as Unidade)); desenhar(); })));
    }
    corpo.append(linha(c.orto.rotulo, c.orto.dica, anguloDoOrto(e.orto_angulo)));
    corpo.append(linha(c.cota.rotulo, formatar(c.cota.dica, { unidade: e.unidade ?? '' }), tamanhoDaCota(e.cota_tamanho)));
    for (const nome of ['snap', 'orto', 'osnap'] as const) {
      corpo.append(linha(c.desenho[nome].rotulo, c.desenho[nome].dica, interruptor(e[nome], (v) => motor(a.motor.interruptor(nome, v)))));
    }
    corpo.append(linha(c.zoomSuave.rotulo, c.zoomSuave.dica, interruptor(zoomSuave(), (v) => guardarZoomSuave(v))));
    corpo.append(linha(c.segundoPlano.rotulo, c.segundoPlano.dica, interruptor(pintarEmSegundoPlano(), (v) => guardarPintarEmSegundoPlano(v))));
    corpo.append(linha(c.textoNoMovimento.rotulo, c.textoNoMovimento.dica, interruptor(textoNoMovimento(), (v) => guardarTextoNoMovimento(v))));
    corpo.append(linha(c.fundo.rotulo, c.fundo.dica, seg([['nada', c.fundo.nada], ['pontos', c.fundo.pontos], ['grade', c.fundo.grade]], e.fundo, (v) => { guardarFundo(v as Fundo); motor(a.motor.fundoDefinir(v as Fundo)); desenhar(); })));
  };

  const salvamento = (): void => {
    const e = a.estado();
    const nome = e.arquivo?.nome;
    const botoes = document.createElement('div');
    botoes.className = 'cfg-duplo';
    const agora = botao(c.salvamento.agora, () => { a.salvar(false); desenhar(); }, true);
    const como = botao(c.salvamento.como, () => a.salvar(true));
    agora.disabled = como.disabled = !a.temDesenho();
    botoes.append(agora, como);
    corpo.append(
      linha(c.salvamento.rotulo, nome ? formatar(c.salvamento.aberto, { nome }) : c.salvamento.nenhum, botoes),
      linha(c.salvamento.formato.rotulo, c.salvamento.formato.dica, seg([['dxf', '.dxf'], ['fcdz', '.fcdz']], formatoPadrao(), (v) => { guardarFormatoPadrao(v as 'dxf' | 'fcdz'); desenhar(); })),
    );
    const nota = document.createElement('p');
    nota.className = 'cfg-nota';
    nota.textContent = c.salvamento.nota;
    corpo.append(nota);
  };

  const idioma = (): void => {
    const atual = codigoAtual();
    const opcoes = Object.keys(IDIOMAS).map((cod): [string, string] => [cod, (IDIOMAS[cod] as { idioma?: { nome?: string } }).idioma?.nome ?? cod]);
    const r = regionalDe(atual);
    corpo.append(
      linha(c.idioma.rotulo, formatar(c.idioma.dica, { decimal: r.decimal, separador: r.separador_de_coordenadas, casas: r.casas }), seg(opcoes, atual, (v) => {
        if (v === atual) return;
        guardarIdioma(v);
        location.reload();
      })),
    );
    const nota = document.createElement('p');
    nota.className = 'cfg-nota';
    nota.textContent = c.idioma.nota;
    corpo.append(nota);
  };

  const apelidos = (): void => {
    const padrao = comandosPadraoDe().comandos;
    const meus = apelidosDoUsuario();
    const restaurar = botao(c.apelidos.restaurar, () => {
      guardarApelidosDoUsuario({});
      motor(a.motor.idioma(regionalDe(), comandosDe()));
      desenhar();
    });
    corpo.append(linha(c.apelidos.rotulo, c.apelidos.dica, restaurar));
    for (const [id, nomes] of Object.entries(padrao)) {
      const campo = document.createElement('input');
      campo.className = 'cfg-campo';
      campo.placeholder = c.apelidos.exemplo;
      campo.value = (meus[id] ?? []).join(', ');
      campo.spellcheck = false;
      campo.addEventListener('change', () => {
        const novos = campo.value.split(/[\s,;]+/).map((x) => x.trim().toUpperCase()).filter(Boolean);
        // Um apelido só pode apontar para um comando: recusa o que já é de outro.
        const todos = comandosDe().comandos;
        const dono = (ap: string): string | undefined => Object.entries(todos).find(([o, ns]) => o !== id && ns.some((n) => n.toUpperCase() === ap))?.[0];
        const repetido = novos.find((ap) => dono(ap));
        campo.classList.toggle('erro', repetido !== undefined);
        if (repetido !== undefined) {
          campo.title = formatar(c.apelidos.conflito, { apelido: repetido, comando: comandosDe().comandos[dono(repetido)!]![0]! });
          return;
        }
        campo.title = '';
        const todosMeus = { ...apelidosDoUsuario(), [id]: novos.filter((n) => !nomes.includes(n)) };
        guardarApelidosDoUsuario(todosMeus);
        motor(a.motor.idioma(regionalDe(), comandosDe()));
      });
      corpo.append(linha(nomes[0]!, formatar(c.apelidos.padrao, { lista: nomes.slice(1).join(', ') || '—' }), campo));
    }
  };

  /** Copia para a área de transferência do sistema e avisa no próprio botão. */
  const copiar = (rotulo: string, valor: string): HTMLButtonElement => {
    const b = botao(rotulo, () => {
      void navigator.clipboard?.writeText(valor).then(() => {
        b.textContent = c.mcp.copiado;
        setTimeout(() => (b.textContent = rotulo), 1600);
      });
    });
    return b;
  };

  /** Uma linha com um bloco de código embaixo do texto (o comando ou o JSON para copiar). */
  const linhaComCodigo = (rotulo: string, dica: string, codigo: string, controle: HTMLElement): HTMLElement => {
    const l = linha(rotulo, dica, controle);
    l.querySelector('.cfg-texto')?.append(Object.assign(document.createElement('div'), { className: 'cfg-codigo', textContent: codigo }));
    return l;
  };

  let vezDoMcp = 0;
  const mcpLinhas = (info: EstadoDoMcp | null): HTMLElement[] => {
    const m = c.mcp;
    const caminho = info?.caminho ?? m.caminhoGenerico;
    const testar = botao(m.testar, () => void 0);
    const resultado = Object.assign(document.createElement('div'), { className: 'cfg-dica' });
    testar.disabled = !info?.existe;
    testar.addEventListener('click', () => {
      testar.disabled = true;
      resultado.textContent = m.testando;
      void mcpTestar()
        .then((r) => (resultado.textContent = r.ok ? formatar(m.testeOk, { n: r.ferramentas }) : formatar(m.testeErro, { motivo: r.mensagem })))
        .catch((e: unknown) => (resultado.textContent = formatar(m.testeErro, { motivo: String(e) })))
        .finally(() => (testar.disabled = false));
    });
    const programa = linha(m.programa, info === null ? m.semApp : info.existe ? m.encontrado : m.naoEncontrado, testar);
    const estado = Object.assign(document.createElement('span'), { className: `cfg-estado ${info?.existe ? 'ok' : 'falta'}`, textContent: info?.existe ? m.estadoOk : m.estadoFalta });
    programa.querySelector('.cfg-rotulo')?.append(estado);
    programa.querySelector('.cfg-texto')?.append(Object.assign(document.createElement('div'), { className: 'cfg-caminho', textContent: caminho }), resultado);
    const comando = `claude mcp add feathercad -- "${caminho}"`;
    const json = JSON.stringify({ mcpServers: { feathercad: { command: caminho, args: [] as string[] } } }, null, 2);
    return [programa, linhaComCodigo(m.claude, m.claudeDica, comando, copiar(m.copiar, comando)), linhaComCodigo(m.outros, m.outrosDica, json, copiar(m.copiar, json))];
  };

  const mcp = (): void => {
    const nota = Object.assign(document.createElement('p'), { className: 'cfg-nota', textContent: c.mcp.texto });
    nota.style.marginTop = '10px';
    corpo.append(nota);
    const vez = ++vezDoMcp;
    const mostrar = (info: EstadoDoMcp | null): void => {
      if (vez !== vezDoMcp || secao !== 'mcp' || modal.hidden) return;
      corpo.append(...mcpLinhas(info));
      corpo.append(linha(c.mcp.ferramentas, c.mcp.ferramentasLista), Object.assign(document.createElement('p'), { className: 'cfg-nota', textContent: c.mcp.privacidade }));
    };
    if (noApp()) void mcpEstado().then(mostrar).catch(() => mostrar(null));
    else mostrar(null);
  };

  const sobre = (): void => {
    const p = document.createElement('p');
    p.className = 'cfg-nota';
    p.textContent = c.sobre.texto;
    const monitor = botao(c.sobre.abrirMonitor, () => {
      esconder();
      void abrirJanelaMonitor().then((aberta) => {
        if (!aberta) devConsole.toggle();
      });
    });
    const medidor = linha(c.sobre.fps.rotulo, c.sobre.fps.dica, interruptor(fpsLigado(), (v) => guardarFps(v)));
    corpo.append(linha(c.sobre.monitor, c.sobre.monitorDica, monitor), medidor, p);
  };

  function desenhar(): void {
    for (const b of nav.querySelectorAll<HTMLElement>('button[data-secao]')) b.setAttribute('aria-current', b.dataset['secao'] === secao ? 'true' : 'false');
    titulo.textContent = c.secoes[secao];
    corpo.replaceChildren();
    ({ aparencia, desenho, salvamento, idioma, apelidos, mcp, sobre })[secao]();
  }

  return {
    el: modal,
    alternar(s) {
      if (s) secao = s;
      if (!modal.hidden && !s) return esconder();
      desenhar();
      modal.hidden = false;
    },
    visivel: () => !modal.hidden,
    atualizar() {
      // Não redesenha debaixo de quem está digitando um apelido.
      const digitando = document.activeElement instanceof HTMLInputElement && document.activeElement.type === 'text';
      if (!modal.hidden && !digitando) desenhar();
    },
  };
}
