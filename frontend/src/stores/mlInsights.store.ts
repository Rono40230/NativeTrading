import { defineStore } from 'pinia'
import { ref } from 'vue'
import { mlInsightsApi } from '@/services/api.ml_insights'
import type { AnalyseGlobale, SuggestionParams, SuggestionsResponse, RetainJobState } from '@/services/api.ml_insights'
import { useAlerteStore } from '@/stores/alerte.store'

export const useMlInsightsStore = defineStore('mlInsights', () => {
  const analyse       = ref<AnalyseGlobale | null>(null)
  const suggestions   = ref<SuggestionParams[]>([])
  const historique    = ref<SuggestionsResponse['historique']>([])
  const chargement    = ref(false)
  const application   = ref(false)
  const retrainState  = ref<RetainJobState | null>(null)
  let   retrainPollId: ReturnType<typeof setInterval> | null = null

  async function chargerStats() {
    chargement.value = true
    try {
      analyse.value = await mlInsightsApi.getStats()
    } catch {
      useAlerteStore().afficherErreur('Impossible de charger les stats ML')
    } finally {
      chargement.value = false
    }
  }

  async function chargerSuggestions() {
    chargement.value = true
    try {
      const res        = await mlInsightsApi.getSuggestions()
      suggestions.value = res.suggestions
      historique.value  = res.historique
    } catch {
      useAlerteStore().afficherErreur('Impossible de charger les suggestions ML')
    } finally {
      chargement.value = false
    }
  }

  async function appliquer(s: SuggestionParams): Promise<boolean> {
    application.value = true
    try {
      await mlInsightsApi.appliquerSuggestion(s)
      useAlerteStore().afficherSucces(`✅ ${s.strategie} ${s.param_name} → ${s.valeur_suggeree}`)
      await chargerSuggestions()
      return true
    } catch {
      useAlerteStore().afficherErreur('Erreur lors de l\'application de la suggestion')
      return false
    } finally {
      application.value = false
    }
  }

  async function chargerDernierRetrain() {
    try {
      const s = await mlInsightsApi.getRetrainLast()
      retrainState.value = s
      // Reprendre le polling si un job est encore en cours (ex: après navigation/reload)
      if (s.en_cours && s.job_id) {
        _demarrerPoll(s.job_id)
      }
    } catch {
      // Pas encore de job — silencieux
    }
  }

  async function declencherRetrain() {
    try {
      const res = await mlInsightsApi.postRetrain()
      useAlerteStore().afficherSucces('🔁 Réentraînement lancé')
      _demarrerPoll(res.job_id)
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { error?: string } } })?.response?.data?.error
      useAlerteStore().afficherErreur(msg ?? 'Impossible de lancer le réentraînement')
    }
  }

  function _demarrerPoll(jobId: string) {
    if (retrainPollId !== null) clearInterval(retrainPollId)
    retrainPollId = setInterval(async () => {
      try {
        const s = await mlInsightsApi.getRetrainStatus(jobId)
        retrainState.value = s
        if (!s.en_cours) {
          clearInterval(retrainPollId!)
          retrainPollId = null
        }
      } catch {
        clearInterval(retrainPollId!)
        retrainPollId = null
      }
    }, 1000)
  }

  return {
    analyse, suggestions, historique, chargement, application, retrainState,
    chargerStats, chargerSuggestions, appliquer, chargerDernierRetrain, declencherRetrain,
  }
})
