/// Types du relais mobile — miroir des payloads de /api/mobile/*.

export type TypeAlerte = 'digest' | 'evenement' | 'presse' | 'smc_approche' | 'smc_contact' | 'prix' | 'sante'

export interface Alerte {
  id: number
  ts: number
  type: TypeAlerte
  titre: string
  asset: string | null
  tf: string | null
  png: boolean
}

export interface Synthese {
  resume: string
  ce_qui_compte: string[]
  confiance: number
}

export interface Annonce {
  ts: number
  heure_paris: string
  devise: string
  titre: string
}

export interface AlerteComplete extends Alerte {
  payload: {
    // digest
    annonces?: Annonce[]
    presse?: { titre: string; score: number; theme: string }[]
    synthese?: Synthese | null
    nuit?: { ts: number; type: string; titre: string; asset: string | null; tf: string | null }[]
    date_paris?: string
    // evenement
    heure_paris?: string
    rappel_min?: number
    titre?: string
    // presse
    score?: number
    // smc
    achat?: boolean
    zone_haut?: number
    zone_bas?: number
    distance_atr?: number
    prix?: number
    // sante
    retard_min?: number
    seuil_min?: number
    marche_ouvert?: boolean
    // evenement / commun
    devise?: string
  }
}

export interface Aujourdhui {
  date_paris: string
  annonces: Annonce[]
  presse: { titre: string; score: number; theme: string }[]
  nuit: { ts: number; type: string; titre: string; asset: string | null; tf: string | null }[]
  synthese: Synthese | null
}

export interface EtatServeur {
  ok: boolean
  heure_serveur: number
  muette: { debut: number; fin: number; tz: string }
}
