import type { Estado, Motor } from '../ponte/index.ts';
import { texto, type Textos } from '../idiomas/index.ts';
import { controleDeCamada, controleDeCor, controleDeEspessura, controleDeTexto, controleDeTipoDeLinha, fonteDePropriedades, type ControleDeTexto } from './propriedades-controles.ts';
import type { CampoDePropriedade, NomeCampoDePropriedade, ResumoDePropriedades } from '../ponte/tipos.ts';
import { Z } from './z.ts';

export const PROPRIEDADES_PAINEL_CSS = `
.prop-painel { position: absolute; right: 64px; top: 16px; width: 268px; z-index: ${Z.paleta}; display: flex; flex-direction: column; background: var(--fundo-painel); border: 1px solid var(--borda); border-radius: 4px; box-shadow: var(--sombra); max-height: calc(100vh - 100px); overflow-y: auto; }
.prop-painel .titulo { display: flex; align-items: center; gap: 8px; padding: 5px 8px; background: var(--fundo-faixa); border-bottom: 1px solid var(--borda); cursor: move; user-select: none; font-weight: 600; position: sticky; top: 0; z-index: 1; }
.prop-painel .titulo .espaco { flex: 1; }
.prop-painel button.fechar { background: none; border: 0; cursor: pointer; font-size: 14px; color: var(--texto-suave); }
.prop-painel .resumo { padding: 6px 10px; color: var(--texto-suave); border-bottom: 1px solid var(--borda); }
.prop-painel .secao { padding: 4px 0 8px; }
.prop-painel .secao .cabeca { padding: 2px 10px; font-weight: 600; background: var(--fundo-faixa); }
.prop-painel .linha { display: grid; grid-template-columns: 100px minmax(0, 1fr); align-items: center; gap: 6px; padding: 3px 10px; }
.prop-painel .linha .rotulo { color: var(--texto-suave); }
.prop-painel select.prop-controle, .prop-painel input.prop-controle-input { width: 100%; height: 22px; background: var(--fundo-area); color: var(--texto); border: 1px solid var(--borda); border-radius: 2px; font: inherit; padding: 0 4px; box-sizing: border-box; }
.prop-painel select.prop-controle:disabled, .prop-painel input.prop-controle-input:disabled { opacity: 0.55; }
`;

export interface PainelDePropriedades {
  el: HTMLElement;
  alternar(): void;
  atualizar(e: Estado): void;
}

