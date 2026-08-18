import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { SentimentMarche, SentimentComposite } from '@/services/api.types'

export const useSentimentStore = defineStore('sentiment', () => {
  // Sentiment prix/variation des marchés (endpoint existant).
  const data = ref<SentimentMarche | null>(null)
  const chargement = ref(false)
  const erreur = ref(false)

  // Sentiment composite 0-100 par classe (nouveau endpoint).
  const composite = ref<SentimentComposite | null>(null)

  let _interval: ReturnType<typeof setInterval> | null = null
  let _intervalComposite: ReturnType<typeof setInterval> | null = null

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

  async function chargerComposite() {
    try {
      composite.value = await apiService.obtenirSentimentComposite()
    } catch {
      // dégradation silencieuse : le composite est optionnel
    }
  }

  function demarrer() {
    charger()
    chargerComposite()
    // Référence veille : le contenu est figé dans la journée — un refresh
    // léger ne sert qu'à rattraper le changement de jour si l'app reste ouverte.
    if (!_interval) _interval = setInterval(charger, 5 * 60_000)
    if (!_intervalComposite) _intervalComposite = setInterval(chargerComposite, 5 * 60_000)
  }

  function arreter() {
    if (_interval) { clearInterval(_interval); _interval = null }
    if (_intervalComposite) { clearInterval(_intervalComposite); _intervalComposite = null }
  }

  return { data, chargement, erreur, composite, charger, chargerComposite, demarrer, arreter }
})
