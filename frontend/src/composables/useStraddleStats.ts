/**
 * Statistiques pour la modale d'analyse Straddle.
 */
import { computed } from 'vue'
import type { ComputedRef, Ref } from 'vue'
import type { Signal } from '@/services/api.service'
import {
  useProbaHeatmap,
  couleurProba,
  K_VALUES as kValues,
  LOSS_RATES as lossRates,
} from '@/composables/useProbaHeatmap'

const TRANCHES_DEF = [
  { label: '40–59',  min: 40,  max: 59  },
  { label: '60–79',  min: 60,  max: 79  },
  { label: '80–100', min: 80,  max: 100 },
]

/**
 * R réel d'un signal Straddle (direction=Both).
 * On prend la meilleure jambe gagnante si TP, sinon -1 si SL des deux jambes.
 */
function straddleR(s: Signal): number | null {
  if (!s.prix_verdict || !s.verdict) return null
  const v = s.verdict.toLowerCase()
  if (v === 'expire') return null
  if (v === 'sl') return -1
  const risk = Math.abs(s.prix_entree - s.stop_loss)
  if (risk <= 0) return null
  // Pour un straddle, le prix_verdict correspond à la jambe qui a gagné
  const pnl = Math.abs(s.prix_verdict - s.prix_entree)
  return parseFloat((pnl / risk).toFixed(2))
}

function calcStatsStraddle(liste: Signal[]) {
  // Verdicts servis en minuscule par /api/signaux (tp1/tp2/tp3/sl/be/ts/expire).
  const maj = (v: string | null | undefined) => (v ?? '').toLowerCase()
  const clos   = liste.filter(s => s.verdict && maj(s.verdict) !== 'expire')
  const total  = clos.length
  const tp1    = clos.filter(s => maj(s.verdict) === 'tp1').length
  const tp2    = clos.filter(s => maj(s.verdict) === 'tp2').length
  const tp3    = clos.filter(s => maj(s.verdict) === 'tp3').length
  const sl     = clos.filter(s => maj(s.verdict) === 'sl').length
  const ts     = clos.filter(s => maj(s.verdict) === 'ts').length
  const expire = liste.filter(s => s.verdict && maj(s.verdict) === 'expire').length
  const gain   = tp1 + tp2 + tp3 + ts
  const winPct = total > 0 ? Math.round(gain / total * 100) : 0
  const tauxSL = total > 0 ? Math.round(sl / total * 100) : 0
  const rs     = clos.map(s => straddleR(s)).filter((v): v is number => v !== null)
  const rMoyen = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  return { total, tp1, tp2, tp3, sl, ts, expire, gain, winPct, tauxSL, rMoyen }
}

export function useStraddleStats(signauxRef: Ref<Signal[]> | ComputedRef<Signal[]>) {
  const signaux = computed(() =>
    signauxRef.value.filter(s => s.strategie?.toLowerCase() === 'straddle')
  )

  const stats = computed(() => calcStatsStraddle(signaux.value))

  const tranches = computed(() =>
    TRANCHES_DEF.map(t => ({
      label: t.label,
      ...calcStatsStraddle(signaux.value.filter(s => s.score >= t.min && s.score <= t.max)),
    }))
  )

  /** Par asset — le Straddle est très lié à l'asset (BTC vs XAUUSD) */
  const parAsset = computed(() => {
    const assets = [...new Set(signaux.value.map(s => s.asset))].sort()
    return assets.map(asset => ({ asset, ...calcStatsStraddle(signaux.value.filter(s => s.asset === asset)) }))
  })

  const sampleSize   = computed(() => Math.max(signaux.value.length, 10))
  const lossRateReel = computed(() => stats.value.tauxSL)

  const { tableauPertes, analyseProba } = useProbaHeatmap(
    lossRateReel,
    sampleSize,
    computed(() => stats.value.rMoyen),
    computed(() => stats.value.winPct),
  )

  return {
    stats, tranches, parAsset,
    kValues, lossRates, sampleSize, lossRateReel,
    tableauPertes, analyseProba, couleurProba,
  }
}
