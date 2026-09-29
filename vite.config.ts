import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const __dirname = dirname(fileURLToPath(import.meta.url));

// `vite dev` serve o index.html com HMR; `vite build` gera o `dist/` que o
// Tauri empacota. A porta é a do `devUrl` do `src-tauri/tauri.conf.json`
// (5174: o FeatherSheet usa 5173, e os dois abrem ao mesmo tempo).
export default defineConfig({
  base: './',
  // Duas páginas no mesmo build: o desenho e o Monitor (janela à parte, F12).
  build: {
    outDir: 'dist',
    target: 'es2022',
    rollupOptions: { input: { index: resolve(__dirname, 'index.html'), monitor: resolve(__dirname, 'monitor.html') } },
  },
  server: {
    port: 5174,
    strictPort: true,
    watch: { ignored: ['**/crates/**', '**/src-tauri/target/**'] },
  },
});
