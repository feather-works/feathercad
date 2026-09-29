//! Painel de Console de Depuração (DevConsole) no próprio aplicativo.
//! Copiado do FeatherSheet (`ui/paineis/debug-console.ts`) em 29/09/2026 — pedido do
//! usuário: manter o F12. Textos vêm do arquivo de língua (`monitor.overlay`).
//! Captura console.error, console.warn, erros globais da janela e rejeições de Promise não tratadas.

import { textos } from '../idiomas/index.ts';
import { Z } from './z.ts';

export interface LogEntry {
  id: number;
  timestamp: string;
  level: 'error' | 'warn' | 'info';
  message: string;
  stack?: string;
}

export const DEBUG_CONSOLE_CSS = `
.dev-console-overlay {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 320px;
  background: #1e1e2e;
  color: #cdd6f4;
  font-family: ui-monospace, Consolas, monospace;
  font-size: 12px;
  z-index: ${Z.console};
  box-shadow: 0 -4px 20px rgba(0,0,0,0.4);
  display: flex;
  flex-direction: column;
  border-top: 2px solid #89b4fa;
}

.dev-console-header {
  height: 36px;
  background: #181825;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #313244;
  user-select: none;
}

.dev-console-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #89b4fa;
}

.dev-console-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dev-console-btn {
  background: #313244;
  border: 1px solid #45475a;
  color: #cdd6f4;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}

.dev-console-btn:hover {
  background: #45475a;
  color: #fff;
}

.dev-console-btn.active {
  background: #89b4fa;
  color: #11111b;
  border-color: #89b4fa;
}

.dev-console-btn.danger-btn {
  background: #f38ba8;
  color: #11111b;
  border-color: #f38ba8;
}

.dev-console-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.dev-console-row {
  display: flex;
  flex-direction: column;
  padding: 6px 8px;
  border-radius: 4px;
  background: #11111b;
  border-left: 3px solid #89b4fa;
  word-break: break-all;
  white-space: pre-wrap;
}

.dev-console-row.error {
  border-left-color: #f38ba8;
  background: rgba(243, 139, 168, 0.1);
  color: #f38ba8;
}

.dev-console-row.warn {
  border-left-color: #f9e2af;
  background: rgba(249, 226, 175, 0.1);
  color: #f9e2af;
}

.dev-console-meta {
  display: flex;
  gap: 10px;
  font-size: 10px;
  opacity: 0.7;
  margin-bottom: 2px;
}

.dev-console-stack {
  margin-top: 4px;
  padding-top: 4px;
  border-top: 1px dashed rgba(255,255,255,0.1);
  color: #a6adc8;
  font-size: 11px;
}

.dev-console-filter-input {
  background: #11111b;
  border: 1px solid #45475a;
  color: #cdd6f4;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 11px;
  outline: none;
  width: 140px;
}

.dev-console-filter-input:focus {
  border-color: #89b4fa;
}
`;

class DevConsoleManager {
  private logs: LogEntry[] = [];
  private logId = 0;
  private container: HTMLElement | null = null;
  private bodyEl: HTMLElement | null = null;
  private isVisible = false;
  private currentFilter: 'all' | 'error' | 'warn' | 'info' = 'all';
  private searchQuery = '';
  /** Quem quer saber de cada entrada nova (a página Monitor, `monitor-ponte.ts`). */
  private readonly ouvintes: ((e: LogEntry) => void)[] = [];

  /** Avisa `f` a cada entrada nova. */
  public ouvir(f: (e: LogEntry) => void): void {
    this.ouvintes.push(f);
  }

  /** As últimas entradas (até 500), da mais velha para a mais nova. */
  public historico(): LogEntry[] {
    return [...this.logs];
  }


  constructor() {
    this.initCapture();
  }

