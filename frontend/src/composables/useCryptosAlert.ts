/**
 * Composable partagé pour le composant CryptosAlert.
 * Types, helpers de formatage et configuration des timeframes sparkline.
 */

export type BadgeNiveau = 'explosion' | 'elan' | 'chaud'

export interface CryptoAlert {
  /** Symbole complet (ex : BTCUSDT) — utilisé comme clé unique */
  symbol: string
  /** Ticker court (ex : BTC) — affiché dans la carte */
  ticker: string
  badge: BadgeNiveau
  change24h: number
  change1h: number | null
  volume24h: number
  /** Rapport volume actuel / volume moyen (spike) */
  volumeRatio: number
  prix: number
  score: number
  /** Momentum 1h en baisse vs 24h — indicateur de pullback potentiel */
  ralentissement?: boolean
}

export interface TFConfig {
  label: string
  interval: string
  limit: number
}

export const TF_CONFIGS: TFConfig[] = [
  { label: '1H', interval: '1h', limit: 48 },
  { label: '4H', interval: '4h', limit: 60 },
  { label: 'D1', interval: '1d', limit: 90 },
]

export function icone(badge: BadgeNiveau): string {
  if (badge === 'explosion') return '🚀'
  if (badge === 'elan') return '⚡'
  return '🔥'
}

export function labelBadge(badge: BadgeNiveau): string {
  if (badge === 'explosion') return 'Explosion +15%'
  if (badge === 'elan') return 'Élan +10%'
  return 'Chaud +5%'
}

export function classeCard(badge: BadgeNiveau): string {
  if (badge === 'explosion') return 'border-red-500/40 bg-red-500/10'
  if (badge === 'elan') return 'border-orange-500/40 bg-orange-500/10'
  return 'border-yellow-500/40 bg-yellow-500/10'
}

export function classScore(score: number): string {
  if (score >= 80) return 'text-emerald-400'
  if (score >= 60) return 'text-yellow-400'
  if (score >= 40) return 'text-orange-400'
  return 'text-red-400'
}

export function formatVolume(vol: number): string {
  if (vol >= 1_000_000_000) return `${(vol / 1_000_000_000).toFixed(1)}B`
  if (vol >= 1_000_000) return `${(vol / 1_000_000).toFixed(1)}M`
  if (vol >= 1_000) return `${(vol / 1_000).toFixed(1)}K`
  return vol.toFixed(0)
}

export function formatPrix(prix: number): string {
  if (prix >= 10_000) return prix.toFixed(0)
  if (prix >= 1) return prix.toFixed(2)
  return prix.toFixed(4)
}

/**
 * Convertit un tableau de closes en points SVG polyline (viewBox 0 0 240 50).
 */
export function sparklinePath(closes: number[]): string {
  if (closes.length < 2) return ''
  const min = Math.min(...closes)
  const max = Math.max(...closes)
  const range = max - min || 1
  return closes
    .map((v, i) => {
      const x = (i / (closes.length - 1)) * 240
      const y = 50 - ((v - min) / range) * 40 - 5
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
}
