import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService, type Signal, type PredictionML, type ScoreSmc } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'

export const useSignalStore = defineStore('signals', () => {
  const signaux = ref<Signal[]>([])
  const prediction = ref<PredictionML | null>(null)
  const scoreSmc = ref<ScoreSmc | null>(null)
  const chargement = ref(false)
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

  async function chargerScoreSmc(asset: string, timeframe = 'M15') {
    try {
      scoreSmc.value = await apiService.analyseSmc(asset, timeframe)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur SMC'
      erreur.value = msg
    }
  }

  /** Injecte un signal reçu en temps réel (WebSocket Signal Engine) en tête de liste. */
  function ajouterSignalTempsReel(signal: Signal) {
    signaux.value = [signal, ...signaux.value].slice(0, 50)
  }

  return {
    signaux,
    prediction,
    scoreSmc,
    chargement,
    erreur,
    chargerSignaux,
    chargerPrediction,
    chargerScoreSmc,
    ajouterSignalTempsReel,
  }
})
