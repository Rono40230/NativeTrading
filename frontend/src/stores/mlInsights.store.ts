import { defineStore } from 'pinia'
import { ref } from 'vue'
import { mlInsightsApi } from '@/services/api.ml_insights'
import type { AnalyseGlobale, RetainJobState } from '@/services/api.ml_insights'
import { useAlerteStore } from '@/stores/alerte.store'

export const useMlInsightsStore = defineStore('mlInsights', () => {
  const analyse       = ref<AnalyseGlobale | null>(null)
  const chargement    = ref(false)
  const retrainState  = ref<RetainJobState | null>(null)
  let   retrainPollId: ReturnType<typeof setInterval> | null = null

  // (Les « suggestions de paramètres » ont été purgées le 09/09 : elles
  // écrivaient dans des tables qu'aucun moteur ne lisait — ROADMAP §6.)

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

  // (Le déclenchement manuel a été retiré le 23/09 : l'entraînement est
  // automatique et hebdomadaire côté backend — le polling ci-dessus affiche
  // la progression quand un job auto est en cours.)

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
    analyse, chargement, retrainState,
    chargerStats, chargerDernierRetrain,
  }
})
