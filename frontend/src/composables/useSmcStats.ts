/**
 * Statistiques pour la modale d'analyse SMC Directionnel.
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

function signalR(s: Signal): number | null {
  if (!s.prix_verdict) return null
  const long = s.direction === 'Long' || s.direction === 'LONG'
  const risk  = Math.abs(s.prix_entree - s.stop_loss)
  if (risk <= 0) return null
  const pnl = long
    ? s.prix_verdict - s.prix_entree
    : s.prix_entree - s.prix_verdict
  return parseFloat((pnl / risk).toFixed(2))
}

function calcStatsSmc(liste: Signal[]) {
  const clos   = liste.filter(s => s.verdict && s.verdict !== 'expire')
  const total  = clos.length
  const tp1    = clos.filter(s => s.verdict === 'TP1').length
  const tp2    = clos.filter(s => s.verdict === 'TP2').length
  const tp3    = clos.filter(s => s.verdict === 'TP3').length
  const sl     = clos.filter(s => s.verdict === 'SL').length
  const expire = liste.filter(s => s.verdict === 'expire').length
  const gain   = tp1 + tp2 + tp3
  const winPct = total > 0 ? Math.round(gain / total * 100) : 0
  const tauxSL = total > 0 ? Math.round(sl / total * 100) : 0
  const rs     = clos.map(s => signalR(s)).filter((v): v is number => v !== null)
  const rMoyen = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  return { total, tp1, tp2, tp3, sl, expire, gain, winPct, tauxSL, rMoyen }
}

export function useSmcStats(signauxRef: Ref<Signal[]> | ComputedRef<Signal[]>) {
  const stats = computed(() => {
    const smc = signauxRef.value.filter(
      s => ['SMC', 'SmcDirectional', 'SMC+IA'].includes(s.strategie)
    )
    const base = calcStatsSmc(smc)
    const avecConviction = smc.filter(s => s.llm_conviction != null)
    const convictionMoyenne = avecConviction.length > 0
      ? Math.round(avecConviction.reduce((acc, s) => acc + (s.llm_conviction ?? 0), 0) / avecConviction.length)
      : 0
    const avecLlm   = smc.filter(s => s.llm_valide != null).length
    const tauxFiltrage = smc.length > 0 ? Math.round(avecLlm / smc.length * 100) : 0
    const longs  = smc.filter(s => s.direction === 'Long' || s.direction === 'LONG').length
    const shorts = smc.filter(s => s.direction === 'Short' || s.direction === 'SHORT').length
    const derniersLlm = smc.filter(s => s.llm_valide != null).slice(0, 5)
    return { ...base, convictionMoyenne, tauxFiltrage, longs, shorts, derniersLlm }
  })

  const signaux = computed(() =>
    signauxRef.value.filter(s => ['SMC', 'SmcDirectional', 'SMC+IA'].includes(s.strategie))
  )

  const tranches = computed(() =>
    TRANCHES_DEF.map(t => ({
      label: t.label,
      ...calcStatsSmc(signaux.value.filter(s => s.score >= t.min && s.score <= t.max)),
    }))
  )

  const parTimeframe = computed(() => {
    const tfs = [...new Set(signaux.value.map(s => s.timeframe))].sort()
    return tfs.map(tf => ({ tf, ...calcStatsSmc(signaux.value.filter(s => s.timeframe === tf)) }))
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
    stats, tranches, parTimeframe,
    kValues, lossRates, sampleSize, lossRateReel,
    tableauPertes, analyseProba, couleurProba,
  }
}
