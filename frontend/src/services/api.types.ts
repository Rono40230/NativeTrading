export type { Candle } from '../generated/Candle'
export type { Timeframe } from '../generated/Timeframe'
export type { Direction } from '../generated/Direction'
export type { Asset } from '../generated/Asset'

// PredictionML : généré depuis api::handlers::ReponsePrediction (source de vérité Rust).
// Re-export aliasé pour préserver le nom historique côté frontend.
export type { ReponsePrediction as PredictionML } from '../generated/PredictionML'

export interface ReponseEntrainement {
  success: boolean
  accuracy_xgb: number
  accuracy_lstm: number
  accuracy_finale: number
  nb_echantillons: number
  duree_ms: number
  derive_detectee: boolean
  message: string
}

export interface HistoriqueEntrainement {
  id: number
  cree_le: number
  asset: string
  timeframe: string
  nb_bougies: number
  accuracy_xgb: number
  accuracy_lstm: number
  accuracy_finale: number
  duree_ms: number
  derive_detectee: boolean
}

export interface HistoriqueML {
  historique: HistoriqueEntrainement[]
  derive_detectee: boolean
  seuil_derive: number
  nb_entrainements: number
}

export interface StatutIA {
  ollama_disponible: boolean
  modele: string
  url: string
}

export interface Signal {
  id: string
  asset: string
  timeframe: string
  direction: string
  score: number
  prix_entree: number
  stop_loss: number
  take_profit: number[]   // [tp1, tp2, tp3]
  strategie: string
  statut: string          // 'Actif' | 'Fermé' | 'Annulé'
  verdict: string | null  // null | 'SL' | 'TP1' | 'TP2' | 'TP3' | 'expire'
  prix_verdict: number | null
  ferme_le: number | null
  cree_le: number
  llm_valide: number | null      // 1=validé | 0=rejeté | null=LLM indispo
  llm_conviction: number | null  // 0–100
  llm_raison: string | null
  llm_sl_suggere: number | null
  llm_tp1_suggere: number | null
  // Jambe SHORT — uniquement renseigné pour les signaux Straddle (direction === 'Both')
  sl_short: number | null
  take_profit_short: number[] | null  // [tp1_short, tp2_short, tp3_short]
  // État courant des deux jambes Straddle (mis à jour par le job toutes les 5 min)
  sl_long_effectif: number | null     // SL courant jambe LONG (null = SL d'origine)
  sl_short_effectif: number | null    // SL courant jambe SHORT (null = sl_short d'origine)
  tps_long_atteints: string[] | null  // ex. ['tp1'] ou ['tp1','tp2']
  tps_short_atteints: string[] | null
  /** Timestamp Unix (UTC) de l'heure d'entrée cible (événement éco). null = entrée immédiate. */
  heure_entree: number | null
}

export interface StatutSignalEngine {
  actif: boolean
  prochain_cycle_dans_secs: number
  signaux_24h: number
  assets_surveilles: number
  timeframes: string[]
  intervalle_secs: number
}

export type { ScoreSmc } from '../generated/ScoreSmc'

export type { PointSerie, ZoneOb, ZoneImbalance, ZoneFvgBpr, ZoneBpr, ZoneIfvg, NiveauxFibonacci, ResultatTendance, NiveauLiquidite, DeviationAsie, RangeAsie, NiveauForceSignal, DirectionSignal, SignalIndicateur, ReponseIndicators, IndicatorsParams } from './api.types.indicators'
export type { PatternHoraire, ReponsePatternsVolatilite, RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA, ImageAvecTF, LigneTendanceKasper, ModeCalculTendance, ReponseTendanceMultiTf, AssetInfo, AnnonceCalendrier, FearGreedData, EntiteSentiment, SentimentMarche, NiveauAlerte, ArticleNews, AlertesNews, ContenuArticle, TraductionReponse } from './api.types.marche'
export type { CouvertureDonnees, RequeteCollecte, ResultatCollecteItem, ResultatCollecte, RocketSignalSave, RocketSignalHistorique, RocketRecommandation, RocketAnalyseLlm, RocketsConfig, StraddleCreneau, ReponseAnalyseStraddle, StraddlePicLive, AnnonceImminente, StraddleVolatiliteLive, StraddleDevSeedResponse, StraddleDevSignalResponse, StraddleStatCategorie, StraddleMonitoringData, StraddleCalibrationRow, RocketsStatPhase, RocketsMonitoringData, RocketsCalibrationRow, StraddleSeuilsEffectifs, RocketsSeuilsEffectifs } from './api.types.rockets'

// ── Signal IA (POST /api/ia/signal) ──────────────────────────────────────────
export interface RequeteSignalIA {
  asset: string
  timeframe: string
  score_smc: number
  prix_actuel: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  confiance_ml: number
  atr: number
  kill_zone_active?: boolean
  sweep_detecte?: boolean
}

export interface ReponseSignalIA {
  signal: Signal | null
  score_confiance: number   // 0–10
  niveau_invalidation: number
  confluences: string[]
  raisonnement: string
  modele: string
}

// ── Types ML Straddle adaptatif ───────────────────────────────────────────────

// ── SMC Directionnel ML ───────────────────────────────────────────────────────

export interface SmcStatCategorie {
  categorie: string
  nb_trades: number
  win_rate: number
  conv_win: number | null
  conv_lose: number | null
  pnl_r_moyen: number | null
}

export interface SmcMonitoringData {
  nb_signals_total: number
  nb_feedbacks_clotures: number
  nb_gagnants: number
  nb_perdants: number
  nb_invalides: number
  win_rate_global: number
  pnl_moyen_r: number | null
  derive_detectee: boolean
  par_categorie: SmcStatCategorie[]
}

export interface SmcCalibrationRow {
  asset: string
  timeframe: string
  categorie: string
  score_smc_seuil: number
  conviction_seuil: number
  nb_trades: number
  win_rate: number
  fiabilite: string
  invalide: boolean
}

export interface AssetParams {
  asset: string
  valeur_pips: number
  sl_pips: number
  pip_to_points: number
  risque_pct: number
  lot_min: number
  lot_max: number
  taille_pip: number
  // champs calculés côté frontend (présentation uniquement)
  investi?: number
  lot?: number
}

export interface PrecisionHoraire {
  ok: boolean
  timing_optimal?: string
  fenetre_entree?: string
  whipsaw_minutes?: number
  nb_occurrences?: number
  atr_pic?: number
  session?: string
  raison?: string
  message?: string
}

// ── Barèmes & seuils effectifs pour les pages Définition ─────────────────────

export interface SmcBaremes {
  tendance: number
  order_block: number
  ifvg: number
  imbalance: number
  fibonacci: number
  total_max: number
}