  private initCapture(): void {
    if (typeof window === 'undefined') return;

    // 1. Intercepta console.error, console.warn e console.log
    const origError = console.error;
    const origWarn = console.warn;
    const origLog = console.log;

    console.error = (...args: unknown[]) => {
      origError.apply(console, args);
      this.addLog('error', args);
    };

    console.warn = (...args: unknown[]) => {
      origWarn.apply(console, args);
      this.addLog('warn', args);
    };

    console.log = (...args: unknown[]) => {
      origLog.apply(console, args);
      this.addLog('info', args);
    };

    // 2. Erros globais da janela
    window.addEventListener('error', (event: ErrorEvent) => {
      const msg = event.message || 'Erro não identificado no navegador';
      const stack = event.error?.stack;
      this.addLog('error', [msg], stack);
    });

    // 3. Rejeições de Promises não tratadas
    window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
      const reason = event.reason;
      const msg = reason?.message || String(reason) || 'Rejeição de Promise não tratada';
      const stack = reason?.stack;
      this.addLog('error', [`[Unhandled Promise Rejection] ${msg}`], stack);
    });
  }

  public addLog(level: 'error' | 'warn' | 'info', args: unknown[], explicitStack?: string): void {
    const timestamp = new Date().toLocaleTimeString('pt-BR', { hour12: false }) + '.' + String(Date.now() % 1000).padStart(3, '0');

    let stack: string | undefined = explicitStack;
    const formattedArgs = args.map((arg) => {
      if (arg instanceof Error) {
        if (!stack) stack = arg.stack;
        return `${arg.name}: ${arg.message}`;
      }
      if (typeof arg === 'object' && arg !== null) {
        try {
          return JSON.stringify(arg, null, 2);
        } catch {
          return String(arg);
        }
      }
      return String(arg);
    }).join(' ');

    const entry: LogEntry = {
      id: ++this.logId,
      timestamp,
      level,
      message: formattedArgs,
      stack,
    };

    this.logs.push(entry);
    if (this.logs.length > 500) {
      this.logs.shift();
    }
    for (const f of this.ouvintes) {
      try {
        f(entry);
      } catch {
        // Quem ouve não derruba o console.
      }
    }

    if (this.isVisible) {
      this.renderLogs();
    }

    // Se estiver no Tauri (.deb / executável nativo), manda o log para o terminal
    const g = globalThis as unknown as {
      __TAURI_INTERNALS__?: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> };
      __TAURI__?: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> };
      __TAURI_INVOKE__?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
    const invokeFn = g.__TAURI_INTERNALS__?.invoke || g.__TAURI__?.invoke || g.__TAURI_INVOKE__;
    if (typeof invokeFn === 'function') {
      const payloadMsg = stack ? `${formattedArgs}\nStack: ${stack}` : formattedArgs;
      invokeFn('log_mensagem', { nivel: level, mensagem: payloadMsg }).catch(() => {});
    }

    // Se for erro crítico, pode auto-abrir o painel para ajudar o usuário a ver imediatamente

    if (level === 'error' && !this.isVisible) {
      this.show();
    }
  }

  public toggle(): void {
    if (this.isVisible) {
      this.hide();
    } else {
      this.show();
    }
  }

  public show(): void {
    this.isVisible = true;
    if (!this.container) {
      this.createUI();
    } else {
      this.container.hidden = false;
    }
    this.renderLogs();
  }

  public hide(): void {
    this.isVisible = false;
    if (this.container) {
      this.container.hidden = true;
    }
  }

  public clear(): void {
    this.logs = [];
    this.renderLogs();
  }

  private createUI(): void {
    const container = document.createElement('div');
    container.className = 'dev-console-overlay';

    // Cabeçalho
    const header = document.createElement('div');
    header.className = 'dev-console-header';

    const title = document.createElement('div');
    title.className = 'dev-console-title';
    title.textContent = textos().monitor.overlay.titulo;

    const actions = document.createElement('div');
    actions.className = 'dev-console-actions';

    // Input de Busca
    const filterInput = document.createElement('input');
    filterInput.className = 'dev-console-filter-input';
    filterInput.placeholder = textos().monitor.overlay.filtrar;
    filterInput.addEventListener('input', () => {
      this.searchQuery = filterInput.value.toLowerCase();
      this.renderLogs();
    });

    // Botões de filtro por nível
    const filterAll = document.createElement('button');
    filterAll.className = 'dev-console-btn active';
    filterAll.textContent = textos().monitor.overlay.todos;

    const filterError = document.createElement('button');
    filterError.className = 'dev-console-btn';
    filterError.textContent = textos().monitor.overlay.erros;

    const filterWarn = document.createElement('button');
    filterWarn.className = 'dev-console-btn';
    filterWarn.textContent = textos().monitor.overlay.avisos;

    const setFilter = (btn: HTMLElement, filter: 'all' | 'error' | 'warn' | 'info') => {
      [filterAll, filterError, filterWarn].forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      this.currentFilter = filter;
      this.renderLogs();
    };

    filterAll.addEventListener('click', () => setFilter(filterAll, 'all'));
    filterError.addEventListener('click', () => setFilter(filterError, 'error'));
    filterWarn.addEventListener('click', () => setFilter(filterWarn, 'warn'));

    // Botão Limpar
    const clearBtn = document.createElement('button');
    clearBtn.className = 'dev-console-btn';
    clearBtn.textContent = textos().monitor.overlay.limpar;
    clearBtn.addEventListener('click', () => this.clear());

    // Botão Copiar Logs
    const copyBtn = document.createElement('button');
    copyBtn.className = 'dev-console-btn';
    copyBtn.textContent = textos().monitor.overlay.copiar;
    copyBtn.addEventListener('click', () => {
      const text = this.logs.map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.message} ${l.stack ? '\n' + l.stack : ''}`).join('\n');
      navigator.clipboard.writeText(text).then(() => {
        copyBtn.textContent = textos().monitor.overlay.copiado;
        setTimeout(() => (copyBtn.textContent = textos().monitor.overlay.copiar), 1500);
      });
    });

    // Botão Fechar
    const closeBtn = document.createElement('button');
    closeBtn.className = 'dev-console-btn danger-btn';
    closeBtn.textContent = textos().monitor.overlay.fechar;
    closeBtn.addEventListener('click', () => this.hide());

    actions.append(filterInput, filterAll, filterError, filterWarn, clearBtn, copyBtn, closeBtn);
    header.append(title, actions);

    // Corpo dos Logs
    const body = document.createElement('div');
    body.className = 'dev-console-body';

    container.append(header, body);
    document.body.append(container);

    this.container = container;
    this.bodyEl = body;
  }

  private renderLogs(): void {
    if (!this.bodyEl) return;
    this.bodyEl.innerHTML = '';

    const filtered = this.logs.filter((log) => {
      if (this.currentFilter !== 'all' && log.level !== this.currentFilter) {
        return false;
      }
      if (this.searchQuery && !log.message.toLowerCase().includes(this.searchQuery)) {
        return false;
      }
      return true;
    });

    if (filtered.length === 0) {
      const empty = document.createElement('div');
      empty.style.color = '#7f849c';
      empty.style.padding = '12px';
      empty.style.fontStyle = 'italic';
      empty.textContent = textos().monitor.overlay.vazio;
      this.bodyEl.append(empty);
      return;
    }

    for (const log of filtered) {
      const row = document.createElement('div');
      row.className = `dev-console-row ${log.level}`;

      const meta = document.createElement('div');
      meta.className = 'dev-console-meta';
      meta.textContent = `[${log.timestamp}] [${log.level.toUpperCase()}]`;

      const msg = document.createElement('div');
      msg.textContent = log.message;

      row.append(meta, msg);

      if (log.stack) {
        const stackEl = document.createElement('div');
        stackEl.className = 'dev-console-stack';
        stackEl.textContent = log.stack;
        row.append(stackEl);
      }

      this.bodyEl.append(row);
    }

    // Auto-scroll para o final
    this.bodyEl.scrollTop = this.bodyEl.scrollHeight;
  }
}

export const devConsole = new DevConsoleManager();
