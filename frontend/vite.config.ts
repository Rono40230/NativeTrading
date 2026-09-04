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
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      }
    }
  },
  envPrefix: ['VITE_', 'TAURI_'],
})
