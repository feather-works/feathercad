import { test } from 'node:test';
import assert from 'node:assert/strict';
import { motorNovo } from '../motor-wasm.ts';
import type { Estado } from '../../src/ponte/index.ts';
import { texto } from '../../src/idiomas/index.ts';
import { registrarAvisos } from '../../src/ui/avisos.ts';

test('motor: estado inicial limpo, com o prompt da língua', () => {
  const m = motorNovo();
  const e = m.estado();
  assert.equal(e.sujo, false);
  assert.equal(e.ferramenta, 'nenhuma');
  assert.equal(texto(e.prompt.chave, e.prompt.vars), 'Digite um comando');
});

test('motor: L na linha de comando (apelido do pt-BR) inicia a ferramenta e desenha', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  assert.equal(m.digitar('L').ferramenta, 'linha');
  assert.equal(m.executar('desfazer').ferramenta, 'linha', 'botão usa o nome interno, sem a tabela');
  m.clique(vista, 400, 300);
  m.clique(vista, 500, 300);
  const e = m.estado();
  assert.equal(e.sujo, true);
  const q = m.quadro(vista, null);
  assert.equal(q.tipo, 'novo');
  if (q.tipo === 'novo') {
    assert.equal(q.cores.length, 1);
    assert.deepEqual([...q.xs], [400, 300, 500, 300]);
  }
});

test('motor: no en-US o mesmo apelido L é LINE, e LINHA não existe', () => {
  const m = motorNovo('en-US');
  assert.equal(m.digitar('LINE').ferramenta, 'linha');
  m.cancelar();
  const e = m.digitar('LINHA');
  const ultima = e.historico.at(-1);
  assert.ok(ultima && 'mensagem' in ultima && ultima.mensagem.chave === 'comando.desconhecido');
});

test('motor: quadro diz "igual" quando nada mudou e o TypeScript não recalcula nada', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.linhaCriar({ x: 0, y: 0 }, { x: 10, y: 0 });
  const q = m.quadro(vista, null);
  assert.equal(q.tipo, 'novo');
  if (q.tipo !== 'novo') return;
  assert.equal(m.quadro(vista, q.versao).tipo, 'igual');
  m.linhaCriar({ x: 0, y: 0 }, { x: 0, y: 10 });
  assert.equal(m.quadro(vista, q.versao).tipo, 'novo');
});

test('motor: cursor devolve ponto resolvido e o texto pronto, com o decimal da língua', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.interruptor('snap', true);
  const r = m.cursorMover(vista, 403.2, 296.9);
  assert.equal(r.texto_x, '0,000');
  assert.equal(r.texto_y, '0,000');
  assert.equal(r.texto_distancia, null);
  const en = motorNovo('en-US');
  en.interruptor('snap', false);
  assert.equal(en.cursorMover(vista, 410.5, 300).texto_x, '10.500');
});

test('motor: orto (F8) trava o cursor e mostra distância e ângulo', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.executar('linha');
  m.clique(vista, 400, 300);
  assert.equal(m.atalho('F8').tratado, true);
  const r = m.cursorMover(vista, 460, 305);
  assert.equal(r.y, 300, 'o orto travou o Y na altura do primeiro ponto');
  assert.equal(r.texto_distancia, '60,000');
  assert.equal(r.texto_angulo, '0,000');
  assert.ok(r.previa && r.guias.length === 1);
});

test('motor: atalhos do AutoCAD — Ctrl+Z, Ctrl+Y, Esc, F7, F3; tecla desconhecida não é tratada', () => {
  const m = motorNovo();
  m.linhaCriar({ x: 0, y: 0 }, { x: 1, y: 1 });
  assert.equal(m.atalho('Ctrl+Z').estado.pode_desfazer, false);
  assert.equal(m.atalho('Ctrl+Y').estado.pode_desfazer, true);
  assert.equal(m.estado().fundo, 'grade', 'o fundo nasce em grade');
  assert.equal(m.atalho('F7').estado.fundo, 'nada', 'F7 desliga o fundo');
  assert.equal(m.atalho('F7').estado.fundo, 'grade', 'e volta ao estilo de antes');
  assert.equal(m.atalho('F9').estado.snap, true, 'F9 é o snap');
  assert.equal(m.atalho('F3').estado.osnap, false);
  m.digitar('L');
  assert.equal(m.atalho('Esc').estado.ferramenta, 'nenhuma');
  assert.equal(m.atalho('Ctrl+Shift+F12').tratado, false);
});

test('motor: coordenada digitada na língua (x;y e @x;y)', () => {
  const m = motorNovo();
  m.digitar('L');
  m.digitar('0;0');
  m.digitar('@10,5;0');
  assert.equal(m.estado().historico.length, 3);
  const q = m.quadro(m.vistaInicial(800, 600), null);
  assert.equal(q.tipo === 'novo' ? q.cores.length : -1, 1);
});

test('motor: zoom mantém o ponto sob o cursor e pan acompanha o mouse (contas do Rust)', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  const z = m.vistaZoom(v, 200, 150, 4);
  assert.equal(z.escala, 4);
  const antes = m.cursorMover(v, 200, 150);
  const depois = m.cursorMover(z, 200, 150);
  assert.equal(depois.texto_x, antes.texto_x, 'o ponto sob o cursor não anda com o zoom');
  assert.equal(depois.texto_y, antes.texto_y);
  const p = m.vistaDeslocar(v, 50, 0);
  assert.equal(m.cursorMover(p, 450, 300).texto_x, m.cursorMover(v, 400, 300).texto_x, 'o desenho acompanha o mouse no pan');
});

