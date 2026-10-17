import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Dev : http://localhost:5174 (ou http://<ip-lan>:5174 depuis le téléphone)
// — le proxy /api rejoint le backend de la maison. En APK, l'app parlera
// directement au serveur configuré dans Réglages (Tailscale).
export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5174,
    proxy: {
      '/api': { target: 'http://127.0.0.1:8080', changeOrigin: true },
    },
  },
})
