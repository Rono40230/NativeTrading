import { computed } from 'vue'
import { usePrixStore } from '@/stores/prix.store'

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
  const prixStore = usePrixStore()

  const top20 = computed((): CryptoAlert[] => {
    const filtrees = Object.entries(prixStore.tickers).filter(([ticker, d]) => {
      if (STABLECOINS.has(ticker)) return false
      if (ticker.endsWith('UP') || ticker.endsWith('DOWN')) return false
      if (ticker.endsWith('BULL') || ticker.endsWith('BEAR')) return false
      if (d.change24h < 10) return false
      if (d.volume24h < 50_000) return false
      return true
    })

    if (filtrees.length === 0) return []

    const maxChange = Math.max(...filtrees.map(([, d]) => d.change24h))
    const maxVolume = Math.max(...filtrees.map(([, d]) => d.volume24h))
    const maxCount  = Math.max(...filtrees.map(([, d]) => d.nbTrades))

    const scorees: CryptoAlert[] = filtrees.map(([ticker, d]) => ({
      symbol: `${ticker}USDT`,
      ticker,
      prix: d.prix,
      change24h: d.change24h,
      volume24h: d.volume24h,
      nbTrades: d.nbTrades,
      score:
        0.5 * (maxChange > 0 ? (d.change24h / maxChange) * 100 : 0) +
        0.3 * (maxVolume > 0 ? (d.volume24h / maxVolume) * 100 : 0) +
        0.2 * (maxCount  > 0 ? (d.nbTrades  / maxCount)  * 100 : 0),
      badge: calculerBadge(d.change24h),
    }))

    return scorees.sort((a, b) => b.score - a.score).slice(0, 20)
  })

  return {
    top20,
    chargement: computed(() => prixStore.chargement),
    erreur: computed(() => prixStore.erreur),
    totalPaires: computed(() => prixStore.totalPaires),
    demarrer: () => prixStore.demarrer(),
    arreter: () => {},
  }
}
