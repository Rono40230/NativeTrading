// Types miroir du backend indicators::signaux::types
export type NiveauForce = 'faible' | 'moyen' | 'fort'
export type DirectionSignal = 'bullish' | 'bearish' | 'neutre'

export interface SignalIndicateur {
  timestamp: number
  /** "EMA" | "RSI" | "MACD" | "Bollinger" | "ATR" */
  source: string
  /** "golden_cross" | "survente_sortie" | "croisement_haussier" | ... */
  type_signal: string
  direction: DirectionSignal
  force: NiveauForce
  description: string
  valeur: number
  /** Prix de clôture au moment du signal — base pour calcul SL/TP */
  prix_entree: number
}

// ── Constantes UI ──────────────────────────────────────────────────────────────

export const FORCE_ORDRE: Record<NiveauForce, number> = {
  faible: 1,
  moyen: 2,
  fort: 3,
}

export const FORCE_LABEL: Record<NiveauForce, string> = {
  faible: '●',
  moyen: '●●',
  fort: '●●●',
}

// ── Interface filtre ──────────────────────────────────────────────────────────

export interface FiltreSignaux {
  forceMin: NiveauForce
  afficherBullish: boolean
  afficherBearish: boolean
  afficherNeutre: boolean
  /** Sources actives : "EMA", "RSI", "MACD", "Bollinger" */
  sources: string[]
  /** Nombre maximum de signaux récents à afficher (0 = tous) */
  nbSignaux: number
  /** Afficher les lignes SL/TP sur le graphique */
  afficherSlTp: boolean
}

export function filtreDefaut(): FiltreSignaux {
  return {
    forceMin: 'faible',
    afficherBullish: false,
    afficherBearish: false,
    afficherNeutre: false,
    sources: [],
    nbSignaux: 0,
    afficherSlTp: false,
  }
}

export function filtrerSignaux(
  signaux: SignalIndicateur[],
  filtre: FiltreSignaux,
): SignalIndicateur[] {
  const minOrdre = FORCE_ORDRE[filtre.forceMin]
  // Si aucune direction cochée → pas de filtre de direction (afficher toutes)
  const filtreDir = filtre.afficherBullish || filtre.afficherBearish || filtre.afficherNeutre
  const filtres = signaux.filter((s) => {
    if (FORCE_ORDRE[s.force] < minOrdre) return false
    if (filtreDir) {
      if (s.direction === 'bullish' && !filtre.afficherBullish) return false
      if (s.direction === 'bearish' && !filtre.afficherBearish) return false
      if (s.direction === 'neutre'  && !filtre.afficherNeutre)  return false
    }
    if (!filtre.sources.includes(s.source)) return false
    return true
  })
  if (filtre.nbSignaux > 0 && filtres.length > filtre.nbSignaux) {
    // Garder les N plus récents (tri desc timestamp, prendre N, re-trier asc)
    return filtres
      .slice()
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, filtre.nbSignaux)
      .sort((a, b) => a.timestamp - b.timestamp)
  }
  return filtres
}