test('motor: pedido malformado e comando desconhecido dão mensagem em português', () => {
  const m = motorNovo();
  assert.throws(() => m.executar('teletransportar'), /não tem o comando/);
  assert.throws(() => m.interruptor('inexistente' as 'grade', true), /não existe o interruptor/);
  assert.throws(() => m['canal'].chamar('teletransportar'), /comando desconhecido na ponte/);
});

test('motor: toda chave de tecla do Rust está na tabela docs/ATALHOS.md', async () => {
  const { readFileSync } = await import('node:fs');
  const doc = readFileSync(new URL('../../docs/ATALHOS.md', import.meta.url), 'utf8').toUpperCase();
  const rs = readFileSync(new URL('../../crates/feathercad-core/src/atalhos.rs', import.meta.url), 'utf8');
  const teclas = [...rs.matchAll(/\("([^"]+)", Acao::/g)].map((m) => m[1]!);
  assert.ok(teclas.length >= 10);
  for (const t of teclas) {
    if (t === 'ESCAPE') continue; // sinônimo de ESC
    // "Ctrl+[" e "Ctrl+\" aparecem como "Ctrl+[ , Ctrl+\" na tabela
    assert.ok(doc.includes(t.replace('CTRL+\\\\', 'CTRL+\\')) || doc.includes(t), `a tecla ${t} não está em docs/ATALHOS.md`);
  }
});

test('motor: todo comando da ponte tem chamada no TypeScript (sem comando órfão)', async () => {
  const { readFileSync } = await import('node:fs');
  const rs = readFileSync(new URL('../../crates/feathercad-ponte/src/lib.rs', import.meta.url), 'utf8');
  const ts = readFileSync(new URL('../../src/ponte/index.ts', import.meta.url), 'utf8');
  const lista = rs.match(/COMANDOS_DA_PONTE: \[&str; \d+\] = \[([^\]]*)\]/)![1]!;
  for (const nome of [...lista.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]!)) {
    assert.ok(ts.includes(`'${nome}'`), `a ponte tem \`${nome}\` mas o Motor (index.ts) não chama`);
  }
});

test('motor: o fundo (nada, pontos, grade) muda o quadro e o passo se adapta ao zoom', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  const grade = m.quadro(v, null);
  assert.equal(grade.tipo, 'novo');
  if (grade.tipo !== 'novo') return;
  assert.ok(grade.fundoX.pos.length > 3 && grade.fundoY.pos.length > 3);
  assert.ok(grade.fundoX.principal.includes(1), 'há linhas principais');
  assert.ok([...grade.fundoX.pos].some((x) => Math.abs(x - 400) < 1e-3), 'a linha da origem está no centro');
  const perto = m.quadro(m.vistaZoom(v, 400, 300, 20), null);
  assert.equal(perto.tipo === 'novo' ? perto.fundoX.pos.length > 0 : false, true);
  m.fundoDefinir('pontos');
  assert.equal(m.estado().fundo, 'pontos');
  m.fundoDefinir('nada');
  const vazio = m.quadro(v, null);
  assert.equal(vazio.tipo === 'novo' ? vazio.fundoX.pos.length + vazio.fundoY.pos.length : -1, 0);
});

test('motor: zoom para dentro/fora em volta do centro, enquadrar o desenho e o texto do zoom (tudo do Rust)', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  assert.equal(m.vistaTexto(v), '100%');
  const mais = m.vistaZoomCentro(v, 2);
  assert.equal(m.vistaTexto(mais), '200%');
  assert.deepEqual(mais.centro, v.centro, 'o centro fica parado');
  assert.deepEqual(m.vistaAjustar(v), v, 'sem desenho, enquadrar volta ao início');
  m.linhaCriar({ x: 1000, y: 1000 }, { x: 1100, y: 1050 });
  const a = m.vistaAjustar(v);
  assert.deepEqual(a.centro, { x: 1050, y: 1025, z: 0 });
  assert.ok(a.escala > 1 && a.escala < 10);
  const en = motorNovo('en-US');
  assert.equal(en.vistaTexto(en.vistaZoomCentro(en.vistaInicial(800, 600), 1.5)), '150%');
});

/** O desenho de exemplo: duas linhas, uma delas na camada 0. */
function comLinhas() {
  const m = motorNovo();
  m.linhaCriar({ x: 0, y: 0 }, { x: 10, y: 5 });
  m.linhaCriar({ x: -3, y: 2 }, { x: 4, y: 4.5 });
  return m;
}

