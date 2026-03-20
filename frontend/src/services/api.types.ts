export interface Candle {
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
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
  accuracy_rf: number
  accuracy_lstm: number
  nb_echantillons: number
  duree_ms: number
  message: string
}

export interface RequeteAnalyseIA {
  asset: string
  timeframe: string
  direction: string
  score_smc: number
  prix_entree: number
  stop_loss: number
  take_profit: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  confiance_ml: number
}

export interface ReponseAnalyseIA {
  analyse: string
  modele: string
}

export interface ReponseChatIA {
  reponse: string
  modele: string
}

export interface ReponseChartIA {
  analyse: string
  modele: string
}

export interface ImageAvecTF {
  base64: string
  timeframe: string
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
  take_profit: string
  strategie: string
  cree_le: number
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


export interface LigneTendanceKasper {
  tf: string
  tendance: 'haussier' | 'baissier' | null
  valeur_ema_rapide: number | null
  valeur_ema_lente: number | null
}

export type ModeCalculTendance = 'bougie_cloturee' | 'bougie_en_cours'

export interface ReponseTendanceMultiTf {
  asset: string
  ema_rapide: number
  ema_lente: number
  mode_calcul: ModeCalculTendance
  lignes: LigneTendanceKasper[]
}

export interface AssetInfo {
  id: string
  nom: string
  type: 'crypto' | 'metal' | 'forex' | 'indice'
}

export interface AnnonceCalendrier {
  id: string
  date_heure: string
  devise: string
  titre: string
  impact: 'High' | 'Medium'
  precedent: string | null
  prevision: string | null
}

export interface EntiteSentiment {
  nom: string
  prix: number
  variation_pct: number
}

export interface SentimentMarche {
  date: string
  usa: EntiteSentiment[]
  europe: EntiteSentiment[]
  matieres_premieres: EntiteSentiment[]
  cryptos: EntiteSentiment[]
  vix: number | null
}

export type NiveauAlerte = 'critique' | 'important' | 'modere' | 'veille'

export interface ArticleNews {
  id: string
  titre: string
  titre_fr?: string
  source: string
  url: string
  date: string
  score: number
  niveau: NiveauAlerte
}

export interface AlertesNews {
  articles: ArticleNews[]
  score_max: number
  mis_a_jour: string
}

export interface ContenuArticle {
  texte: string
}

export interface TraductionReponse {
  texte_fr: string
}

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
