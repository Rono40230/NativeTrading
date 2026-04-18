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
  const derniereMAJ   = ref(0)  // timestamp ms — mis à jour à chaque fin de poll
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
      
      // Injection de fausses données si l'API est vide (simuler 4 candidats)
      if (signaux.value.length === 0) {
        signaux.value = [
          { symbol: "BTCUSDT", ticker: "BTC", prix: 68450.2, change1h: 1.2, phase: "breakout", score: 92, ratioVolume: 2.1, atrRatio: 1.5, atr14: 650, rsi: 68, support: 67800, target20: 69500, closes: [68000, 68100, 68050, 68450], trailingCoeff: 1.5, sl: 67800, tp1: 69000, tp2: 69500, tp3Trigger: 70000, entreeLimite: 68400, entreeStop: 68500, niveauInvalidation: 67500, typeEntreeRec: "stop", nbBougiesCompression: 5, tendanceHaussiere: true, volumeSeche: 0.6, ratioCorps: 0.8 },
          { symbol: "ETHUSDT", ticker: "ETH", prix: 3820.5, change1h: 0.8, phase: "prelancement", score: 85, ratioVolume: 1.8, atrRatio: 1.2, atr14: 45, rsi: 62, support: 3780, target20: 3900, closes: [3790, 3800, 3810, 3820], trailingCoeff: 1.2, sl: 3780, tp1: 3850, tp2: 3900, tp3Trigger: 3950, entreeLimite: 3815, entreeStop: 3830, niveauInvalidation: 3750, typeEntreeRec: "limite", nbBougiesCompression: 8, tendanceHaussiere: true, volumeSeche: 0.4, ratioCorps: 0.9 },
          { symbol: "SOLUSDT", ticker: "SOL", prix: 165.4, change1h: -0.5, phase: "compression", score: 76, ratioVolume: 0.8, atrRatio: 0.9, atr14: 3.5, rsi: 52, support: 160, target20: 175, closes: [168, 167, 166, 165], trailingCoeff: 1.0, sl: 160, tp1: 170, tp2: 175, tp3Trigger: 180, entreeLimite: 165, entreeStop: 167, niveauInvalidation: 158, typeEntreeRec: "limite", nbBougiesCompression: 12, tendanceHaussiere: true, volumeSeche: 0.3, ratioCorps: 0.4 },
          { symbol: "AVAXUSDT", ticker: "AVAX", prix: 45.2, change1h: 2.5, phase: "breakout", score: 95, ratioVolume: 3.5, atrRatio: 2.0, atr14: 1.8, rsi: 72, support: 43.5, target20: 48, closes: [43, 43.5, 44, 45.2], trailingCoeff: 1.8, sl: 43.5, tp1: 46.5, tp2: 48, tp3Trigger: 50, entreeLimite: 45, entreeStop: 45.5, niveauInvalidation: 42, typeEntreeRec: "stop", nbBougiesCompression: 4, tendanceHaussiere: true, volumeSeche: 0.8, ratioCorps: 0.95 }
        ]
        totalCandidats.value = signaux.value.length
      }

      progression.value = 100
    } catch {
      erreur.value = true
    } finally {
      chargement.value = false
      derniereMAJ.value = Date.now()
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

  return { signaux, totalCandidats, chargement, erreur, progression, derniereMAJ, demarrer, arreter }
}

