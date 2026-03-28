export interface Candle {
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface EquityPoint {
  timestamp: number
  capital: number
}

export interface BacktestResults {
  total_trades: number
  winning_trades: number
  losing_trades: number
  win_rate: number
  capital_initial: number
  capital_final: number
  roi_pct: number
  profit_net: number
  sharpe_ratio: number
  max_drawdown_pct: number
  profit_factor: number
  // Pyramidalisation SMC (0 pour Straddle)
  nb_tp1: number
  nb_tp2: number
  nb_tp3: number
  nb_sl: number
  nb_expirations: number
  // Nombre de Straddles posés (= total_trades / 2)
  nb_straddles: number
  equity_curve?: EquityPoint[]
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


