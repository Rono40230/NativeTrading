import { defineStore } from 'pinia'
import { ref } from 'vue'
import { acceptHMRUpdate } from 'pinia'
import { backtestApi } from '@/services/api.backtest'
import type { BacktestResult, Recommandation } from '@/services/api.backtest'
import { useAlerteStore } from '@/stores/alerte.store'

export interface ConfigBacktest {
  asset:      string
  timeframe:  string
  nb_jours:   number
  capital:    number
  risque_pct: number
}

export const useBacktestStore = defineStore('backtest', () => {
  const resultStraddle          = ref<BacktestResult | null>(null)
  const resultSmc               = ref<BacktestResult | null>(null)
  const recoStraddle            = ref<Recommandation[]>([])
  const recoSmc                 = ref<Recommandation[]>([])
  const duree_straddle          = ref<number | null>(null)
  const duree_smc               = ref<number | null>(null)
  const chargement              = ref(false)

  async function lancerComparaison(config: ConfigBacktest): Promise<void> {
    chargement.value    = true
    resultStraddle.value = null
    resultSmc.value      = null
    recoStraddle.value   = []
    recoSmc.value        = []
    try {
      const [repS, repM] = await Promise.all([
        backtestApi.lancer({ ...config, strategie: 'straddle' }),
        backtestApi.lancer({ ...config, strategie: 'smc' }),
      ])
      resultStraddle.value = repS.result
      recoStraddle.value   = repS.recommandations
      duree_straddle.value = repS.duree_ms
      resultSmc.value      = repM.result
      recoSmc.value        = repM.recommandations
      duree_smc.value      = repM.duree_ms
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur backtest'
      useAlerteStore().afficherErreur(msg)
    } finally {
      chargement.value = false
    }
  }

  return {
    resultStraddle, resultSmc,
    recoStraddle, recoSmc,
    duree_straddle, duree_smc,
    chargement,
    lancerComparaison,
  }
})

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useBacktestStore, import.meta.hot))
}
