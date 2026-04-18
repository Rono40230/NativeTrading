import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { SentimentMarche } from '@/services/api.types'

export const useSentimentStore = defineStore('sentiment', () => {
  const data = ref<SentimentMarche | null>(null)
  const chargement = ref(false)
  const erreur = ref(false)
  let _interval: ReturnType<typeof setInterval> | null = null

  async function charger() {
    if (chargement.value) return
    chargement.value = true
    try {
      data.value = await apiService.obtenirSentimentMarche()
      erreur.value = false
    } catch {
      if (!data.value) erreur.value = true
    } finally {
      chargement.value = false
    }
  }

  function demarrer() {
    charger()
    if (!_interval) _interval = setInterval(charger, 60_000)
  }

  function arreter() {
    if (_interval) { clearInterval(_interval); _interval = null }
  }

  return { data, chargement, erreur, charger, demarrer, arreter }
})
