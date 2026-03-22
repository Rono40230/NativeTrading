import { ref } from 'vue'
import { apiService } from '@/services/api.service'

export type PhaseRocket = 'compression' | 'prelancement' | 'breakout'

export interface SignalRocket {
  symbol:      string
  ticker:      string
  prix:        number
  change1h:    number
  phase:       PhaseRocket
  score:       number
  ratioVolume: number
  atrRatio:    number
  rsi:         number
  support:     number
  target20:    number
  closes:      number[]
}

const POLL_MS = 30_000

export function useVeilleRockets() {
  const signaux        = ref<SignalRocket[]>([])
  const totalCandidats = ref(0)
  const chargement     = ref(false)
  const erreur         = ref(false)
  const progression    = ref(0)
  let intervalle: ReturnType<typeof setInterval> | null = null

  async function scanner() {
    if (chargement.value) return
    chargement.value = true
    erreur.value     = false
    try {
      const raw = await apiService.getRocketsScan()
      if (Array.isArray(raw)) {
        // Ancien format : tableau direct
        signaux.value        = raw as SignalRocket[]
        totalCandidats.value = 0
      } else {
        // Nouveau format : { signaux, totalCandidats }
        const obj = raw as { signaux: SignalRocket[]; totalCandidats: number }
        signaux.value        = obj.signaux ?? []
        totalCandidats.value = obj.totalCandidats ?? 0
      }
      progression.value = 100
    } catch {
      erreur.value = true
    } finally {
      chargement.value = false
    }
  }

  function demarrer() {
    scanner()
    intervalle = setInterval(scanner, POLL_MS)
  }

  function arreter() {
    if (intervalle !== null) { clearInterval(intervalle); intervalle = null }
  }

  return { signaux, totalCandidats, chargement, erreur, progression, demarrer, arreter }
}

