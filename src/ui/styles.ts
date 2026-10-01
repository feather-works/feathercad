/** Concatena o CSS de cada componente. Nenhuma folha de CSS global. */

import { ABAS_ARQUIVO_CSS } from './abas-arquivo.ts';
import { AREA_DESENHO_CSS } from './area-desenho.ts';
import { MEDIDOR_FPS_CSS } from './medidor-fps.ts';
import { BARRA_NAVEGACAO_CSS } from './barra-navegacao.ts';
import { ICONES_CSS } from './camadas-icones.ts';
import { CAMADAS_PAINEL_CSS } from './camadas-painel.ts';
import { PROPRIEDADES_PAINEL_CSS } from './propriedades-painel.ts';
import { RIBBON_WIDGETS_CSS } from './ribbon-widgets.ts';
import { VISTA_CUBO_CSS } from './vista-cubo.ts';
import { MENU_CONTEXTO_CSS } from './menu-contexto.ts';
import { CAIXA_UNIDADE_CSS } from './caixa-unidade.ts';
import { CAIXA_EXPORTAR_CSS } from './caixa-exportar.ts';
import { CAIXA_FOLHA_CSS } from './caixa-folha.ts';
import { CONFIGURACOES_CSS } from './configuracoes.ts';
import { CAIXA_CONFIRMAR_CSS } from './caixa-confirmar.ts';
import { PAGINA_INICIAL_CSS } from './pagina-inicial.ts';
import { TOAST_CSS } from './toast.ts';
import { BASE_CSS } from './base.ts';
import { DEBUG_CONSOLE_CSS } from './console-depuracao.ts';
import { LINHA_COMANDO_CSS } from './linha-comando.ts';
import { RIBBON_CSS } from './ribbon.ts';
import { RODAPE_CSS } from './rodape.ts';
import { TITULO_CSS } from './titulo.ts';

export const CSS_DO_APP = [BASE_CSS, TITULO_CSS, RIBBON_CSS, ABAS_ARQUIVO_CSS, PAGINA_INICIAL_CSS, AREA_DESENHO_CSS, MEDIDOR_FPS_CSS, BARRA_NAVEGACAO_CSS, VISTA_CUBO_CSS, ICONES_CSS, CAMADAS_PAINEL_CSS, PROPRIEDADES_PAINEL_CSS, RIBBON_WIDGETS_CSS, LINHA_COMANDO_CSS, RODAPE_CSS, CAIXA_CONFIRMAR_CSS, CAIXA_UNIDADE_CSS, CAIXA_EXPORTAR_CSS, CAIXA_FOLHA_CSS, MENU_CONTEXTO_CSS, CONFIGURACOES_CSS, TOAST_CSS, DEBUG_CONSOLE_CSS].join('\n');

/** Instala o CSS do app na página. */
export function instalarEstilos(): void {
  const estilo = document.createElement('style');
  estilo.textContent = CSS_DO_APP;
  document.head.append(estilo);
}
