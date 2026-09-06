import { computed } from 'vue'
import type { ComputedRef } from 'vue'
import type { Signal } from '@/services/api.service'
import {
  useProbaHeatmap,
  couleurProba,
  K_VALUES as kValues,
  LOSS_RATES as lossRates,
} from '@/composables/useProbaHeatmap'

// ── Helpers privés ─────────────────────────────────────────────────────────

/// R réalisé d'un trade : la vérité du moteur (r_realise), repli calculé
/// sur le prix de sortie.
function rDuSignal(s: Signal): number | null {
  if (s.r_realise !== null && s.r_realise !== undefined) return s.r_realise
  if (!s.prix_verdict) return null
  const risk = s.prix_entree - s.stop_loss
  if (Math.abs(risk) < 1e-9) return null
  const pnl = s.direction?.toUpperCase() === 'LONG'
    ? s.prix_verdict - s.prix_entree
    : s.prix_entree - s.prix_verdict
  return pnl / Math.abs(risk)
}

/// Univers d'un symbole : les cryptos du scanner Binance finissent en USDT.
function universDuSymbole(a: string): 'crypto' | 'action' {
  return a.endsWith('USDT') ? 'crypto' : 'action'
}

function calcStats(liste: Signal[]) {
  const total = liste.length
  const rs = liste.map(rDuSignal).filter((v): v is number => v !== null)
  const gagnants = rs.filter(r => r > 0).length
  const perdants = rs.filter(r => r <= 0).length
  const rSomme = parseFloat(rs.reduce((a, b) => a + b, 0).toFixed(2))
  const rMoyen = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  const verdicts = liste.reduce<Record<string, number>>((acc, s) => {
    const v = (s.verdict ?? '—').toUpperCase()
    acc[v] = (acc[v] ?? 0) + 1
    return acc
  }, {})
  const winPct = total > 0 ? Math.round(gagnants / total * 100) : 0
  return { total, gagnants, perdants, gain: gagnants, sl: perdants, winPct, rSomme, rMoyen, verdicts }
}

// ── Composable exporté ─────────────────────────────────────────────────────
// V2 (05/09, §10) : stats sur les signaux officiels rockets (table partagée)
// — verdicts SL/TS et R réalisés. Les ex-tranches de score et phases v1
// (moteur ATR disparu) sont remplacées par la ventilation par univers.

export function useRocketsStats(signauxRef: ComputedRef<Signal[]>) {
  const stats = computed(() => {
    const s = calcStats(signauxRef.value)
    return { ...s, tauxGagnants: s.winPct, tauxSL: s.total > 0 ? Math.round(s.perdants / s.total * 100) : 0 }
  })

  const parUnivers = computed(() =>
    (['crypto', 'action'] as const).map(u => ({
      label: u,
      ...calcStats(signauxRef.value.filter(s => universDuSymbole(s.asset) === u)),
    })),
  )

  const sampleSize = computed(() => Math.max(signauxRef.value.length, 10))
  const lossRateReel = computed(() => stats.value.tauxSL)

  const { tableauPertes, analyseProba } = useProbaHeatmap(
    lossRateReel,
    sampleSize,
    computed(() => stats.value.rMoyen),
    computed(() => stats.value.tauxGagnants),
  )

  return {
    stats, parUnivers,
    kValues, lossRates, sampleSize, lossRateReel,
    tableauPertes, analyseProba, couleurProba,
  }
}
