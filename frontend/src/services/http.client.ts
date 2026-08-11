import axios from 'axios'

// TODO 1.7 CPG : dériver d'une variable d'env (VITE_API_URL) au lieu de hardcoder.
export const BASE_URL = 'http://localhost:8080'

// URL de base pour les WebSockets (déduite de BASE_URL : http(s) → ws(s)).
export const WS_BASE_URL = BASE_URL.replace(/^http/, 'ws')

// Instance axios partagée unique pour toute l'app (couche HTTP unifiée).
export const http = axios.create({ baseURL: BASE_URL, timeout: 15000 })