test('motor: gravar em DXF e em .fcdz e reabrir dá o mesmo desenho (bytes atravessam a ponte crus)', () => {
  const origem = comLinhas();
  const vista = origem.vistaInicial(800, 600);
  const quadroOrigem = origem.quadro(vista, null);
  for (const formato of ['dxf', 'fcdz'] as const) {
    const bytes = origem.documentoGravar(formato);
    assert.ok(bytes.length > 100);
    const destino = motorNovo();
    const e = destino.documentoAbrir(bytes);
    assert.equal(e.sujo, false, 'aberto nasce limpo');
    assert.equal(e.arquivo, null, 'o disco é quem diz o caminho');
    const q = destino.quadro(vista, null);
    assert.equal(q.tipo === 'novo' ? q.cores.length : -1, 2, formato);
    assert.deepEqual(q.tipo === 'novo' ? [...q.xs] : [], quadroOrigem.tipo === 'novo' ? [...quadroOrigem.xs] : [], formato);
  }
  assert.equal(String.fromCharCode(...origem.documentoGravar('fcdz').subarray(0, 2)), 'PK');
  assert.match(new TextDecoder().decode(origem.documentoGravar('dxf')), /AC1009/);
});

test('motor: o que ficou de fora ao abrir vai para a linha de comando como aviso traduzível', () => {
  const m = motorNovo();
  const dxf = '0\nSECTION\n2\nENTITIES\n0\nSPLINE\n8\n0\n10\n0\n20\n0\n40\n1\n0\nENDSEC\n0\nEOF\n';
  const e = m.documentoAbrir(new TextEncoder().encode(dxf));
  const ultima = e.historico.at(-1);
  assert.ok(ultima && 'mensagem' in ultima);
  if (ultima && 'mensagem' in ultima) {
    assert.equal(ultima.mensagem.chave, 'aviso.entidadesIgnoradas');
    assert.match(texto(ultima.mensagem.chave, ultima.mensagem.vars), /1 objeto\(s\).*1 SPLINE/);
    assert.match(texto(ultima.mensagem.chave, ultima.mensagem.vars, 'en-US'), /1 object\(s\).*1 SPLINE/);
  }
  assert.throws(() => m.documentoAbrir(new TextEncoder().encode('lixo\n')), /linha 1/);
});

test('motor: arquivo do disco — nome, salvo limpa o sujo, novo esvazia', () => {
  const m = comLinhas();
  assert.equal(m.estado().sujo, true);
  m.documentoGravar('dxf');
  const e = m.documentoArquivoDefinir('/casa/plantas/térreo.dxf', 'dxf', true);
  assert.equal(e.sujo, false);
  assert.deepEqual(e.arquivo, { caminho: '/casa/plantas/térreo.dxf', nome: 'térreo.dxf', formato: 'dxf' });
  assert.deepEqual(m.documentoArquivo(), e.arquivo);
  m.desfazer();
  assert.equal(m.estado().sujo, true, 'mexeu depois de salvar');
  const novo = m.documentoNovo();
  assert.deepEqual([novo.arquivo, novo.sujo, novo.pode_desfazer], [null, false, false]);
  assert.equal(m.quadro(m.vistaInicial(800, 600), null).tipo === 'novo' ? (m.quadro(m.vistaInicial(800, 600), null) as { cores: Uint32Array }).cores.length : -1, 0);
  assert.throws(() => m.documentoArquivoDefinir('/x.dwg', 'dwg' as 'dxf', false), /formato/);
});

test('motor: Ctrl+N/O/S/Shift+S pedem ação de janela (o motor escolhe qual, a interface executa)', () => {
  const m = motorNovo();
  assert.equal(m.atalho('Ctrl+O').janela, 'abrir');
  assert.equal(m.atalho('Ctrl+S').janela, 'salvar');
  assert.equal(m.atalho('Ctrl+Shift+S').janela, 'salvar_como');
  assert.equal(m.atalho('Ctrl+N').janela, 'novo');
  assert.equal(m.atalho('F8').janela, null);
});

test('motor: os avisos do último abrir/salvar ficam no estado (para irem ao console) e o novo limpa', () => {
  const m = motorNovo();
  const dxf = '0\nSECTION\n2\nENTITIES\n0\nXLINE\n8\n0\n0\nMESH\n8\n0\n0\nMESH\n8\n0\n0\nENDSEC\n0\nEOF\n';
  const e = m.documentoAbrir(new TextEncoder().encode(dxf));
  assert.deepEqual(e.avisos_do_arquivo.map((a) => a.chave), ['aviso.entidadesIgnoradas']);
  assert.equal(e.avisos_do_arquivo[0]!.vars['tipos'], '2 MESH, 1 XLINE');
  const linhas: string[] = [];
  registrarAvisos(e, 'planta.dxf', (msg) => linhas.push(msg));
  assert.equal(linhas.length, 1);
  assert.match(linhas[0]!, /^\[arquivo\] planta\.dxf: 3 objeto\(s\) ficaram de fora.*2 MESH, 1 XLINE/);
  assert.deepEqual(m.documentoNovo().avisos_do_arquivo, []);
  m.linhaCriar({ x: 0, y: 0 }, { x: 1, y: 1 });
  assert.deepEqual(m.estado().avisos_do_arquivo, [], 'um comando comum não repete o aviso antigo como novo');
});

test('avisos: um estado sem o campo (motor antigo) não quebra o registro', () => {
  const linhas: string[] = [];
  registrarAvisos({} as Estado, 'x.dxf', (m) => linhas.push(m));
  assert.deepEqual(linhas, []);
});

