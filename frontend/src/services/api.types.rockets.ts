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

export interface RocketSignalSave {
  ticker: string; phase: string; score: number
  prix_entree: number; stop_loss: number; target: number
  ratio_volume: number; atr_ratio: number; rsi: number
}

export interface RocketSignalHistorique {
  id: number; ticker: string; phase: string; score: number
  prix_entree: number; stop_loss: number
  target: number; target2: number | null; target3: number | null
  statut: string
  verdict: string | null; prix_verdict: number | null
  prix_peak: number | null; atr14: number | null; rsi: number
  ratio_volume: number; atr_ratio: number
  llm_valide: number | null; llm_conviction: number | null; llm_raison: string | null
  trailing_coeff: number | null; pct_tp1: number; pct_tp2: number; pct_trailing: number
  cree_le: string; maj_le: string | null
  pnl_r: number | null; gagnant: number | null
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

export interface RocketsConfig {
  score_min: number
  phases_actives: string[]
  rsi_max: number
  rsi_min: number
  ratio_volume_min: number
  vol_marche_min: number
  vente_partielle?: boolean
  sl_mult: number
  trailing_coeff_min: number
  trailing_coeff_max: number
  seuil_score_faible: number
  seuil_score_fort: number
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
  backtest_winrate: number | null
  backtest_profit_factor: number | null
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
