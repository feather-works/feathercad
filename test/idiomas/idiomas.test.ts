import { test } from 'node:test';
import assert from 'node:assert/strict';
import { COMANDOS, COMPLETAS, IDIOMAS, REFERENCIA } from '../../src/idiomas/registro.ts';
import { formatar, texto, textos } from '../../src/idiomas/index.ts';
import { motorNovo } from '../motor-wasm.ts';

/** Os caminhos de todas as folhas (texto) de um arquivo: "ribbon.botoes.linha.rotulo". */
function folhas(obj: unknown, prefixo = ''): string[] {
  if (typeof obj !== 'object' || obj === null) return [prefixo];
  return Object.entries(obj).flatMap(([k, v]) => folhas(v, prefixo ? `${prefixo}.${k}` : k));
}

const daReferencia = new Set(folhas(IDIOMAS[REFERENCIA]));

test('idiomas: nenhuma língua tem chave que não existe na referência (erro de digitação)', () => {
  for (const [codigo, arquivo] of Object.entries(IDIOMAS)) {
    const sobrando = folhas(arquivo).filter((c) => !daReferencia.has(c));
    assert.deepEqual(sobrando, [], `${codigo}.json tem chaves que o pt-BR não tem`);
  }
});

test('idiomas: as línguas que o app entrega estão completas', () => {
  for (const codigo of COMPLETAS) {
    const tem = new Set(folhas(IDIOMAS[codigo]));
    assert.deepEqual([...daReferencia].filter((c) => !tem.has(c)), [], `${codigo}.json está sem estas traduções`);
  }
});

test('idiomas: todo arquivo diz o próprio código e a configuração regional inteira', () => {
  for (const [codigo, arquivo] of Object.entries(IDIOMAS)) {
    assert.equal(arquivo.idioma?.codigo, codigo);
    assert.ok(arquivo.idioma?.decimal && arquivo.idioma?.separadorDeCoordenadas && arquivo.idioma?.casas);
    assert.notEqual(arquivo.idioma?.decimal, arquivo.idioma?.separadorDeCoordenadas, `${codigo}: decimal e separador de coordenadas iguais confundem o ponto digitado`);
  }
});

test('idiomas: toda língua tem o arquivo de comandos, e o Rust aceita a tabela', () => {
  const conhecidos = motorNovo().comandos();
  for (const codigo of Object.keys(IDIOMAS)) {
    const tabela = COMANDOS[codigo];
    assert.ok(tabela, `falta comandos.${codigo}.json`);
    assert.deepEqual(Object.keys(tabela.comandos).sort(), [...conhecidos].sort(), `${codigo}: a tabela tem que ter exatamente os comandos do motor`);
    assert.deepEqual(Object.keys(tabela.opcoes), ['fechar', 'solida', 'padrao', 'area', 'distancia'], `${codigo}: as opções de ferramenta`);
    motorNovo(codigo); // o Rust recusa comando que não conhece
  }
});

test('idiomas: texto que falta numa língua vem da referência', () => {
  const g = IDIOMAS as Record<string, unknown>;
  g['xx-TESTE'] = { idioma: { codigo: 'xx-TESTE' }, app: { nome: 'Nome de teste' } };
  try {
    const t = textos('xx-TESTE');
    assert.equal(t.app.nome, 'Nome de teste');
    assert.equal(t.app.semTitulo, textos(REFERENCIA).app.semTitulo);
  } finally {
    delete g['xx-TESTE'];
  }
});

test('idiomas: formatar troca as variáveis e deixa as desconhecidas', () => {
  assert.equal(formatar('Comando {nome} e {n}', { nome: 'X' }), 'Comando X e {n}');
});

