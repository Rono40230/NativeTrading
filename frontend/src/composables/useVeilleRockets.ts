import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketSignalHistorique } from '@/services/api.types'

export type PhaseRocket = 'compression' | 'prelancement' | 'breakout'

export interface SignalRocket {
  symbol:                string
  ticker:                string
  prix:                  number
  change1h:              number
  phase:                 PhaseRocket
  score:                 number
  ratioVolume:           number
  atrRatio:              number
  atr14:                 number
  rsi:                   number
  support:               number
  target20:              number
  closes:                number[]
  trailingCoeff:         number
  sl:                    number
  tp1:                   number
  tp2:                   number
  tp3Trigger:            number
  entreeLimite:          number
  entreeStop:            number
  niveauInvalidation:    number
  typeEntreeRec:         string  // "limite" | "stop"
  nbBougiesCompression:  number
  tendanceHaussiere:     boolean
  volumeSeche:           number  // <0.75 = assèchement VCP valide
  ratioCorps:            number  // qualité bougie (0.0–1.0)
}

const POLL_MS = 30_000

// ── Détection nouveaux signaux (partagée entre instances) ─────────────────────
const dernierIdConnu = ref<number | null>(null)
const signalAlerte   = ref<RocketSignalHistorique | null>(null)

async function verifierNouveauxSignaux() {
  try {
    const liste = await apiService.rocketsActifs()
    // Filtrer only attente/ouvert (signaux actifs, pas clôturés)
    const actifs = liste.filter(s => s.statut === 'attente' || s.statut === 'ouvert')
    if (actifs.length === 0) return
    const plusRecent = actifs[0]
    // Premier chargement : mémoriser sans alerter
    if (dernierIdConnu.value === null) {
      dernierIdConnu.value = plusRecent.id
      return
    }
    // Nouveau signal détecté
    if (plusRecent.id > dernierIdConnu.value) {
      dernierIdConnu.value = plusRecent.id
      signalAlerte.value = plusRecent
    }
  } catch {
    // Silencieux
  }
}

export function useRocketAlerte() {
  return { signalAlerte }
}

export function useVeilleRockets() {
  const signaux        = ref<SignalRocket[]>([])
  const totalCandidats = ref(0)
  const chargement     = ref(false)
  const erreur         = ref(false)
  const progression    = ref(0)
  let intervalle: ReturnType<typeof setInterval> | null = null

  async function scanner(silencieux = false) {
    if (chargement.value) return
    if (!silencieux || signaux.value.length === 0) chargement.value = true
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
    intervalle = setInterval(() => {
      scanner(true)
    }, POLL_MS)
  }

  function arreter() {
    if (intervalle !== null) { clearInterval(intervalle); intervalle = null }
  }

  return { signaux, totalCandidats, chargement, erreur, progression, demarrer, arreter }
}

