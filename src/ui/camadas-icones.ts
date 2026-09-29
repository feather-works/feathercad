/** Os ícones do gerenciador de camadas: SVG simples, coloridos por variável de tema (nunca cor solta). */

/** As cores dos ícones (por variável de tema). */
export const ICONES_CSS = `
.ic { display: inline-flex; vertical-align: middle; }
.ic-ativo { color: var(--icone-ativo); }
.ic-frio { color: var(--icone-frio); }
.ic-bloqueio { color: var(--icone-bloqueio); }
.ic-ok { color: var(--icone-ok); }
.ic-inativo { color: var(--texto-suave); opacity: 0.6; }
.ic-neutro { color: var(--texto); }
`;

function svg(corpo: string, classe: string, dica?: string): HTMLElement {
  const el = document.createElement('span');
  el.className = `ic ${classe}`;
  el.innerHTML = `<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">${corpo}</svg>`;
  if (dica) el.title = dica;
  return el;
}

/** A lâmpada: acesa (ativada) ou apagada. */
export function iconeLampada(ligada: boolean): HTMLElement {
  return svg('<path d="M5.5 10.5c-1.3-.9-2-2.2-2-3.6a4.5 4.5 0 0 1 9 0c0 1.4-.7 2.7-2 3.6V12h-5z" fill="currentColor" fill-opacity="0.35"/><path d="M6 14h4M6.5 12v2M9.5 12v2"/>', ligada ? 'ic-ativo' : 'ic-inativo');
}

/** O sol (descongelada) ou o floco de neve (congelada). */
export function iconeSolOuNeve(congelada: boolean): HTMLElement {
  return congelada
    ? svg('<path d="M8 1.5v13M2.4 4.75l11.2 6.5M2.4 11.25l11.2-6.5M6.5 3L8 4.5 9.5 3M6.5 13L8 11.5 9.5 13"/>', 'ic-frio')
    : svg('<circle cx="8" cy="8" r="2.8" fill="currentColor" fill-opacity="0.4"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4"/>', 'ic-ativo');
}

/** O cadeado: aberto ou fechado. */
export function iconeCadeado(bloqueada: boolean): HTMLElement {
  return bloqueada
    ? svg('<rect x="3.5" y="7" width="9" height="6.5" rx="1" fill="currentColor" fill-opacity="0.35"/><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2"/>', 'ic-bloqueio')
    : svg('<rect x="3.5" y="7" width="9" height="6.5" rx="1"/><path d="M5.5 7V5a2.5 2.5 0 0 1 4.8-1"/>', 'ic-inativo');
}

/** A impressora: com ou sem plotagem. */
export function iconeImpressora(plotar: boolean): HTMLElement {
  return svg(
    `<rect x="4" y="2" width="8" height="4"/><rect x="2" y="6" width="12" height="5" rx="1" ${plotar ? 'fill="currentColor" fill-opacity="0.3"' : ''}/><rect x="4.5" y="9.5" width="7" height="4.5" fill="none"/>${plotar ? '' : '<path d="M2 14L14 2"/>'}`,
    plotar ? 'ic-neutro' : 'ic-inativo',
  );
}

/** O estado da camada: a atual (✓), com objetos ou vazia. */
export function iconeStatus(atual: boolean, usada: boolean, dica: string): HTMLElement {
  if (atual) return svg('<path d="M3 8.5l3.2 3.2L13 4.8" stroke-width="2"/>', 'ic-ok', dica);
  return svg(usada ? '<path d="M3 4l5-2 5 2-5 2zM3 8l5 2 5-2M3 11.5l5 2 5-2"/>' : '<path d="M3 4l5-2 5 2-5 2z"/><path d="M3 8l5 2 5-2M3 11.5l5 2 5-2" stroke-opacity="0.35"/>', usada ? 'ic-neutro' : 'ic-inativo', dica);
}
