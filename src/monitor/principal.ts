/** A entrada do `monitor.html`: aplica o tema da suíte e monta a página Monitor na janela. */
import { iniciarTema } from '../ui/tema.ts';
import { montarMonitor } from './pagina.ts';

iniciarTema();
montarMonitor(document.getElementById('monitor-app')!);