test('polilinha: PL, pontos, Fechar (F) e Enter criam uma entidade só, com a prévia em lista', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  assert.equal(m.digitar('PL').ferramenta, 'polilinha');
  assert.equal(texto(m.estado().prompt.chave, {}), 'PLINHA Especifique o ponto inicial:');
  m.clique(v, 400, 300);
  m.clique(v, 500, 300);
  m.clique(v, 500, 200);
  assert.equal(m.cursorMover(v, 450, 200).previa.length, 3, 'dois trechos marcados + o último até o cursor');
  m.digitar('F');
  const e = m.estado();
  assert.equal(e.ferramenta, 'nenhuma');
  const q = m.quadro(v, null);
  assert.equal(q.tipo === 'novo' ? q.cores.length : -1, 3, 'fechada: 3 trechos para 3 vértices');
  assert.equal(m.desfazer().pode_desfazer, false, 'foi UMA entidade só');
});

test('polilinha: Enter termina, Esc guarda o que foi marcado (2+ pontos), 1 ponto só não cria nada, poucos pontos não fecha', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  const contar = (): number => {
    const q = m.quadro(v, null);
    return q.tipo === 'novo' ? q.cores.length : -1;
  };
  m.executar('polilinha');
  m.clique(v, 400, 300);
  m.clique(v, 500, 300);
  m.digitar('');
  assert.equal(contar(), 1);
  m.executar('polilinha');
  m.clique(v, 100, 100);
  m.clique(v, 150, 100);
  assert.equal(m.atalho('Esc').estado.ferramenta, 'nenhuma');
  assert.equal(contar(), 2, 'Esc mantém a polilinha já marcada');
  m.executar('polilinha');
  m.clique(v, 10, 10);
  m.digitar('');
  assert.equal(contar(), 2, 'um ponto só não vira polilinha');
  m.executar('polilinha');
  m.clique(v, 10, 10);
  m.clique(v, 20, 20);
  const msg = m.digitar('F').historico.at(-1);
  assert.ok(msg && 'mensagem' in msg && msg.mensagem.chave === 'polilinha.poucosPontos');
  assert.equal(m.estado().ferramenta, 'polilinha', 'continua marcando');
});

test('polilinha: no en-US a opção é Close (C), e F não fecha (F é FILLET)', () => {
  const m = motorNovo('en-US');
  const v = m.vistaInicial(800, 600);
  m.digitar('PLINE');
  for (const [x, y] of [[100, 100], [200, 100], [200, 200]] as const) m.clique(v, x, y);
  assert.equal(m.digitar('Q').ferramenta, 'polilinha', 'Q não é opção nem comando: cai em ponto inválido');
  assert.equal(m.digitar('C').ferramenta, 'nenhuma');
});

test('texto e polilinha vindos do DXF chegam ao quadro (texto já em pixels, com giro e alinhamento)', () => {
  const m = motorNovo();
  const dxf = '0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\n0\n10\n0\n20\n0\n40\n20\n1\nCozinha\n50\n90\n72\n1\n73\n2\n11\n0\n21\n0\n0\nLWPOLYLINE\n8\n0\n90\n2\n70\n0\n10\n0\n20\n0\n42\n1\n10\n100\n20\n0\n0\nENDSEC\n0\nEOF\n';
  const e = m.documentoAbrir(new TextEncoder().encode(dxf));
  assert.deepEqual(e.avisos_do_arquivo, []);
  const q = m.quadro(m.vistaInicial(800, 600), null);
  assert.equal(q.tipo, 'novo');
  if (q.tipo !== 'novo') return;
  assert.equal(q.textos.length, 1);
  const t = q.textos[0]!;
  assert.deepEqual([t.conteudo, t.h, t.base, t.altura], ['Cozinha', 'center', 'middle', 20]);
  assert.ok(Math.abs(t.giro + Math.PI / 2) < 1e-4, 'o giro de 90° vira -π/2 no canvas (Y para baixo)');
  assert.deepEqual([t.x, t.y], [400, 300]);
  assert.ok(q.cores.length > 8, 'o arco (bulge 1 = meia-volta) virou várias cordas');
});

test('círculo: C, centro e raio digitado ou clicado; a prévia é o círculo inteiro em pixels', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  assert.equal(m.digitar('C').ferramenta, 'circulo');
  m.clique(v, 400, 300);
  const previa = m.cursorMover(v, 460, 300).previa;
  assert.ok(previa.length > 15, `um círculo de 60 px vira dezenas de cordas, deu ${previa.length}`);
  assert.equal(m.cursorMover(v, 460, 300).texto_distancia, '60,000', 'a distância na entrada dinâmica é o raio');
  assert.equal(m.digitar('40').ferramenta, 'nenhuma');
  const q = m.quadro(v, null);
  assert.ok(q.tipo === 'novo' && q.cores.length > 15);
  assert.equal(m.estado().pode_desfazer, true);
});

