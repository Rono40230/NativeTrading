/**
 * Calculs probabilistes génériques pour les séries de SL consécutifs.
 * Partagé entre Rockets, SMC et Straddle.
 */
import { computed } from 'vue'
import type { ComputedRef, Ref } from 'vue'

export const K_VALUES   = [2, 3, 4, 5, 6, 7, 8, 9, 10]
export const LOSS_RATES = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95]

export function probAtLeastKCons(n: number, p: number, k: number): number {
  if (n < k || p <= 0) return 0
  if (p >= 1) return 100
  const dp = new Array(k).fill(0)
  dp[0] = 1.0
  for (let i = 0; i < n; i++) {
    const next = new Array(k).fill(0)
    for (let j = 0; j < k; j++) {
      if (dp[j] === 0) continue
      next[0] += dp[j] * (1 - p)
      if (j + 1 < k) next[j + 1] += dp[j] * p
    }
    dp.splice(0, dp.length, ...next)
  }
  const pNever = dp.reduce((a, b) => a + b, 0)
  return Math.round(Math.max(0, Math.min(100, (1 - pNever) * 100)) * 10) / 10
}

export function couleurProba(pct: number): string {
  const hue       = Math.round(pct * 1.2)
  const lightness = pct > 5 ? 28 : 12
  return `hsl(${hue}, 70%, ${lightness}%)`
}

export interface ProbaHeatmapResult {
  tableauPertes: ComputedRef<{ lossRate: number; isActual: boolean; probs: number[] }[]>
  analyseProba: ComputedRef<{
    kCritique50: number
    kDanger: number
    probAuKDanger: number
    kSurete: number
    esperance: number
  }>
}

/**
 * @param lossRateRef  - taux de perte en % (0–100), ex: 40 pour 40%
 * @param sampleSizeRef - nombre de trades utilisés pour le calcul
 * @param rMoyenRef    - R moyen des trades gagnants (pour espérance)
 * @param winRateRef   - taux de victoire en % (0–100)
 */
export function useProbaHeatmap(
  lossRateRef: Ref<number> | ComputedRef<number>,
  sampleSizeRef: Ref<number> | ComputedRef<number>,
  rMoyenRef: Ref<number> | ComputedRef<number>,
  winRateRef: Ref<number> | ComputedRef<number>,
): ProbaHeatmapResult {
  const tableauPertes = computed(() => {
    const n      = sampleSizeRef.value
    const actual = lossRateRef.value
    const nearest = LOSS_RATES.reduce((prev, cur) =>
      Math.abs(cur - actual) < Math.abs(prev - actual) ? cur : prev, LOSS_RATES[0])
    return LOSS_RATES.map(lr => ({
      lossRate: lr,
      isActual: lr === nearest && actual > 0,
      probs: K_VALUES.map(k => probAtLeastKCons(n, lr / 100, k)),
    }))
  })

  const analyseProba = computed(() => {
    const n  = sampleSizeRef.value
    const lr = lossRateRef.value
    if (lr === 0) return { kCritique50: 0, kDanger: 0, probAuKDanger: 0, kSurete: 0, esperance: 0 }
    const p             = lr / 100
    const kCritique50   = K_VALUES.find(k => probAtLeastKCons(n, p, k) >= 50) ?? K_VALUES[K_VALUES.length - 1]
    const kDanger       = K_VALUES.find(k => probAtLeastKCons(n, p, k) < 30) ?? K_VALUES[K_VALUES.length - 1]
    const probAuKDanger = probAtLeastKCons(n, p, kDanger)
    const kSurete       = K_VALUES.find(k => probAtLeastKCons(n, p, k) < 5)  ?? K_VALUES[K_VALUES.length - 1]
    const wr            = winRateRef.value / 100
    const esperance     = parseFloat((wr * rMoyenRef.value + (1 - wr) * (-1)).toFixed(2))
    return { kCritique50, kDanger, probAuKDanger, kSurete, esperance }
  })

  return { tableauPertes, analyseProba }
}
