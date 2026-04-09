import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'

const TICKERS_URL = '/api/marche/tickers'   // proxy backend → Binance (évite CORS/451)
const NON_CRYPTO_POLL_MS = 15_000
const CRYPTO_POLL_MS = 10_000              // polling backend au lieu du WS Binance direct

// Assets non-crypto alimentés par IG Markets via le backend (métaux, forex, indices)
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

  let actif = false
  let intervalCrypto: ReturnType<typeof setInterval> | null = null
  let intervalNonCrypto: ReturnType<typeof setInterval> | null = null

  // Appel backend proxy — pas d'appel direct vers api.binance.com
  async function chargerCrypto() {
    try {
      const res = await fetch(TICKERS_URL)
      if (!res.ok) { erreur.value = true; return }
      const data = await res.json() as Record<string, { prix: number; change24h: number; volume24h: number; nb_trades: number }>
      let count = 0
      for (const [ticker, t] of Object.entries(data)) {
        tickers.value[ticker] = {
          prix: t.prix,
          change24h: t.change24h,
          volume24h: t.volume24h,
          nbTrades: t.nb_trades,
        }
        count++
      }
      totalPaires.value = count
      erreur.value = false
    } catch { erreur.value = true }
  }

  async function chargerNonCrypto() {
    try {
      const prixIg = await apiService.getPrixAssets(NON_CRYPTO_ASSETS)
      for (const [ticker, prix] of Object.entries(prixIg)) {
        tickers.value[ticker] = { prix, change24h: 0, volume24h: 0, nbTrades: 0 }
      }
    } catch { /* données précédentes conservées */ }
  }

  function getPrix(ticker: string): number | null {
    return tickers.value[ticker]?.prix ?? null
  }

  function demarrer() {
    if (actif) return
    actif = true
    chargement.value = true
    chargerCrypto().then(() => { chargement.value = false })
    chargerNonCrypto()
    intervalCrypto = setInterval(chargerCrypto, CRYPTO_POLL_MS)
    intervalNonCrypto = setInterval(chargerNonCrypto, NON_CRYPTO_POLL_MS)
  }

  function arreter() {
    actif = false
    if (intervalCrypto) { clearInterval(intervalCrypto); intervalCrypto = null }
    if (intervalNonCrypto) { clearInterval(intervalNonCrypto); intervalNonCrypto = null }
  }

  return { tickers, chargement, erreur, totalPaires, getPrix, demarrer, arreter }
})