test('arco: A, três pontos; os alinhados avisam e continuam pedindo', () => {
  const m = motorNovo();
  const v = m.vistaInicial(800, 600);
  m.digitar('A');
  m.clique(v, 500, 300);
  m.clique(v, 400, 200);
  assert.ok(m.cursorMover(v, 300, 300).previa.length > 10, 'prévia do arco');
  m.clique(v, 300, 300);
  assert.equal(m.estado().ferramenta, 'nenhuma');
  const q = m.quadro(v, null);
  assert.ok(q.tipo === 'novo' && q.cores.length > 10);
  m.executar('arco');
  m.clique(v, 0, 0);
  m.clique(v, 10, 10);
  const e = m.clique(v, 20, 20);
  const ultima = e.historico.at(-1);
  assert.ok(ultima && 'mensagem' in ultima && ultima.mensagem.chave === 'arco.alinhados');
  assert.equal(e.ferramenta, 'arco');
});

test('círculo e arco vindos do DXF chegam ao quadro e o aviso some (extrusão -Z espelha)', () => {
  const m = motorNovo();
  const dxf = '0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n50\n0\nARC\n8\n0\n10\n0\n20\n0\n40\n80\n50\n0\n51\n90\n0\nENDSEC\n0\nEOF\n';
  const e = m.documentoAbrir(new TextEncoder().encode(dxf));
  assert.deepEqual(e.avisos_do_arquivo, []);
  const q = m.quadro(m.vistaInicial(800, 600), null);
  assert.ok(q.tipo === 'novo' && q.cores.length > 20);
});

test('motor: o Inserir lista no estado os blocos do desenho (sem os anônimos) e só enquanto espera o nome', () => {
  const m = motorNovo();
  const bloco = (nome: string) => `0\nBLOCK\n8\n0\n2\n${nome}\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n0\nENDBLK\n`;
  const dxf = `0\nSECTION\n2\nBLOCKS\n${bloco('porta')}${bloco('*D1')}${bloco('janela')}0\nENDSEC\n0\nEOF\n`;
  assert.deepEqual(m.documentoAbrir(new TextEncoder().encode(dxf)).blocos, []);
  assert.deepEqual(m.executar('inserir').blocos, ['porta', 'janela']);
  const e = m.digitar('janela');
  assert.equal(e.ferramenta, 'inserir');
  assert.deepEqual(e.blocos, [], 'com o bloco escolhido a lista some');
});

test('edição: clicar seleciona (o quadro muda e o realce vem azul), mover em 2 cliques, Ctrl+Z desfaz', () => {
  const m = motorNovo();
  m.linhaCriar({ x: 0, y: 0 }, { x: 100, y: 0 });
  const v = m.vistaAjustar(m.vistaInicial(800, 600));
  const antes = m.quadro(v, null);
  assert.equal(antes.tipo, 'novo');
  if (antes.tipo !== 'novo') return;
  const meio = { x: (antes.xs[0]! + antes.xs[2]!) / 2, y: (antes.xs[1]! + antes.xs[3]!) / 2 };
  assert.equal(m.clique(v, meio.x, meio.y).selecionados, 1);
  const depois = m.quadro(v, antes.versao);
  assert.equal(depois.tipo, 'novo', 'só a seleção mudou, mas o quadro precisa ser refeito');
  if (depois.tipo === 'novo') assert.equal(depois.cores[0], 0x3b82f6, 'realce da seleção');
  assert.equal(m.executar('mover').ferramenta, 'mover');
  m.clique(v, meio.x, meio.y);
  const e = m.clique(v, meio.x, meio.y + 40);
  assert.equal(e.ferramenta, 'nenhuma');
  assert.equal(e.selecionados, 0);
  const movido = m.quadro(v, null);
  if (movido.tipo === 'novo') assert.ok(Math.abs(movido.xs[1]! - (antes.xs[1]! + 40)) < 1e-3, 'a linha desceu 40 px');
  assert.ok(m.desfazer().pode_refazer);
  const volta = m.quadro(v, null);
  if (volta.tipo === 'novo') assert.ok(Math.abs(volta.xs[1]! - antes.xs[1]!) < 1e-3);
});

test('edição: os apelidos digitados chegam aos comandos novos nas duas línguas', () => {
  for (const [codigo, nomes] of [['pt-BR', ['MOVER', 'M', 'ESPELHAR', 'APARAR', 'CONCORDANCIA', 'RET', 'H']], ['en-US', ['MOVE', 'M', 'MIRROR', 'TRIM', 'FILLET', 'REC', 'H']]] as const) {
    for (const nome of nomes) {
      const m = motorNovo(codigo);
      assert.notEqual(m.digitar(nome).ferramenta, 'nenhuma', `${codigo}: ${nome}`);
    }
  }
});

