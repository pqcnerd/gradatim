import { defineConfig } from 'vite';
import monacoEditorPlugin from 'vite-plugin-monaco-editor';

export default defineConfig({
  root: __dirname,
  base: './',
  plugins: [
    monacoEditorPlugin({
      languageWorkers: ['editorWorkerService', 'css', 'html', 'json', 'typescript'],
      langs: ['c', 'cpp', 'plaintext']
    })
  ],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true
  }
});

