/// Types du relais mobile — miroir des payloads de /api/mobile/*.

export type TypeAlerte = 'digest' | 'evenement' | 'presse' | 'signal' | 'smc_approche' | 'smc_contact' | 'prix' | 'sante'

export interface Alerte {
  id: number
  ts: number
  type: TypeAlerte
  titre: string
  asset: string | null
  tf: string | null
  png: boolean
  /** Alertes SMC uniquement (absent sinon) : côté achat. */
  achat?: boolean
  /** Alertes prix uniquement (absent sinon) : niveau armé. */
  niveau?: number | null
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

/** Ligne « pendant la nuit » : une position ouverte, par stratégie (19/10). */
export interface PositionNuit {
  ts: number
  titre: string
}

export interface AlerteComplete extends Alerte {
  payload: {
    // digest
    annonces?: Annonce[]
    presse?: { titre: string; score: number; theme: string }[]
    synthese?: Synthese | null
    nuit?: PositionNuit[]
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
  nuit: PositionNuit[]
}

export interface EtatServeur {
  ok: boolean
  heure_serveur: number
  muette: { debut: number; fin: number; tz: string }
}

/** Onglet « Revue de presse » (owner 19→22/10) : 4 sections × 10
 * articles, chaque article cliquable vers sa page source. */
export interface ArticlePresse {
  titre: string
  url: string
  score: number
  theme: string
  section: 'macro' | 'cryptos' | 'metaux' | 'techno'
  ajoute_le: number
}

export interface BriefPresse {
  contenu: string
  nb_articles: number
  genere_le: number
}

export interface Presse {
  articles: ArticlePresse[]
  brief: BriefPresse | null
}

/** Page « Prix » : niveaux armés + déclenchées 24 h (owner 20/10). */
export interface NiveauPrix {
  asset: string
  prix: number
  sens: string
  cree_le: number
}

export interface PrixPage {
  niveaux: NiveauPrix[]
  declenchees: Alerte[]
}

/** Ligne de /api/alertes-prix (app principale — repli niveaux armés). */
export interface AlertePrixPrincipale {
  id: number
  asset: string
  prix: number
  sens: string
  active: boolean
  cree_le: number
}

/** Rapport d'activité (journal 24 h, miroir exact de la carte 📊 du
 * dashboard : Émis / Clôturés / Σ±R par stratégie) + signaux en cours. */
export interface RapportStrategie {
  id: string
  nom: string
  icone: string
  emis: number
  clotures: number
  r: number
}

export interface SignalEnCours {
  strategie: string
  asset: string
  sens: string
  /** R flottant net (Double = somme des jambes). */
  r: number
  dollars: number | null
  heure: number
}

export interface Activite {
  rapports: RapportStrategie[]
  en_cours: SignalEnCours[]
}

/** Bandeau marché + zones + calendrier (owner 21/10). */
export interface ActifMarche {
  asset: string
  prix: number
  variation: number
}

export interface EvenementCal {
  ts: number
  heure_paris: string
  devise: string
  titre: string
}

export interface ZoneBande {
  asset: string
  tf: string
  achat: boolean
  zone_haut: number
  zone_bas: number
  distance_atr: number
  distance_pct: number
  prix: number
}