test('camadas: a tabela vem do motor (filtro, busca, ordem) e cada ação é desfeita com um Ctrl+Z', () => {
  const m = motorNovo();
  const c = { filtro: 'todas', busca: '', ordem: 'nome', crescente: true } as const;
  const nova = (nome: string) => m.camadaAcao({ acao: 'nova', nome, consulta: c });
  nova('Camada10');
  nova('Camada2');
  const r = nova('Paredes');
  assert.equal(r.erro, null);
  assert.deepEqual(r.tabela.linhas.map((l) => l.nome), ['0', 'Camada2', 'Camada10', 'Paredes'], 'ordem natural, feita no Rust');
  assert.deepEqual(m.camadas({ ...c, busca: 'camada*' }).linhas.map((l) => l.nome), ['Camada2', 'Camada10']);
  assert.deepEqual([m.camadas(c).exibidas, m.camadas(c).totais], [4, 4]);
  assert.equal(nova('paredes').erro, 'camadas.erro.nomeRepetido');
  const i = r.tabela.linhas.find((l) => l.nome === 'Paredes')!.indice;
  const cong = m.camadaAcao({ acao: 'propriedade', indices: [i], propriedade: 'bloqueada', valor: true, consulta: c });
  assert.equal(cong.tabela.linhas.find((l) => l.indice === i)!.bloqueada, true);
  assert.equal(m.camadaAcao({ acao: 'propriedade', indices: [i], propriedade: 'espessura', valor: 0.5, consulta: c }).tabela.linhas.find((l) => l.indice === i)!.espessura, 0.5);
  assert.equal(m.camadaAcao({ acao: 'atual', indices: [i], consulta: c }).tabela.atual, 'Paredes');
  assert.equal(m.camadaAcao({ acao: 'propriedade', indices: [i], propriedade: 'congelada', valor: true, consulta: c }).erro, 'camadas.erro.camadaAtual');
  assert.equal(m.camadaAcao({ acao: 'excluir', indices: [i], consulta: c }).erro, 'camadas.erro.camadaAtual');
  m.desfazer();
  assert.equal(m.camadas(c).linhas.find((l) => l.indice === i)!.espessura, null, 'o Ctrl+Z desfez a espessura');
});

test('propriedades: o resumo dos selecionados e a troca de cor, tipo de linha e camada de uma vez', () => {
  const m = motorNovo();
  m.linhaCriar({ x: 0, y: 0 }, { x: 100, y: 0 });
  const v = m.vistaAjustar(m.vistaInicial(800, 600));
  const q = m.quadro(v, null);
  if (q.tipo !== 'novo') throw new Error('quadro');
  assert.equal(m.propriedadesSelecao().n, 0);
  m.clique(v, (q.xs[0]! + q.xs[2]!) / 2, (q.xs[1]! + q.xs[3]!) / 2);
  let r = m.propriedadesSelecao();
  assert.deepEqual([r.n, r.cor, r.tipo_linha], [1, { estado: 'igual', valor: 'porcamada' }, { estado: 'igual', valor: 'porcamada' }]);
  assert.equal(r.cores_basicas.length, 7);
  m.propriedadesDefinir('cor', '#ff0000');
  m.propriedadesDefinir('tipo_linha', 'DASHED');
  m.camadaAcao({ acao: 'nova', nome: 'Cotas', consulta: { filtro: 'todas', busca: '', ordem: 'nome', crescente: true } });
  m.propriedadesDefinir('camada', 'Cotas');
  r = m.propriedadesSelecao();
  assert.deepEqual([r.cor, r.tipo_linha, r.camada], [{ estado: 'igual', valor: '#ff0000' }, { estado: 'igual', valor: 'DASHED' }, { estado: 'igual', valor: 'Cotas' }]);
  assert.equal(m.propriedadesDefinir('cor', 'verde').historico.at(-1) !== undefined, true, 'cor inválida vira mensagem no histórico');
  const antes = m.quadro(v, null);
  assert.equal(antes.tipo, 'novo');
  if (antes.tipo === 'novo') assert.ok(antes.cores.length > 1, 'o tracejado vira vários segmentos');
});

test('unidade: o Novo já nasce com a que foi escolhida; trocar depois suja; abrir sem unidade fica sem', () => {
  const m = motorNovo();
  const novo = m.documentoNovo('cm');
  assert.equal(novo.unidade, 'cm');
  assert.equal(novo.sujo, false);
  const trocado = m.unidadeDefinir('m');
  assert.equal(trocado.unidade, 'm');
  assert.equal(trocado.sujo, true);
  const volta = m.documentoAbrir(m.documentoGravar('dxf'));
  assert.equal(volta.unidade, 'm');
  assert.equal(m.documentoNovo().unidade, null);
  assert.throws(() => m.unidadeDefinir('pol' as never));
});

test('distância digitada: o número vale na unidade do desenho, na direção do cursor; com sufixo converte', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.documentoNovo('cm');
  m.digitar('L');
  m.clique(vista, 400, 300);
  // Sem mexer o cursor não há direção: o motor avisa e nada é criado.
  m.digitar('10');
  const ultima = m.estado().historico.at(-1)!;
  assert.ok('mensagem' in ultima && ultima.mensagem.chave === 'comando.semDirecao');
  m.cursorMover(vista, 400, 200);
  m.digitar('10');
  m.cursorMover(vista, 400, 250);
  m.digitar('2m'); // 2 m = 200 cm, na unidade do desenho
  m.digitar('');
  const q = m.quadro(vista, null);
  assert.equal(q.tipo, 'novo');
  if (q.tipo === 'novo') assert.deepEqual([...q.xs], [400, 300, 400, 290, 400, 290, 400, 90]);
});

test('distância digitada: raio do círculo aceita unidade (50mm em desenho em cm = 5)', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.documentoNovo('cm');
  m.digitar('C');
  m.clique(vista, 400, 300);
  m.digitar('50mm');
  assert.equal(m.estado().ferramenta, 'nenhuma');
  const q = m.quadro(vista, null);
  if (q.tipo === 'novo') assert.ok(q.xs.length > 0);
});

