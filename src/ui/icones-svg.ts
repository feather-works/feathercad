/** Ícones SVG vetoriais limpos para o FeatherCAD (substitui emojis e caracteres unicode). */

function svg(body: string, size = 18, viewBox = '0 0 24 24'): string {
  return `<svg width="${size}" height="${size}" viewBox="${viewBox}" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="display:inline-block;vertical-align:middle;">${body}</svg>`;
}

export const ICONES_SVG: Record<string, string> = {
  // Arquivo & Título
  novo: svg('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>'),
  abrir: svg('<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>'),
  salvar: svg('<path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/>'),
  exportar: svg('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M12 18v-6M9 15l3 3 3-3"/>'),
  pdf: svg('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 13h2a1.5 1.5 0 0 0 0-3H9v6M14 10v6M14 10h2a2 2 0 0 1 0 4h-2"/>'),
  dxf: svg('<path d="M4 4h16v16H4z"/><path d="M8 8h3v8H8zM13 8l3 4-3 4M16 8l-3 4 3 4"/>'),
  fcdz: svg('<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/>'),
  desfazer: svg('<path d="M9 14L4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v0a5.5 5.5 0 0 1-5.5 5.5H11"/>'),
  refazer: svg('<path d="M15 14l5-5-5-5"/><path d="M20 9H9.5A5.5 5.5 0 0 0 4 14.5v0A5.5 5.5 0 0 0 9.5 20H13"/>'),
  configuracoes: svg('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>'),
  tema: svg('<path d="M12 21a9 9 0 1 0 0-18v18z" fill="currentColor"/>'),

  // Área de transferência e Edição
  colar: svg('<path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>'),
  copiar: svg('<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>'),
  recortar: svg('<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><line x1="20" y1="4" x2="8.12" y2="15.88"/><line x1="14.47" y1="14.48" x2="20" y2="20"/><line x1="8.12" y1="8.12" x2="12" y2="12"/>'),
  apagar: svg('<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>'),

  // Desenhar
  linha: svg('<line x1="4" y1="20" x2="20" y2="4"/><circle cx="4" cy="20" r="2" fill="currentColor"/><circle cx="20" cy="4" r="2" fill="currentColor"/>'),
  polilinha: svg('<polyline points="3 19 9 7 15 15 21 5"/><circle cx="3" cy="19" r="1.5" fill="currentColor"/><circle cx="9" cy="7" r="1.5" fill="currentColor"/><circle cx="15" cy="15" r="1.5" fill="currentColor"/><circle cx="21" cy="5" r="1.5" fill="currentColor"/>'),
  circulo: svg('<circle cx="12" cy="12" r="8"/><line x1="12" y1="9" x2="12" y2="15"/><line x1="9" y1="12" x2="15" y2="12"/>'),
  arco: svg('<path d="M4 16A10 10 0 0 1 20 16"/><circle cx="4" cy="16" r="1.5" fill="currentColor"/><circle cx="20" cy="16" r="1.5" fill="currentColor"/>'),
  retangulo: svg('<rect x="4" y="6" width="16" height="12" rx="1"/>'),
  elipse: svg('<ellipse cx="12" cy="12" rx="9" ry="5"/>'),
  hachura: svg('<rect x="4" y="4" width="16" height="16" rx="1"/><path d="M4 14l10-10M4 20l16-16M10 20l10-10"/>'),
  hachura_padrao: svg('<rect x="4" y="4" width="16" height="16" rx="1"/><path d="M4 10h16M4 16h16M10 4v16M16 4v16"/>'),

  // Modificar
  mover: svg('<path d="M12 2v20M2 12h20M12 2l-3 3M12 2l3 3M12 22l-3-3M12 22l3-3M2 12l3-3M2 12l3 3M22 12l-3-3M22 12l-3 3"/>'),
  esticar: svg('<rect x="3" y="7" width="10" height="10" stroke-dasharray="2 2"/><path d="M13 12h8M18 9l3 3-3 3"/>'),
  girar: svg('<path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>'),
  espelhar: svg('<path d="M12 3v18" stroke-dasharray="3 3"/><polygon points="4 18 9 18 9 6 4 18"/><polygon points="20 18 15 18 15 6 20 18"/>'),
  escala: svg('<rect x="4" y="12" width="8" height="8"/><rect x="4" y="4" width="16" height="16"/><path d="M12 12l6-6M18 9v-3h-3"/>'),
  aparar: svg('<circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><path d="M8 7.5L19 18.5M8 16.5L19 5.5"/>'),
  deslocar: svg('<path d="M5 6h14M5 12h14M5 18h14" stroke-dasharray="3 3"/>'),
  estender: svg('<path d="M19 4v16M4 12h11M12 9l3 3-3 3"/>'),
  concordancia: svg('<path d="M4 20h8a6 6 0 0 0 6-6V4"/>'),
  matriz: svg('<rect x="4" y="4" width="4" height="4"/><rect x="10" y="4" width="4" height="4"/><rect x="16" y="4" width="4" height="4"/><rect x="4" y="10" width="4" height="4"/><rect x="10" y="10" width="4" height="4"/><rect x="16" y="10" width="4" height="4"/><rect x="4" y="16" width="4" height="4"/><rect x="10" y="16" width="4" height="4"/><rect x="16" y="16" width="4" height="4"/>'),

  // Anotação
  texto: svg('<path d="M6 20L12 4l6 16M8.5 14h7"/>'),
  cota: svg('<path d="M4 12h16M4 6v12M20 6v12M4 12l3-2.5v5z" fill="currentColor"/><path d="M20 12l-3-2.5v5z" fill="currentColor"/>'),
  cota_alinhada: svg('<path d="M5 19L19 5M5 13v6h6M13 5h6v6"/>'),
  cota_raio: svg('<circle cx="12" cy="12" r="8"/><path d="M12 12l6-6M18 6h-4M18 6v4"/>'),
  cota_diametro: svg('<circle cx="12" cy="12" r="8"/><path d="M6 18l12-12M6 18l3-1M6 18l1-3M18 6l-3 1M18 6l-1 3"/>'),
  chamada: svg('<path d="M4 18l10-10h6M4 18l4-1.5M4 18l1.5-4"/>'),
  tabela: svg('<rect x="3" y="4" width="18" height="16" rx="1"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="14" x2="21" y2="14"/><line x1="9" y1="4" x2="9" y2="20"/><line x1="15" y1="4" x2="15" y2="20"/>'),

  // Utilitários & Outros
  medir: svg('<path d="M3 7h18v10H3z"/><line x1="7" y1="7" x2="7" y2="11"/><line x1="11" y1="7" x2="11" y2="10"/><line x1="15" y1="7" x2="15" y2="11"/><line x1="19" y1="7" x2="19" y2="10"/>'),
  contar: svg('<line x1="10" y1="3" x2="8" y2="21"/><line x1="16" y1="3" x2="14" y2="21"/><line x1="4" y1="9" x2="20" y2="9"/><line x1="4" y1="15" x2="20" y2="15"/>'),
  igualar_propriedades: svg('<path d="M18.37 2.63a2.12 2.12 0 0 1 3 3L13.5 13.5l-4 1 1-4 7.87-7.87zM3 21c1.5 0 3-1.5 3-3s-1.5-3-3-3-3 1.5-3 3 1.5 3 3 3z"/>'),
  camadas: svg('<path d="M12 2L2 7l10 5 10-5-10-5zM2 12l10 5 10-5M2 17l10 5 10-5"/>'),
  bloco: svg('<rect x="4" y="4" width="16" height="16" rx="2"/><rect x="8" y="8" width="8" height="8"/>'),
  agrupar: svg('<rect x="3" y="3" width="18" height="18" rx="2" stroke-dasharray="3 3"/><circle cx="8" cy="8" r="2"/><circle cx="16" cy="16" r="2"/>'),

  // Visualização & Modos
  enquadrar: svg('<path d="M4 8V4h4M16 4h4v4M20 16v4h-4M8 20H4v-4"/>'),
  zoom_mais: svg('<circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16" y2="16"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/>'),
  zoom_menos: svg('<circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16" y2="16"/><line x1="8" y1="11" x2="14" y2="11"/>'),
  propriedades: svg('<rect x="4" y="3" width="16" height="18" rx="2"/><line x1="8" y1="7" x2="16" y2="7"/><line x1="8" y1="11" x2="16" y2="11"/><line x1="8" y1="15" x2="13" y2="15"/>'),
  monitor: svg('<rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>'),
  unidades: svg('<polygon points="4 20 4 4 20 20 4 20"/><line x1="4" y1="12" x2="8" y2="12"/><line x1="4" y1="16" x2="10" y2="16"/><line x1="12" y1="20" x2="12" y2="16"/><line x1="16" y1="20" x2="16" y2="14"/>'),
  limpar: svg('<path d="M18 13l-5 5L4 9l5-5 9 9zM2 20h20"/>'),
  mcp: svg('<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9 9h2v2H9zM13 9h2v2h-2zM9 14h6"/>'),
  ia: svg('<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>'),
  impressao: svg('<polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/>'),
  sobre: svg('<circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>'),
  folha: svg('<rect x="4" y="3" width="16" height="18" rx="1"/><path d="M14 15h4v4h-4z"/>'),

  // Paramétrico
  param_coincidente: svg('<circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2" fill="currentColor"/>'),
  param_colinear: svg('<line x1="3" y1="12" x2="10" y2="12"/><line x1="14" y1="12" x2="21" y2="12" stroke-dasharray="2 2"/>'),
  param_concentrico: svg('<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="4"/>'),
  param_paralelo: svg('<line x1="6" y1="19" x2="12" y2="5"/><line x1="12" y1="19" x2="18" y2="5"/>'),
  param_perpendicular: svg('<line x1="4" y1="19" x2="20" y2="19"/><line x1="12" y1="19" x2="12" y2="5"/>'),
  param_tangente: svg('<circle cx="12" cy="14" r="6"/><line x1="4" y1="8" x2="20" y2="8"/>'),
  param_horizontal: svg('<line x1="4" y1="12" x2="20" y2="12"/><polyline points="7 9 4 12 7 15"/><polyline points="17 9 20 12 17 15"/>'),
  param_vertical: svg('<line x1="12" y1="4" x2="12" y2="20"/><polyline points="9 7 12 4 15 7"/><polyline points="9 17 12 20 15 17"/>'),
};

/** Retorna a string SVG do ícone pelo ID ou pelo símbolo de atalho. */
export function obterIconeSVG(id: string, simboloBackup = ''): string {
  // Mapeamentos diretos por id de botão ou chave genérica
  const chave = id.replace(/^(anot_|exib_|geren_|saida_|colab_|param_|folha_)/, '');
  if (ICONES_SVG[id]) return ICONES_SVG[id];
  if (ICONES_SVG[chave]) return ICONES_SVG[chave];
  // Se não encontrar svg específico, devolve texto limpo sem emoji
  return simboloBackup;
}
