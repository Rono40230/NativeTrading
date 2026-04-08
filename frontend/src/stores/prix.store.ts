import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'

const BINANCE_24H_URL = 'https://api.binance.com/api/v3/ticker/24hr'
const BINANCE_WS_URL = 'wss://stream.binance.com:9443/ws/!ticker@arr'
const NON_CRYPTO_POLL_MS = 15_000
const FALLBACK_POLL_MS = 30_000
const WS_RECONNECT_MS = 5_000

// Assets non-crypto alimentés par Yahoo Finance via le backend (métaux, forex, indices)
const NON_CRYPTO_ASSETS = [
  'XAUUSD', 'XAGUSD', 'XPTUSD', 'XPDUSD',
  'EURUSD', 'GBPUSD', 'USDJPY', 'USDCHF', 'AUDUSD', 'USDCAD', 'NZDUSD',
  'GBPJPY', 'CADJPY', 'NZDJPY', 'EURJPY', 'EURGBP',
  'DAX', 'NAS100', 'SP500', 'US30', 'FTSE100', 'CAC40', 'JP225',
]

export interface TickerData {
  prix: number
  change24h: number
  volume24h: number
  nbTrades: number
}

export const usePrixStore = defineStore('prix', () => {
  const tickers = ref<Record<string, TickerData>>({})
  const chargement = ref(false)
  const erreur = ref(false)
  const totalPaires = ref(0)

  let ws: WebSocket | null = null
  let wsActif = false
  let wsConnecte = false
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let intervalNonCrypto: ReturnType<typeof setInterval> | null = null
  let intervalFallback: ReturnType<typeof setInterval> | null = null

  // Bootstrap HTTP + fallback polling (utilisé si WebSocket bloqué par CSP avant rebuild)
  async function chargerCryptoHttp() {
    try {
      const res = await fetch(BINANCE_24H_URL)
      if (!res.ok) return
      const data = await res.json() as Array<Record<string, string>>
      let count = 0
      for (const t of data) {
        if (!t.symbol?.endsWith('USDT')) continue
        if (t.symbol.endsWith('UPUSDT') || t.symbol.endsWith('DOWNUSDT')) continue
        if (t.symbol.endsWith('BULLUSDT') || t.symbol.endsWith('BEARUSDT')) continue
        const ticker = t.symbol.slice(0, -4)
        tickers.value[ticker] = {
          prix: parseFloat(t.lastPrice),
          change24h: parseFloat(t.priceChangePercent),
          volume24h: parseFloat(t.quoteVolume),
          nbTrades: parseInt(t.count, 10),
        }
        count++
      }
      totalPaires.value = count
      erreur.value = false
    } catch { erreur.value = true }
  }

  function traiterMessageWs(data: string) {
    try {
      const ticks = JSON.parse(data) as Array<Record<string, string>>
      if (!Array.isArray(ticks)) return
      let count = 0
      for (const t of ticks) {
        const s = t.s
        if (!s?.endsWith('USDT')) continue
        if (s.endsWith('UPUSDT') || s.endsWith('DOWNUSDT')) continue
        if (s.endsWith('BULLUSDT') || s.endsWith('BEARUSDT')) continue
        const ticker = s.slice(0, -4)
        tickers.value[ticker] = {
          prix: parseFloat(t.c),
          change24h: parseFloat(t.P),
          volume24h: parseFloat(t.q),
          nbTrades: parseInt(t.n, 10),
        }
        count++
      }
      totalPaires.value = count
      erreur.value = false
      wsConnecte = true
      // WebSocket actif : désactiver le fallback polling crypto
      if (intervalFallback) { clearInterval(intervalFallback); intervalFallback = null }
    } catch { /* message malformé ignoré */ }
  }

  function connecterWs() {
    if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) return
    ws = new WebSocket(BINANCE_WS_URL)
    ws.onmessage = (ev) => traiterMessageWs(ev.data as string)
    ws.onerror = () => { ws?.close() }
    ws.onclose = () => {
      ws = null
      wsConnecte = false
      if (!wsActif) return
      // Réactiver fallback polling si WS coupé
      if (!intervalFallback) intervalFallback = setInterval(chargerCryptoHttp, FALLBACK_POLL_MS)
      reconnectTimer = setTimeout(connecterWs, WS_RECONNECT_MS)
    }
  }

  async function chargerNonCrypto() {
    try {
      const prixIb = await apiService.getPrixAssets(NON_CRYPTO_ASSETS)
      for (const [ticker, prix] of Object.entries(prixIb)) {
        tickers.value[ticker] = { prix, change24h: 0, volume24h: 0, nbTrades: 0 }
      }
    } catch { /* données précédentes conservées */ }
  }

  function getPrix(ticker: string): number | null {
    return tickers.value[ticker]?.prix ?? null
  }

  function demarrer() {
    if (wsActif) return
    wsActif = true
    chargement.value = true
    // Bootstrap immédiat via HTTP pour peupler le store sans attendre le WS
    chargerCryptoHttp().then(() => { chargement.value = false })
    chargerNonCrypto()
    // WebSocket pour mises à jour temps réel
    connecterWs()
    // Fallback polling au cas où le WS est bloqué (ex: CSP avant rebuild Tauri)
    intervalFallback = setInterval(chargerCryptoHttp, FALLBACK_POLL_MS)
    intervalNonCrypto = setInterval(chargerNonCrypto, NON_CRYPTO_POLL_MS)
  }

  function arreter() {
    wsActif = false
    wsConnecte = false
    if (reconnectTimer) { clearTimeout(reconnectTimer); reconnectTimer = null }
    if (ws) { ws.close(); ws = null }
    if (intervalFallback) { clearInterval(intervalFallback); intervalFallback = null }
    if (intervalNonCrypto) { clearInterval(intervalNonCrypto); intervalNonCrypto = null }
  }

  return { tickers, chargement, erreur, totalPaires, getPrix, demarrer, arreter }
})

