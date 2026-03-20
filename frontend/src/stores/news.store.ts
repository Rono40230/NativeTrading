import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { AlertesNews } from '@/services/api.types'

export const useNewsStore = defineStore('news', () => {
  const data = ref<AlertesNews | null>(null)
  const chargement = ref(false)
  const erreur = ref(false)
  let intervalle: ReturnType<typeof setInterval> | null = null

  const scoreMax = computed(() => data.value?.score_max ?? 0)

  /** Articles filtrés : uniquement ceux avec un score ≥ 40 (modéré ou plus) */
  const articlesPertinents = computed(() =>
    (data.value?.articles ?? []).filter(a => a.score >= 40).slice(0, 8),
  )

  /** true si une alerte critique est en cours (score ≥ 80) */
  const alerteCritique = computed(() => scoreMax.value >= 80)

  /** Premier article critique pour le bandeau */
  const articleCritique = computed(() =>
    alerteCritique.value
      ? (data.value?.articles ?? []).find(a => a.niveau === 'critique') ?? null
      : null,
  )

  async function charger() {
    if (chargement.value) return
    chargement.value = true
    erreur.value = false
    try {
      data.value = await apiService.obtenirAlertes()
    } catch {
      erreur.value = !data.value
    } finally {
      chargement.value = false
    }
  }

  function demarrerPolling() {
    charger()
    intervalle = setInterval(charger, 120_000) // toutes les 2 min
  }

  function arreterPolling() {
    if (intervalle !== null) {
      clearInterval(intervalle)
      intervalle = null
    }
  }

  return {
    data,
    chargement,
    erreur,
    scoreMax,
    articlesPertinents,
    alerteCritique,
    articleCritique,
    charger,
    demarrerPolling,
    arreterPolling,
  }
})
