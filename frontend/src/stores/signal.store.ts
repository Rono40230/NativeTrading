import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService, type Signal, type PredictionML, type BacktestResults } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'

export const useSignalStore = defineStore('signals', () => {
  const signaux = ref<Signal[]>([])
  const prediction = ref<PredictionML | null>(null)
  const backtest = ref<BacktestResults | null>(null)
  const chargement = ref(false)
  const chargementBacktest = ref(false)
  const erreur = ref<string | null>(null)

  async function chargerSignaux(limit = 20) {
    chargement.value = true
    erreur.value = null
    try {
      signaux.value = await apiService.getSignaux(limit)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur réseau'
      erreur.value = msg
    } finally {
      chargement.value = false
    }
  }

  async function chargerPrediction(asset: string, timeframe = 'M15') {
    try {
      prediction.value = await apiService.predictML(asset, timeframe)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur ML'
      erreur.value = msg
    }
  }

  async function lancerBacktest(
    asset: string,
    timeframe = 'M15',
    capital?: number,
    limit = 500
  ) {
    const capitalEffectif = capital ?? useSettingsStore().capitalDepart
    chargementBacktest.value = true
    erreur.value = null
    try {
      backtest.value = await apiService.runBacktest(asset, timeframe, capitalEffectif, limit)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur backtest'
      erreur.value = msg
    } finally {
      chargementBacktest.value = false
    }
  }

  return {
    signaux,
    prediction,
    backtest,
    chargement,
    chargementBacktest,
    erreur,
    chargerSignaux,
    chargerPrediction,
    lancerBacktest,
  }
})
