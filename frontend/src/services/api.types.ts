export interface Candle {
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface PredictionML {
  asset: string
  direction: string
  confiance: number
  est_confiant: boolean
  modele_pret: boolean
}

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
}

export interface StatutSignalEngine {
  actif: boolean
  prochain_cycle_dans_secs: number
  signaux_24h: number
  assets_surveilles: number
  timeframes: string[]
  intervalle_secs: number
}

export interface ScoreSmc {
  total: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  direction: string
  confluence: boolean
  kill_zone_active: boolean
  sweep_detecte: boolean
}

export type { PointSerie, ZoneOb, ZoneImbalance, ZoneFvgBpr, ZoneBpr, ZoneIfvg, NiveauxFibonacci, ResultatTendance, NiveauLiquidite, DeviationAsie, RangeAsie, NiveauForceSignal, DirectionSignal, SignalIndicateur, ReponseIndicators, IndicatorsParams } from './api.types.indicators'
export type { PatternHoraire, ReponsePatternsVolatilite, RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA, ImageAvecTF, LigneTendanceKasper, ModeCalculTendance, ReponseTendanceMultiTf, AssetInfo, AnnonceCalendrier, FearGreedData, EntiteSentiment, SentimentMarche, NiveauAlerte, ArticleNews, AlertesNews, ContenuArticle, TraductionReponse } from './api.types.marche'
export type { CouvertureDonnees, RequeteCollecte, ResultatCollecteItem, ResultatCollecte, RocketSignalSave, RocketSignalHistorique, RocketRecommandation, RocketAnalyseLlm, RocketsConfig, StraddleCreneau, ReponseAnalyseStraddle } from './api.types.rockets'

// ── Types ML Straddle adaptatif ───────────────────────────────────────────────

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

export interface AnnonceImminente {
  nom: string | null
  devise: string | null
  impact: string
  dans_min: number
}

export interface StraddleVolatiliteLive {
  pics: StraddlePicLive[]
  resume: {
    pics_2h: number
    assets_actifs: string[]
    annonces_prochaines_90min: AnnonceImminente[]
  }
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
  total_trades: number
  nb_gagnants: number
  win_rate_global: number
  pnl_r_total: number | null
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

// ── Rockets ML adaptatif ──────────────────────────────────────────────────────

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
