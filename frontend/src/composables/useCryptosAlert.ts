import { ref } from 'vue'

export type BadgeNiveau = 'explosion' | 'breakout' | 'chaud' | 'haussier'

export interface CryptoAlert {
  symbol: string
  ticker: string
  prix: number
  change24h: number
  volume24h: number
  nbTrades: number
  score: number
  badge: BadgeNiveau
}

const BINANCE_24H_URL = 'https://api.binance.com/api/v3/ticker/24hr'

const STABLECOINS = new Set([
  'BUSD', 'USDC', 'TUSD', 'DAI', 'USDP', 'FDUSD', 'USDS',
  'EUR', 'GBP', 'BVND', 'PAX', 'SUSD',
])

function calculerBadge(change: number): BadgeNiveau {
  if (change >= 20) return 'explosion'
  if (change >= 10) return 'breakout'
  if (change >= 5) return 'chaud'
  return 'haussier'
}

export function useCryptosAlert() {
  const top20 = ref<CryptoAlert[]>([])
  const chargement = ref(false)
  const erreur = ref(false)
  let intervalle: ReturnType<typeof setInterval> | null = null

  async function charger() {
    if (chargement.value) return
    chargement.value = true
    erreur.value = false
    try {
      const res = await fetch(BINANCE_24H_URL)
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json() as Array<Record<string, string>>

      const filtrees = data.filter(t => {
        if (typeof t.symbol !== 'string') return false
        if (!t.symbol.endsWith('USDT')) return false
        // Exclure les tokens leveragés
        if (t.symbol.endsWith('UPUSDT') || t.symbol.endsWith('DOWNUSDT')) return false
        if (t.symbol.endsWith('BULLUSDT') || t.symbol.endsWith('BEARUSDT')) return false
        // Exclure les stablecoins
        const ticker = t.symbol.slice(0, -4)
        if (STABLECOINS.has(ticker)) return false
        // Uniquement les hausses à 2 chiffres (≥10%)
        if (parseFloat(t.priceChangePercent) < 10) return false
        // Volume minimum 50k USDT (inclure les micro-caps)
        if (parseFloat(t.quoteVolume) < 50_000) return false
        return true
      })

      if (filtrees.length === 0) {
        top20.value = []
        return
      }

      const maxChange = Math.max(...filtrees.map(t => parseFloat(t.priceChangePercent)))
      const maxVolume = Math.max(...filtrees.map(t => parseFloat(t.quoteVolume)))
      const maxCount  = Math.max(...filtrees.map(t => parseInt(t.count, 10)))

      const scorees: CryptoAlert[] = filtrees.map(t => {
        const change  = parseFloat(t.priceChangePercent)
        const volume  = parseFloat(t.quoteVolume)
        const nTrades = parseInt(t.count, 10)
        const score =
          0.5 * (maxChange > 0 ? (change  / maxChange) * 100 : 0) +
          0.3 * (maxVolume > 0 ? (volume  / maxVolume) * 100 : 0) +
          0.2 * (maxCount  > 0 ? (nTrades / maxCount)  * 100 : 0)
        return {
          symbol:   t.symbol,
          ticker:   t.symbol.slice(0, -4),
          prix:     parseFloat(t.lastPrice),
          change24h: change,
          volume24h: volume,
          nbTrades: nTrades,
          score,
          badge: calculerBadge(change),
        }
      })

      scorees.sort((a, b) => b.score - a.score)
      top20.value = scorees.slice(0, 20)
    } catch {
      erreur.value = true
    } finally {
      chargement.value = false
    }
  }

  function demarrer() {
    charger()
    intervalle = setInterval(charger, 60_000)
  }

  function arreter() {
    if (intervalle !== null) {
      clearInterval(intervalle)
      intervalle = null
    }
  }

  return { top20, chargement, erreur, demarrer, arreter }
}