test('tamanho da cota: o motor guarda, null volta ao automático, zero ou negativo é recusado', () => {
  const m = motorNovo();
  assert.equal(m.estado().cota_tamanho, null);
  assert.equal(m.cotaTamanhoDefinir(12.5).cota_tamanho, 12.5);
  assert.throws(() => m.cotaTamanhoDefinir(0));
  assert.throws(() => m.cotaTamanhoDefinir(-3));
  assert.equal(m.estado().cota_tamanho, 12.5);
  assert.equal(m.cotaTamanhoDefinir(null).cota_tamanho, null);
});

test('grab: arrastar um objeto o move (um Ctrl+Z desfaz) e o botão direito seleciona e monta o menu do momento', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.linhaCriar({ x: 0, y: 0 }, { x: 100, y: 0 });
  // Botão direito sobre a linha: ela vira a seleção e o menu oferece mover e apagar.
  const r = m.menuContexto(vista, 450, 300);
  assert.equal(r.estado.selecionados, 1);
  const ids = r.itens.filter((i) => !i.separador).map((i) => i.id);
  assert.ok(ids.includes('mover') && ids.includes('apagar'));
  // Grab: pega em (450,300) e solta 50 px acima.
  m.digitar('');
  const pegou = m.arrastoIniciar(vista, 450, 300);
  assert.equal(pegou.arrastando, true);
  const fim = m.arrastoFim(vista, 450, 250);
  assert.equal(fim.arrastando, false);
  assert.equal(fim.sujo, true);
  const q = m.quadro(vista, null);
  if (q.tipo === 'novo') assert.deepEqual([...q.xs], [400, 250, 500, 250]);
  m.executar('desfazer');
  // No vazio o grab não pega nada e o menu de "nada" oferece as configurações.
  assert.equal(m.arrastoIniciar(vista, 10, 10).arrastando, false);
  m.cancelar(); // Esc limpa a seleção que o grab deixou
  const vazio = m.menuContexto(vista, 10, 10);
  assert.ok(vazio.itens.some((i) => i.id === 'janela:configuracoes'));
});

test('motor: a lista de comandos que a interface espera é exatamente a da ponte, e o motor novo não falta nada', async () => {
  const { readFileSync } = await import('node:fs');
  const { COMANDOS_DA_PONTE_ESPERADOS } = await import('../../src/ponte/index.ts');
  const rs = readFileSync(new URL('../../crates/feathercad-ponte/src/lib.rs', import.meta.url), 'utf8');
  const lista = rs.match(/COMANDOS_DA_PONTE: \[&str; \d+\] = \[([^\]]*)\]/)![1]!;
  const doRust = [...lista.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]!);
  assert.deepEqual([...COMANDOS_DA_PONTE_ESPERADOS], doRust);
  assert.deepEqual(motorNovo().comandosQueFaltam(), []);
});

test('grab ao vivo: o quadro principal esconde os arrastados e quadro_arrasto os desenha deslocados (texto inclusive)', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.linhaCriar({ x: 0, y: 0 }, { x: 100, y: 0 });
  m.linhaCriar({ x: 0, y: 50 }, { x: 100, y: 50 });
  const antes = m.quadro(vista, null);
  assert.ok(antes.tipo === 'novo' && antes.cores.length === 2);
  assert.equal(m.arrastoIniciar(vista, 450, 300).arrastando, true);
  // Só a linha pegada some do quadro principal (a outra fica).
  const durante = m.quadro(vista, null);
  assert.ok(durante.tipo === 'novo' && durante.cores.length === 1);
  const vivo = m.quadroArrasto(vista, 450, 250);
  assert.ok(vivo.tipo === 'novo');
  if (vivo.tipo === 'novo') assert.deepEqual([...vivo.xs], [400, 250, 500, 250]);
  m.arrastoFim(vista, 450, 250);
  const depois = m.quadro(vista, null);
  assert.ok(depois.tipo === 'novo' && depois.cores.length === 2, 'ao soltar voltam os dois');
});

test('seleção por arrasto: esquerda→direita só pega o que cabe inteiro (janela), direita→esquerda basta tocar (cruzando); o motor diz qual é', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.linhaCriar({ x: -100, y: 0 }, { x: 100, y: 0 }); // atravessa a janela abaixo
  m.linhaCriar({ x: -10, y: 20 }, { x: 10, y: 20 }); // cabe dentro
  // Mouse desce no vazio (canto 1) e a janela abre; o cursor à direita = janela (azul).
  const aberto = m.clique(vista, 350, 200);
  assert.equal(aberto.janela_pendente, true);
  assert.equal(m.cursorMover(vista, 450, 340).janela, 'dentro');
  assert.equal(m.cursorMover(vista, 250, 340).janela, 'toca');
  // Soltar à direita: só a linha curta (cabe inteira), a longa não.
  assert.equal(m.clique(vista, 450, 340).selecionados, 1);
  m.cancelar();
  // Da direita para a esquerda: as duas (a longa é só tocada).
  m.clique(vista, 450, 200);
  assert.equal(m.clique(vista, 350, 340).selecionados, 2);
});

