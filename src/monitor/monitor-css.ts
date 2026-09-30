/** O estilo da página Monitor (`src/monitor/pagina.ts`), sobre as variáveis de cor do app. */
export const MONITOR_CSS = String.raw`
body { margin: 0; background: var(--fundo-area); color: var(--texto); font-family: var(--fonte); font-size: 13px; }
.mon { display: flex; flex-direction: column; height: 100vh; }
.mon-topo { display: flex; align-items: center; gap: 16px; padding: 10px 16px 0; border-bottom: 1px solid var(--borda); }
.mon-topo h1 { font-size: 15px; margin: 0 12px 10px 0; }
.mon-abas { display: flex; gap: 2px; }
.mon-abas button { border: 0; border-bottom: 2px solid transparent; background: none; color: var(--texto-suave); padding: 8px 14px; font: inherit; cursor: pointer; }
.mon-abas button[aria-selected="true"] { color: var(--destaque); border-bottom-color: var(--destaque); font-weight: 600; }
.mon-contador { margin-left: 6px; padding: 0 6px; border-radius: 8px; background: var(--danger, #d93025); color: #fff; font-size: 11px; }
.mon-corpo { flex: 1; min-height: 0; }
.mon-aba { display: flex; flex-direction: column; gap: 12px; height: 100%; box-sizing: border-box; padding: 12px 16px; overflow: auto; }
.mon-aba[hidden] { display: none; }
.mon-cartoes { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.mon-cartao { display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; border: 1px solid var(--borda); border-radius: 8px; }
.mon-cartao strong { font-size: 20px; }
.mon-cartao span { color: var(--texto-suave); font-size: 12px; }
.mon-grafico { width: 100%; height: 120px; border: 1px solid var(--borda); border-radius: 8px; }
.mon-aba h3 { margin: 4px 0 0; font-size: 13px; }
.mon-tabela { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
.mon-tabela th, .mon-tabela td { text-align: left; padding: 4px 8px; border-bottom: 1px solid var(--borda); vertical-align: top; }
.mon-tabela th { position: sticky; top: 0; background: var(--fundo-area); color: var(--texto-suave); font-weight: 500; }
.mon-tabela td:nth-child(n+3) { text-align: right; white-space: nowrap; }
.mon-tabela th:nth-child(n+3) { text-align: right; }
.mon-lenta td { color: var(--danger, #d93025); }
.mon-barra { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.mon-barra button, .mon-barra select, .mon-barra input[type="search"] { padding: 4px 10px; border: 1px solid var(--borda); border-radius: 6px; background: var(--fundo-area); color: var(--texto); font: inherit; }
.mon-barra button[aria-pressed="true"] { border-color: var(--destaque); color: var(--destaque); }
.mon-barra input[type="search"] { flex: 1; min-width: 160px; }
.mon-lista { flex: 1; min-height: 0; overflow: auto; font-family: ui-monospace, Consolas, monospace; font-size: 12px; }
.mon-log { padding: 3px 4px; border-bottom: 1px solid var(--borda); white-space: pre-wrap; word-break: break-word; }
.mon-log time { color: var(--texto-suave); margin-right: 8px; }
.mon-error { color: var(--danger, #d93025); }
.mon-warn { color: #b06000; }
.mon-log pre { margin: 4px 0 4px 20px; color: var(--texto-suave); }
`;
