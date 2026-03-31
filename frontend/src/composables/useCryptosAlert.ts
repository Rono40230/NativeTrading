import { computed, ref, watch } from 'vue'
import { usePrixStore } from '@/stores/prix.store'

export type BadgeNiveau = 'explosion' | 'élan' | 'chaud' | 'haussier'

export interface CryptoAlert {
  symbol:         string
  ticker:         string
  prix:           number
  change24h:      number
  change1h:       number | null
  volume24h:      number
  volumeRatio:    number
  nbTrades:       number
  score:          number
  badge:          BadgeNiveau
  ralentissement: boolean
}

const STABLECOINS = new Set([
  'BUSD', 'USDC', 'TUSD', 'DAI', 'USDP', 'FDUSD', 'USDS',
  'EUR', 'GBP', 'BVND', 'PAX', 'SUSD',
])

function calculerBadge(change: number): BadgeNiveau {
  if (change >= 20) return 'explosion'
  if (change >= 10) return 'élan'
  if (change >= 5)  return 'chaud'
  return 'haussier'
}

/** Variation 1h normalisée : ≤0% → 0 pts, +5%+ → 100 pts */
function scoreChange1h(c1h: number | null): number {
  if (c1h === null || !isFinite(c1h) || c1h <= 0) return 0
  return Math.min(c1h / 5, 1) * 100
}

// ── Helpers UI exportés (utilisés par CryptosAlert.vue) ────────────────────

export const TF_CONFIGS = [
  { label: '1H', interval: '1m',  limit: 60 },
  { label: '4H', interval: '5m',  limit: 48 },
  { label: 'D1', interval: '1h',  limit: 24 },
  { label: 'W1', interval: '4h',  limit: 42 },
]

export function icone(badge: BadgeNiveau): string {
  if (badge === 'explosion') return '🚀'
  if (badge === 'élan') return '⚡'
  if (badge === 'chaud') return '🔥'
  return '📈'
}

export function classeCard(badge: BadgeNiveau): string {
  if (badge === 'explosion') return 'border-red-500/50 bg-red-500/10'
  if (badge === 'élan') return 'border-orange-500/40 bg-orange-500/10'
  if (badge === 'chaud') return 'border-yellow-500/30 bg-yellow-500/[0.08]'
  return 'border-emerald-500/20 bg-emerald-500/[0.05]'
}

export function formatVolume(v: number): string {
  if (v >= 1_000_000_000) return `${(v / 1_000_000_000).toFixed(1)}B$`
  if (v >= 1_000_000)     return `${(v / 1_000_000).toFixed(1)}M$`
  if (v >= 1_000)         return `${(v / 1_000).toFixed(0)}K$`
  return `${v.toFixed(0)}$`
}

export function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

export function classScore(s: number): string {
  if (s >= 70) return 'text-red-400'
  if (s >= 50) return 'text-orange-400'
  return 'text-emerald-400'
}

export function labelBadge(badge: BadgeNiveau): string {
  if (badge === 'explosion') return '🚀 Explosion'
  if (badge === 'élan') return '⚡ Élan'
  if (badge === 'chaud') return '🔥 Chaud'
  return '📈 Haussier'
}

export function sparklinePath(closes: number[]): string {
  const W = 240, H = 48
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

export function useCryptosAlert() {
  const prixStore = usePrixStore()
  const change1hMap = ref<Record<string, number>>({})

  async function enrichir1h(tickers: string[]) {
    if (tickers.length === 0) return
    const symbols = JSON.stringify(tickers.map(t => `${t}USDT`))
    try {
      const res = await fetch(
        `https://api.binance.com/api/v3/ticker?symbols=${encodeURIComponent(symbols)}&windowSize=1h`
      )
      if (!res.ok) return
      const data = await res.json() as Array<{ symbol: string; priceChangePercent: string }>
      const next: Record<string, number> = { ...change1hMap.value }
      for (const t of data) {
        next[t.symbol.slice(0, -4)] = parseFloat(t.priceChangePercent)
      }
      change1hMap.value = next
    } catch { /* silencieux */ }
  }

  // Déclenche l'enrichissement 1h à chaque refresh du store (toutes les 10s)
  watch(() => prixStore.tickers, (tickers) => {
    const top30 = Object.entries(tickers)
      .filter(([ticker, d]) => {
        if (STABLECOINS.has(ticker)) return false
        if (ticker.endsWith('UP') || ticker.endsWith('DOWN')) return false
        if (ticker.endsWith('BULL') || ticker.endsWith('BEAR')) return false
        return d.change24h >= 5 && d.volume24h >= 50_000
      })
      .sort(([, a], [, b]) => b.change24h - a.change24h)
      .slice(0, 30)
      .map(([t]) => t)
    enrichir1h(top30)
  }, { immediate: true })

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

    // Volume spike : ratio vs médiane de tous les tickers USDT filtrés
    const allVols = filtrees.map(([, d]) => d.volume24h).sort((a, b) => a - b)
    const medianVol = allVols[Math.floor(allVols.length / 2)] || 1
    const VOL_RATIO_MAX = 20 // cap pour éviter la dominance d'un outlier

    const maxChange = Math.max(...filtrees.map(([, d]) => d.change24h))
    const maxCount  = Math.max(...filtrees.map(([, d]) => d.nbTrades))

    const scorees: CryptoAlert[] = filtrees.map(([ticker, d]) => {
      const volRatio = Math.min(d.volume24h / medianVol, VOL_RATIO_MAX)
      const c1h      = change1hMap.value[ticker] ?? null
      const score =
        0.35 * (maxChange > 0 ? (d.change24h / maxChange) * 100 : 0) +
        0.25 * scoreChange1h(c1h) +
        0.30 * (volRatio / VOL_RATIO_MAX) * 100 +
        0.10 * (maxCount  > 0 ? (d.nbTrades  / maxCount) * 100 : 0)
      return {
        symbol: `${ticker}USDT`,
        ticker,
        prix: d.prix,
        change24h: d.change24h,
        change1h: c1h,
        volume24h: d.volume24h,
        volumeRatio: volRatio,
        nbTrades: d.nbTrades,
        score,
        badge: calculerBadge(d.change24h),
        ralentissement: c1h !== null && isFinite(c1h) && c1h < -1,
      }
    })

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