test('seleção cruzando: círculo só é pego se o traço toca a janela, não se a janela está dentro dele', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.digitar('C');
  m.digitar('0;0');
  m.digitar('100');
  // Janela pequena no meio do círculo (direita→esquerda): não toca o traço.
  m.clique(vista, 420, 320);
  assert.equal(m.clique(vista, 380, 280).selecionados, 0);
  // Janela que cruza a borda do círculo (x=100 → pixel 500): pega.
  m.clique(vista, 520, 320);
  assert.equal(m.clique(vista, 480, 280).selecionados, 1);
});

test('orto com passo de ângulo: 90 trava nos eixos; 45, 30 e 15 travam nesses múltiplos; ângulo inválido é recusado', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.interruptor('osnap', false);
  m.interruptor('orto', true);
  m.digitar('L');
  m.clique(vista, 400, 300); // origem
  // 90° (padrão): mouse em (500, 310) trava no eixo X.
  let r = m.cursorMover(vista, 500, 310);
  assert.ok(Math.abs(r.y - 300) < 1e-6 && Math.abs(r.x - 500) < 1e-6);
  // 45°: mouse perto da diagonal (500, 190) → cai em (x, y) com |dx| = |dy|.
  assert.equal(m.ortoAnguloDefinir(45).orto_angulo, 45);
  r = m.cursorMover(vista, 500, 190);
  assert.ok(Math.abs(Math.abs(r.x - 400) - Math.abs(r.y - 300)) < 1e-6, `diagonal: ${r.x},${r.y}`);
  // 30°: mouse a ~28° do eixo X trava em 30° (tan 30° ≈ 0,577).
  m.ortoAnguloDefinir(30);
  r = m.cursorMover(vista, 500, 300 - 53);
  assert.ok(Math.abs((300 - r.y) / (r.x - 400) - Math.tan(Math.PI / 6)) < 1e-6);
  // 15°: 100 px na horizontal com o mouse a 10° vai para 15°.
  m.ortoAnguloDefinir(15);
  r = m.cursorMover(vista, 500, 300 - 17.6);
  assert.ok(Math.abs((300 - r.y) / (r.x - 400) - Math.tan(Math.PI / 12)) < 1e-6);
  assert.throws(() => m.ortoAnguloDefinir(0));
  assert.throws(() => m.ortoAnguloDefinir(200));
  assert.equal(m.estado().orto_angulo, 15);
});

test('polar como no AutoCAD: @10<90, 10<0 (da origem) e <45 travando a direção da próxima distância', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.digitar('L');
  m.digitar('0;0');
  m.digitar('@10<90'); // 10 para cima
  m.digitar('@10<180'); // 10 para a esquerda
  m.digitar('20<0'); // absoluto: (20, 0), da origem
  m.digitar('');
  const q = m.quadro(vista, null);
  assert.equal(q.tipo, 'novo');
  if (q.tipo === 'novo') assert.deepEqual([...q.xs].map((v) => Math.round(v * 1000) / 1000), [400, 300, 400, 290, 400, 290, 390, 290, 390, 290, 420, 300]);
  // <45 trava a direção; o cursor só diz para que lado; 10 sai em 45°.
  m.digitar('L');
  m.clique(vista, 400, 300);
  const travou = m.digitar('<45');
  const ultima = travou.historico.at(-1)!;
  assert.ok('mensagem' in ultima && ultima.mensagem.chave === 'comando.anguloTravado');
  const r = m.cursorMover(vista, 500, 250); // perto de 45° (x cresce, y sobe): fica na reta de 45°
  assert.ok(Math.abs((300 - r.y) - (r.x - 400)) < 1e-6);
  m.digitar('10');
  m.digitar('');
  const q2 = m.quadro(vista, null);
  if (q2.tipo === 'novo') {
    const ultimo = [...q2.xs].slice(-4);
    const d = 10 / Math.SQRT2;
    assert.ok(Math.abs(ultimo[2]! - (400 + d)) < 1e-3 && Math.abs(ultimo[3]! - (300 - d)) < 1e-3, `${ultimo}`);
  }
  // Em pt-BR o decimal é vírgula: 2,5<30.
  m.digitar('L');
  m.digitar('0;0');
  m.digitar('@2,5<0');
  m.digitar('');
});

test('motor: tipos de snap ao objeto são configuráveis (meio da linha liga e desliga)', () => {
  const m = motorNovo();
  const vista = m.vistaInicial(800, 600);
  m.executar('linha');
  m.clique(vista, 400, 300);
  m.clique(vista, 500, 300);
  m.cancelar();
  assert.equal(m.estado().osnap_modos.meio, true, 'o meio vem ligado de fábrica');
  const perto = m.cursorMover(vista, 452, 303);
  assert.equal(perto.marca, true);
  assert.equal(perto.snap, 'meio');
  assert.equal(perto.texto_x, '50,000');
  assert.equal(m.osnapModosDefinir({ meio: false }).osnap_modos.meio, false);
  assert.equal(m.cursorMover(vista, 452, 303).marca, false, 'sem o meio ligado não gruda');
  assert.equal(m.osnapModosDefinir({ proximo: true }).osnap_modos.proximo, true);
  assert.equal(m.cursorMover(vista, 452, 303).snap, 'proximo', 'o mais próximo gruda na própria linha');
  assert.throws(() => m.osnapModosDefinir({ inexistente: true } as never), /não existe/);
});
