import { computed } from 'vue'
import type { ComputedRef } from 'vue'
import type { RocketSignalHistorique } from '@/services/api.types'
import {
  useProbaHeatmap,
  couleurProba,
  K_VALUES as kValues,
  LOSS_RATES as lossRates,
} from '@/composables/useProbaHeatmap'

// ── Helpers privés ─────────────────────────────────────────────────────────

function rocketR(r: RocketSignalHistorique): number | null {
  if (!r.verdict || !r.prix_verdict) return null
  const risk = r.prix_entree - r.stop_loss
  if (risk <= 0) return null
  return (r.prix_verdict - r.prix_entree) / risk
}

function calcStats(liste: RocketSignalHistorique[]) {
  const clos   = liste.filter(r => r.verdict && r.verdict !== 'expire')
  const total  = clos.length
  const tp1    = clos.filter(r => r.verdict === 'TP1' || r.verdict === 'confirme').length
  const tp2    = clos.filter(r => r.verdict === 'TP2').length
  const tp3    = clos.filter(r => r.verdict === 'TP3').length
  const sl     = clos.filter(r => r.verdict === 'invalide').length
  const expire = liste.filter(r => r.verdict === 'expire').length
  const gain   = tp1 + tp2 + tp3
  const winPct = total > 0 ? Math.round(gain / total * 100) : 0
  const rs     = clos.map(r => rocketR(r)).filter((v): v is number => v !== null)
  const rMoyen = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  return { total, tp1, tp2, tp3, sl, expire, gain, winPct, rMoyen }
}

const TRANCHES_DEF = [
  { label: '15–39',  min: 15,  max: 39  },
  { label: '40–59',  min: 40,  max: 59  },
  { label: '60–79',  min: 60,  max: 79  },
  { label: '80–100', min: 80,  max: 100 },
]

function probAtLeastKCons(n: number, p: number, k: number): number {
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

// ── Composable exporté ─────────────────────────────────────────────────────

export function useRocketsStats(rocketsRef: ComputedRef<RocketSignalHistorique[]>) {
  const stats = computed(() => {
    const s = calcStats(rocketsRef.value)
    return { ...s, tauxGagnants: s.winPct, tauxSL: s.total > 0 ? Math.round(s.sl / s.total * 100) : 0 }
  })

  const tranches = computed(() =>
    TRANCHES_DEF.map(t => ({
      label: t.label,
      ...calcStats(rocketsRef.value.filter(r => r.score >= t.min && r.score <= t.max)),
    }))
  )

  const phases = computed(() => {
    const ps = [...new Set(rocketsRef.value.map(r => r.phase))]
    return ps.map(phase => ({ phase, ...calcStats(rocketsRef.value.filter(r => r.phase === phase)) }))
  })

  function classePhase(phase: string): string {
    if (phase.toLowerCase().includes('break')) return 'bg-emerald-900/60 text-emerald-300'
    if (phase.toLowerCase().includes('bull'))  return 'bg-blue-900/60 text-blue-300'
    if (phase.toLowerCase().includes('bear'))  return 'bg-red-900/60 text-red-300'
    return 'bg-yellow-900/60 text-yellow-300'
  }

  const sampleSize   = computed(() => Math.max(rocketsRef.value.length, 10))
  const lossRateReel = computed(() => stats.value.tauxSL)

  const { tableauPertes, analyseProba } = useProbaHeatmap(
    lossRateReel,
    sampleSize,
    computed(() => stats.value.rMoyen),
    computed(() => stats.value.tauxGagnants),
  )

  return {
    stats, tranches, phases, classePhase,
    kValues, lossRates, sampleSize, lossRateReel,
    tableauPertes, analyseProba, couleurProba,
  }
}
