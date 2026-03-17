import { computed, type ComputedRef } from 'vue'
import type { Candle } from '@/services/api.service'

export function useChartStats(bougies: ComputedRef<Candle[]>) {
  const dernierPrix = computed(() => {
    const b = bougies.value
    return b.length > 0 ? b[b.length - 1].close : null
  })

  const variation = computed(() => {
    const b = bougies.value
    if (b.length < 2) return 0
    const avant = b[b.length - 2].close
    const apres = b[b.length - 1].close
    return ((apres - avant) / avant) * 100
  })

  const stats = computed(() => {
    const b = bougies.value
    if (b.length === 0) return null
    const high = Math.max(...b.map((c) => c.high))
    const low = Math.min(...b.map((c) => c.low))
    const volumeMoy = b.reduce((s, c) => s + c.volume, 0) / b.length
    const dernier = b[b.length - 1]
    const range = high - low
    const positionRange = range > 0 ? ((dernier.close - low) / range) * 100 : 50
    const volRelatif = volumeMoy > 0 ? dernier.volume / volumeMoy : 1
    const vwapDen = b.reduce((s, c) => s + c.volume, 0)
    const vwap = vwapDen > 0
      ? b.reduce((s, c) => s + ((c.high + c.low + c.close) / 3) * c.volume, 0) / vwapDen
      : dernier.close
    return { count: b.length, high, low, volumeMoy, range, positionRange, volRelatif, vwap }
  })

  return { dernierPrix, variation, stats }
}
