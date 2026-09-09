/**
 * Statistiques pour la page d'analyse SMC Directionnel.
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

// Score brut du moteur v12 (16 composantes) — les signaux réels vont de 6
// à 19, seuil trade BSZones = 7. Trois bandes métier : seuil / confirmé / fort.
const TRANCHES_DEF = [
  { label: '6–8',   min: 6,  max: 8  },
  { label: '9–11',  min: 9,  max: 11 },
  { label: '12–19', min: 12, max: 19 },
]

function maj(v: string | null | undefined): string {
  return (v ?? '').toLowerCase()
}

// Verdicts réels en base : 'SL' | 'TP1+BE' | 'TP2+BE' | 'TP3' | 'Expire'.
// R : la vérité moteur (r_realise) d'abord, recalcul au prix de verdict en repli.
function signalR(s: Signal): number | null {
  if (s.r_realise != null) return s.r_realise
  if (!s.prix_verdict) return null
  const long = maj(s.direction) === 'long'
  const risk  = Math.abs(s.prix_entree - s.stop_loss)
  if (risk <= 0) return null
  const pnl = long
    ? s.prix_verdict - s.prix_entree
    : s.prix_entree - s.prix_verdict
  return parseFloat((pnl / risk).toFixed(2))
}

function calcStatsSmc(liste: Signal[]) {
  const clos   = liste.filter(s => s.verdict && maj(s.verdict) !== 'expire')
  const total  = clos.length
  const tp1    = clos.filter(s => maj(s.verdict).startsWith('tp1')).length
  const tp2    = clos.filter(s => maj(s.verdict).startsWith('tp2')).length
  const tp3    = clos.filter(s => maj(s.verdict).startsWith('tp3')).length
  const sl     = clos.filter(s => maj(s.verdict) === 'sl').length
  const expire = liste.filter(s => maj(s.verdict) === 'expire').length
  const gain   = tp1 + tp2 + tp3
  const winPct = total > 0 ? Math.round(gain / total * 100) : 0
  const tauxSL = total > 0 ? Math.round(sl / total * 100) : 0
  const rs     = clos.map(s => signalR(s)).filter((v): v is number => v !== null)
  const rMoyen = rs.length > 0 ? parseFloat((rs.reduce((a, b) => a + b, 0) / rs.length).toFixed(2)) : 0
  return { total, tp1, tp2, tp3, sl, expire, gain, winPct, tauxSL, rMoyen }
}

export function useSmcStats(signauxRef: Ref<Signal[]> | ComputedRef<Signal[]>) {
  // Le rail conviction (depuis 07/09) remplit llm_conviction/llm_raison ;
  // llm_valide (ancien filtre binaire) n'est plus alimenté — null partout.
  const stats = computed(() => {
    const smc = signauxRef.value.filter(s => maj(s.strategie).startsWith('smc'))
    const base = calcStatsSmc(smc)
    const avecConviction = smc.filter(s => s.llm_conviction != null)
    const convictionMoyenne = avecConviction.length > 0
      ? Math.round(avecConviction.reduce((acc, s) => acc + (s.llm_conviction ?? 0), 0) / avecConviction.length)
      : 0
    const tauxFiltrage = smc.length > 0 ? Math.round(avecConviction.length / smc.length * 100) : 0
    const longs  = smc.filter(s => maj(s.direction) === 'long').length
    const shorts = smc.filter(s => maj(s.direction) === 'short').length
    const derniersLlm = avecConviction.slice(0, 5)
    return { ...base, convictionMoyenne, tauxFiltrage, longs, shorts, derniersLlm }
  })

  const signaux = computed(() =>
    signauxRef.value.filter(s => maj(s.strategie).startsWith('smc'))
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

  // Même base de comptage que le loss rate (clôturés tranchés, hors Expire)
  // — la heatmap annonçait « N trades clôturés » sur l'effectif total.
  const sampleSize   = computed(() => Math.max(stats.value.total, 10))
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
