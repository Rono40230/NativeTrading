import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { AlertesNews } from '@/services/api.types'

export type ThemeNews = 'tous' | 'macro' | 'crypto' | 'metaux'

export const useNewsStore = defineStore('news', () => {
  const data = ref<AlertesNews | null>(null)
  const chargement = ref(false)
  const erreur = ref(false)
  const themeActif = ref<ThemeNews>('tous')
  let intervalle: ReturnType<typeof setInterval> | null = null

  const scoreMax = computed(() => data.value?.score_max ?? 0)

  /** Tous les articles avec score ≥ 40 */
  const articlesFiltered = computed(() =>
    (data.value?.articles ?? []).filter(a => a.score >= 40),
  )

  /** Articles affichés selon l'onglet actif, max 15 */
  const articlesPertinents = computed(() => {
    const base = articlesFiltered.value
    const filtres = themeActif.value === 'tous'
      ? base
      : base.filter(a => a.theme === themeActif.value)
    return filtres.slice(0, 15)
  })

  /** Score max par thème (pour sélectionner l'onglet par défaut) */
  const scoreMaxParTheme = computed(() => {
    const res: Record<ThemeNews, number> = { tous: 0, macro: 0, crypto: 0, metaux: 0 }
    for (const a of articlesFiltered.value) {
      res.tous = Math.max(res.tous, a.score)
      if (a.theme === 'macro') res.macro = Math.max(res.macro, a.score)
      else if (a.theme === 'crypto') res.crypto = Math.max(res.crypto, a.score)
      else if (a.theme === 'metaux') res.metaux = Math.max(res.metaux, a.score)
    }
    return res
  })

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
      // Sélectionner automatiquement le thème dominant si toujours sur 'tous'
      if (themeActif.value === 'tous') {
        const s = scoreMaxParTheme.value
        const dominant = (['macro', 'crypto', 'metaux'] as ThemeNews[]).reduce(
          (best, t) => (s[t] > s[best] ? t : best),
          'tous' as ThemeNews,
        )
        themeActif.value = dominant
      }
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
    themeActif,
    scoreMax,
    scoreMaxParTheme,
    articlesPertinents,
    alerteCritique,
    articleCritique,
    charger,
    demarrerPolling,
    arreterPolling,
  }
})