export function criarPainelDePropriedades(t: Textos, motor: Motor, aoMudar: (e: Estado) => void): PainelDePropriedades {
  const el = document.createElement('div');
  el.className = 'prop-painel';
  el.hidden = true;
  const resumoDe = fonteDePropriedades(motor);
  const definir = (campo: NomeCampoDePropriedade) => (valor: string) => aoMudar(motor.propriedadesDefinir(campo, valor));

  const titulo = document.createElement('div');
  titulo.className = 'titulo';
  const nome = document.createElement('span');
  nome.textContent = t.propriedades.titulo;
  const espaco = document.createElement('span');
  espaco.className = 'espaco';
  const fechar = document.createElement('button');
  fechar.className = 'fechar';
  fechar.textContent = '✕';
  fechar.title = t.propriedades.fechar;
  fechar.addEventListener('click', () => (el.hidden = true));
  titulo.append(nome, espaco, fechar);

  const resumo = document.createElement('div');
  resumo.className = 'resumo';

  // Seção Geral
  const secaoGeral = document.createElement('div');
  secaoGeral.className = 'secao';
  const cabecaGeral = document.createElement('div');
  cabecaGeral.className = 'cabeca';
  cabecaGeral.textContent = t.propriedades.geral;
  secaoGeral.append(cabecaGeral);
  const controlesGeral = [
    { rotulo: t.propriedades.cor, c: controleDeCor(t, definir('cor')) },
    { rotulo: t.propriedades.camada, c: controleDeCamada(t, motor, definir('camada')) },
    { rotulo: t.propriedades.tipoLinha, c: controleDeTipoDeLinha(t, definir('tipo_linha')) },
    { rotulo: t.propriedades.espessura, c: controleDeEspessura(t, definir('espessura')) },
  ];
  for (const { rotulo, c } of controlesGeral) {
    const linha = document.createElement('div');
    linha.className = 'linha';
    const r = document.createElement('span');
    r.className = 'rotulo';
    r.textContent = rotulo;
    linha.append(r, c.el);
    secaoGeral.append(linha);
  }

  // Seção Geometria e 3D
  const secaoGeometria = document.createElement('div');
  secaoGeometria.className = 'secao';
  const cabecaGeometria = document.createElement('div');
  cabecaGeometria.className = 'cabeca';
  cabecaGeometria.textContent = `${t.propriedades.geometria} / 3D`;
  secaoGeometria.append(cabecaGeometria);
  const controlesGeometria = [
    { rotulo: t.propriedades.posicaoX, prop: 'posicao_x' as const, c: controleDeTexto(t.propriedades.posicaoX, definir('posicao_x')) },
    { rotulo: t.propriedades.posicaoY, prop: 'posicao_y' as const, c: controleDeTexto(t.propriedades.posicaoY, definir('posicao_y')) },
    { rotulo: 'Posição Z (Elevação)', prop: 'posicao_z' as const, c: controleDeTexto('Posição Z', definir('posicao_z')) },
    { rotulo: t.propriedades.comprimento, prop: 'comprimento' as const, c: controleDeTexto(t.propriedades.comprimento, definir('comprimento')) },
    { rotulo: t.propriedades.raio, prop: 'raio' as const, c: controleDeTexto(t.propriedades.raio, definir('raio')) },
    { rotulo: t.propriedades.diametro, prop: 'diametro' as const, c: controleDeTexto(t.propriedades.diametro, definir('diametro')) },
    { rotulo: t.propriedades.area, prop: 'area' as const, c: controleDeTexto(t.propriedades.area, definir('area')) },
    { rotulo: 'Volume 3D', prop: 'volume' as const, c: controleDeTexto('Volume 3D', definir('volume')) },
    { rotulo: 'Altura de Extrusão', prop: 'altura_extrusao' as const, c: controleDeTexto('Altura de Extrusão', definir('altura_extrusao')) },
    { rotulo: 'Dimensão Z (Espessura)', prop: 'dimensao_z' as const, c: controleDeTexto('Dimensão Z', definir('dimensao_z')) },
    { rotulo: t.propriedades.tamanhoGeral, prop: 'tamanho_geral' as const, c: controleDeTexto(t.propriedades.tamanhoGeral, definir('tamanho_geral')) },
    { rotulo: t.propriedades.rotacao, prop: 'rotacao' as const, c: controleDeTexto(t.propriedades.rotacao, definir('rotacao')) },
    { rotulo: 'Rotação X 3D', prop: 'rotacao_x' as const, c: controleDeTexto('Rotação X', definir('rotacao_x')) },
    { rotulo: 'Rotação Y 3D', prop: 'rotacao_y' as const, c: controleDeTexto('Rotação Y', definir('rotacao_y')) },
    { rotulo: 'Rotação Z 3D', prop: 'rotacao_z' as const, c: controleDeTexto('Rotação Z', definir('rotacao_z')) },
  ];
  const linhasGeometria: Array<{ el: HTMLElement; prop: keyof ResumoDePropriedades; c: ControleDeTexto }> = [];
  for (const { rotulo, prop, c } of controlesGeometria) {
    const linha = document.createElement('div');
    linha.className = 'linha';
    const r = document.createElement('span');
    r.className = 'rotulo';
    r.textContent = rotulo;
    linha.append(r, c.el);
    secaoGeometria.append(linha);
    linhasGeometria.push({ el: linha, prop, c });
  }

  // Seção Texto
  const secaoTexto = document.createElement('div');
  secaoTexto.className = 'secao';
  const cabecaTexto = document.createElement('div');
  cabecaTexto.className = 'cabeca';
  cabecaTexto.textContent = t.propriedades.texto;
  secaoTexto.append(cabecaTexto);
  const controlesTexto = [
    { rotulo: t.propriedades.conteudo, prop: 'conteudo' as const, c: controleDeTexto(t.propriedades.conteudo, definir('conteudo')) },
  ];
  const linhasTexto: Array<{ el: HTMLElement; prop: keyof ResumoDePropriedades; c: ControleDeTexto }> = [];
  for (const { rotulo, prop, c } of controlesTexto) {
    const linha = document.createElement('div');
    linha.className = 'linha';
    const r = document.createElement('span');
    r.className = 'rotulo';
    r.textContent = rotulo;
    linha.append(r, c.el);
    secaoTexto.append(linha);
    linhasTexto.push({ el: linha, prop, c });
  }

  el.append(titulo, resumo, secaoGeral, secaoGeometria, secaoTexto);

  titulo.addEventListener('mousedown', (ev) => {
    if ((ev.target as HTMLElement).closest('button')) return;
    const pai = el.offsetParent as HTMLElement | null;
    const [x0, y0, esq, topo] = [ev.clientX, ev.clientY, el.offsetLeft, el.offsetTop];
    const mover = (m: MouseEvent): void => {
      el.style.right = 'auto';
      el.style.left = `${Math.max(0, Math.min((pai?.clientWidth ?? 0) - 60, esq + m.clientX - x0))}px`;
      el.style.top = `${Math.max(0, Math.min((pai?.clientHeight ?? 0) - 30, topo + m.clientY - y0))}px`;
    };
    const soltar = (): void => {
      window.removeEventListener('mousemove', mover);
      window.removeEventListener('mouseup', soltar);
    };
    window.addEventListener('mousemove', mover);
    window.addEventListener('mouseup', soltar);
    ev.preventDefault();
  });
  for (const ev of ['mousedown', 'mousemove', 'wheel', 'dblclick', 'contextmenu'] as const) el.addEventListener(ev, (e) => e.stopPropagation());

  const mostrar = (e: Estado): void => {
    const r = resumoDe(e);
    resumo.textContent = r.n === 0 ? t.propriedades.nenhum : texto('propriedades.objetos', { n: String(r.n) });
    for (const { c } of controlesGeral) c.mostrar(r);

    let temGeometria = false;
    for (const { el: linhaEl, prop, c } of linhasGeometria) {
      const campo = r[prop] as CampoDePropriedade | undefined;
      const tem = campo && campo.estado !== 'vazio';
      linhaEl.hidden = !tem;
      if (tem) {
        c.mostrar(campo, t.propriedades.variado);
        temGeometria = true;
      }
    }
    secaoGeometria.hidden = !temGeometria;

    let temTexto = false;
    for (const { el: linhaEl, prop, c } of linhasTexto) {
      const campo = r[prop] as CampoDePropriedade | undefined;
      const tem = campo && campo.estado !== 'vazio';
      linhaEl.hidden = !tem;
      if (tem) {
        c.mostrar(campo, t.propriedades.variado);
        temTexto = true;
      }
    }
    secaoTexto.hidden = !temTexto;
  };
  return {
    el,
    alternar() {
      el.hidden = !el.hidden;
      if (!el.hidden) mostrar(motor.estado());
    },
    atualizar(e) {
      if (!el.hidden) mostrar(e);
    },
  };
}
