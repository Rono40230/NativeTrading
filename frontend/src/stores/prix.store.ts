import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'

const BINANCE_24H_URL = 'https://api.binance.com/api/v3/ticker/24hr'
const POLL_MS = 10_000

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
  let intervalle: ReturnType<typeof setInterval> | null = null

  async function charger() {
    if (chargement.value) return
    chargement.value = true
    erreur.value = false
    try {
      const res = await fetch(BINANCE_24H_URL)
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json() as Array<Record<string, string>>

      const map: Record<string, TickerData> = {}
      let count = 0
      for (const t of data) {
        if (!t.symbol?.endsWith('USDT')) continue
        if (t.symbol.endsWith('UPUSDT') || t.symbol.endsWith('DOWNUSDT')) continue
        if (t.symbol.endsWith('BULLUSDT') || t.symbol.endsWith('BEARUSDT')) continue
        const ticker = t.symbol.slice(0, -4)
        map[ticker] = {
          prix: parseFloat(t.lastPrice),
          change24h: parseFloat(t.priceChangePercent),
          volume24h: parseFloat(t.quoteVolume),
          nbTrades: parseInt(t.count, 10),
        }
        count++
      }
      // Fusion en place : ne jamais vider le store - mettre à jour seulement les entrées reçues
      for (const [ticker, data] of Object.entries(map)) {
        tickers.value[ticker] = data
      }
      totalPaires.value = count

      // Enrichissement non-crypto (métaux, forex, indices) via backend → Yahoo Finance
      const prixIb = await apiService.getPrixAssets(NON_CRYPTO_ASSETS)
      for (const [ticker, prix] of Object.entries(prixIb)) {
        tickers.value[ticker] = { prix, change24h: 0, volume24h: 0, nbTrades: 0 }
      }
    } catch {
      erreur.value = true
    } finally {
      chargement.value = false
    }
  }

  function getPrix(ticker: string): number | null {
    return tickers.value[ticker]?.prix ?? null
  }

  // Idempotent : un seul fetch tourne quel que soit le nombre d'appelants
  function demarrer() {
    if (intervalle) return
    charger()
    intervalle = setInterval(charger, POLL_MS)
  }

  function arreter() {
    if (intervalle) { clearInterval(intervalle); intervalle = null }
  }

  return { tickers, chargement, erreur, totalPaires, getPrix, demarrer, arreter }
})
