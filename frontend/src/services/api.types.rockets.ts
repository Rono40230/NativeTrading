/// Types liés à la stratégie Rockets, aux créneaux Straddle et à la collecte de données.
/// Importés et re-exportés depuis api.types.ts — ne pas importer ce fichier directement.

export interface CouvertureDonnees {
  asset: string
  timeframe: string
  count: number
  min_ts: number
  max_ts: number
}

export interface RequeteCollecte {
  assets?: string[]
  timeframes?: string[]
  mois?: number
}

export interface ResultatCollecteItem {
  asset: string
  timeframe: string
  fetched?: number
  inseres?: number
  erreur?: string
}

export interface ResultatCollecte {
  total_inseres: number
  mois: number
  resultats: ResultatCollecteItem[]
}

export interface RocketRecommandation {
  type: string
  description: string
  impact_estime: string
  priorite: 'haute' | 'moyenne' | 'faible'
}

export interface RocketAnalyseLlm {
  id: number
  nb_trades: number
  synthese: string
  meilleur_setup: string | null
  pire_setup: string | null
  recommandations: string // JSON brut
  cree_le: string
}

// ── Straddle ──────────────────────────────────────────────────────────────────

export interface StraddleCreneau {
  id: number
  asset: string
  jour_semaine: number | null
  heure_debut: string
  heure_fin: string
  atr_moyen: number | null
  frequence: number | null
  llm_raison: string | null
  llm_conviction: number | null
  statut: 'a_tester' | 'valide' | 'invalide'
  cree_le: string
  // Précision M5
  timing_optimal: string | null
  fenetre_entree: string | null
  whipsaw_minutes: number | null
  precision_nb_occurrences: number | null
  precision_atr_pic: number | null
}

export interface ReponseAnalyseStraddle {
  creneaux: StraddleCreneau[]
  nb_analyses: number
  nb_retenus: number
  message?: string
}

// ── Monitoring adaptatif Straddle ─────────────────────────────────────────────

export interface StraddlePicLive {
  asset: string
  timeframe: string
  timestamp_pic: number
  ratio_atr: number
  categorie: string
  evenement_nom: string | null
  session_active: string
  kill_zone: boolean
  signal_genere: boolean
  signal_id: number | null
}


export interface StraddleStatCategorie {
  categorie: string
  nb_trades: number
  win_rate: number
  score_llm_win: number | null
  score_llm_lose: number | null
  pnl_r_moyen: number | null
}

export interface StraddleMonitoringData {
  nb_signals_total: number
  nb_feedbacks_clotures: number
  nb_gagnants: number
  nb_perdants: number
  nb_invalides: number
  win_rate_global: number
  pnl_moyen_r: number | null
  derive_detectee: boolean
  par_categorie: StraddleStatCategorie[]
}

export interface StraddleCalibrationRow {
  asset: string
  categorie: string
  score_seuil: number
  ratio_atr_min: number
  fiabilite: string
  nb_trades: number
  win_rate: number
}

// ── Monitoring adaptatif Rockets ──────────────────────────────────────────────

export interface RocketsStatPhase {
  phase: string
  nb_trades: number
  win_rate: number
  conv_win: number | null
  conv_lose: number | null
  pnl_r_moyen: number | null
}

export interface RocketsMonitoringData {
  nb_signals_total: number
  nb_feedbacks_clotures: number
  nb_gagnants: number
  nb_perdants: number
  nb_invalides: number
  win_rate_global: number
  pnl_moyen_r: number | null
  derive_detectee: boolean
  par_phase: RocketsStatPhase[]
}

export interface RocketsCalibrationRow {
  phase: string
  session: string
  score_min: number
  conviction_min: number
  nb_trades: number
  win_rate: number
  fiabilite: string
  invalide: boolean
}

