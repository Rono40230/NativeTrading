import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
        extensions: ['.mjs', '.mts', '.ts', '.tsx', '.vue', '.js', '.jsx', '.json'],
  },
  // Port 1420 = port standard Tauri dev (pas de navigateur externe)
  server: {
    port: 1420,
    strictPort: true,
    // 28/09 : le webview WebKitGTK mettait les chunks en cache disque — au
    // premier chargement après un run.sh il rejouait l'ancienne app (bug
    // « première modale ancienne, deuxième nouvelle »). no-store : chaque
    // démarrage sert le dist fraîchement construit, zéro cache local.
    headers: { 'Cache-Control': 'no-store' },
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      }
    }
  },
  // Même politique pour `vite preview` (le mode de run.sh — c'est LUI qui
  // sert le dist en production papier).
  preview: {
    headers: { 'Cache-Control': 'no-store' },
  },
  envPrefix: ['VITE_', 'TAURI_'],
})