test('idiomas: toda chave de mensagem que o Rust pode devolver existe em todas as línguas', () => {
  // As chaves que `sessao.rs` produz (prompt e histórico).
  const chaves = [
    'comando.digite',
    'comando.desconhecido',
    'comando.pontoInvalido',
    'linha.primeiroPonto',
    'linha.proximoPonto',
    'desfazer.feito',
    'desfazer.nada',
    'refazer.feito',
    'refazer.nada',
    'polilinha.primeiroPonto',
    'polilinha.proximoPonto',
    'polilinha.poucosPontos',
    'circulo.centro',
    'circulo.raio',
    'arco.primeiroPonto',
    'arco.segundoPonto',
    'arco.pontoFinal',
    'arco.alinhados',
    'insercao.nome',
    'insercao.ponto',
    'insercao.desconhecido',
    'insercao.semBlocos',
    'edicao.selecionar',
    'edicao.janelaSegundo',
    'edicao.esticarJanela',
    'edicao.base',
    'edicao.destino',
    'edicao.angulo',
    'edicao.fator',
    'edicao.espelho1',
    'edicao.espelho2',
    'edicao.matriz',
    'edicao.matrizInvalida',
    'edicao.nadaSelecionado',
    'edicao.selecionados',
    'edicao.apagados',
    'edicao.alterados',
    'edicao.copiados',
    'edicao.destinoInvalido',
    'edicao.espelhoInvalido',
    'aparar.clique',
    'aparar.semCorte',
    'aparar.naoSuporta',
    'concordancia.primeira',
    'concordancia.segunda',
    'concordancia.paralelas',
    'concordancia.raioGrande',
    'concordancia.soLinhas',
    'concordancia.raioNegativo',
    'retangulo.primeiroCanto',
    'retangulo.outroCanto',
    'retangulo.invalido',
    'elipse.centro',
    'elipse.eixo',
    'elipse.outroEixo',
    'elipse.invalida',
    'hachura.pontoPadrao',
    'hachura.pontoSolida',
    'hachura.semContorno',
    'propriedades.semCamada',
    'propriedades.corInvalida',
    'propriedades.tipoInvalido',
    'propriedades.espessuraInvalida',
    'texto.ponto',
    'texto.altura',
    'texto.conteudo',
    'texto.alturaInvalida',
    'cota.primeiro',
    'cota.segundo',
    'cota.posicao',
    'cota.nula',
    'chamada.ponta',
    'chamada.pouso',
    'chamada.texto',
    'tabela.ponto',
    'tabela.dados',
    'tabela.invalida',
    'medir.primeiro',
    'medir.segundo',
    'medir.areaPonto',
    'medir.distancia',
    'medir.area',
    'medir.poucosPontos',
    'medir.demais',
    'colar.ponto',
    'colar.vazia',
    'colar.feito',
    'tornarAtual.clique',
    'tornarAtual.feito',
    'igualar.origem',
    'igualar.destino',
    'prancheta.copiados',
    'prancheta.recortados',
    'agrupar.criado',
    'agrupar.desfeito',
    'bloco.nome',
    'bloco.criado',
    'bloco.erro.nomeVazio',
    'bloco.erro.nomeInvalido',
    'bloco.erro.nomeRepetido',
    'bloco.erro.semObjetos',
    'edicao.alvoCamada',
    'aviso.entidadesIgnoradas',
    'aviso.espacoPapelIgnorado',
    'aviso.dentroDeBlocos',
    'aviso.hachuraNaoGravada',
    'aviso.hachuraDecomposta',
    'aviso.corAproximada',
    'aviso.espessuraNaoGravada',
  ];
  for (const codigo of COMPLETAS) {
    for (const c of chaves) assert.notEqual(texto(c, { nome: 'x', texto: 'x', n: '1', tipos: 'x' }, codigo), c, `${codigo}: falta ${c}`);
  }
});

test('apelidos do usuário: entram na tabela mandada ao Rust, sem repetir os da língua', async () => {
  const guardado = new Map<string, string>();
  (globalThis as { localStorage?: unknown }).localStorage = {
    getItem: (k: string) => guardado.get(k) ?? null,
    setItem: (k: string, v: string) => void guardado.set(k, v),
  };
  const { comandosDe, comandosPadraoDe, guardarApelidosDoUsuario } = await import('../../src/idiomas/index.ts');
  guardarApelidosDoUsuario({ mover: ['G', 'M'] });
  const mover = comandosDe('pt-BR').comandos['mover']!;
  assert.ok(mover.includes('G'));
  assert.equal(mover.filter((a) => a === 'M').length, 1);
  assert.ok(!comandosPadraoDe('pt-BR').comandos['mover']!.includes('G'));
  delete (globalThis as { localStorage?: unknown }).localStorage;
});

test('menu do botão direito: todo comando do motor tem rótulo em todas as línguas', async () => {
  const motor = (await import('../motor-wasm.ts')).motorNovo();
  for (const codigo of COMPLETAS) {
    for (const c of [...motor.comandos(), 'enter', 'cancelar', 'fechar', 'propriedades', 'zoom_ajustar', 'configuracoes', 'snap', 'orto', 'osnap', 'grade']) {
      assert.notEqual(texto(`menu.item.${c}`, {}, codigo), `menu.item.${c}`, `${codigo}: falta menu.item.${c}`);
    }
    assert.match(texto('menu.repetir', {}, codigo), /\{nome\}/);
  }
});
