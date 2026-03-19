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

export interface ScoreSmc {
  total: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  direction: string
  confluence: boolean
}

export interface PointSerie {
  time: number
  value: number
}

export interface ZoneOb {
  prix_haut: number
  prix_bas: number
  direction: string
  force: number
  timestamp: number
}

export interface ZoneImbalance {
  haut: number
  bas: number
  type_zone: 'FvgBull' | 'FvgBear' | 'OgBull' | 'OgBear'
  remplie: boolean
  timestamp: number
}

/** Zone FVG unifiée — type_zone : "FvgBull" | "FvgBear" | "Bpr" */
export interface ZoneFvgBpr {
  type_zone: string
  haut: number
  bas: number
  /** Renseignés seulement pour type_zone === "Bpr" */
  bull_haut: number
  bull_bas: number
  bear_haut: number
  bear_bas: number
  timestamp: number
}
/** Alias de compatibilité */
export type ZoneBpr = ZoneFvgBpr

export interface ZoneIfvg {
  prix_haut: number
  prix_bas: number
  direction: string
  timestamp: number
  timestamp_inversion: number
}

export interface NiveauxFibonacci {
  swing_haut: number
  swing_bas: number
  niveau_236: number
  niveau_382: number
  niveau_500: number
  niveau_618: number
  niveau_786: number
}

export interface ResultatTendance {
  direction: string
  dernier_sommet: number
  dernier_creux: number
  force: number
}

export interface NiveauLiquidite {
  prix: number
  cote: 'BSL' | 'SSL'
    /** "swing" | "asie" | "daily" */
  categorie: string
  equal: boolean
  swepe: boolean
  /** Unix secondes \u2014 bougie de formation (bord gauche de la ligne) */
  timestamp: number
}
export interface DeviationAsie {
  prix: number
  direction: 'H' | 'L'
  numero: number
}

export interface RangeAsie {
  timestamp_debut: number
  timestamp_fin: number
  haut: number
  bas: number
  deviations: DeviationAsie[]
}
export type NiveauForceSignal = 'faible' | 'moyen' | 'fort'
export type DirectionSignal = 'bullish' | 'bearish' | 'neutre'

export interface SignalIndicateur {
  timestamp: number
  source: string
  type_signal: string
  direction: DirectionSignal
  force: NiveauForceSignal
  description: string
  valeur: number
  prix_entree: number
}

export interface ReponseIndicators {
  ema?: PointSerie[]
  rsi?: PointSerie[]
  atr?: PointSerie[]
  macd?: { macd: PointSerie[]; signal: PointSerie[]; histogramme: PointSerie[] }
  bollinger?: { haute: PointSerie[]; milieu: PointSerie[]; basse: PointSerie[] }
  order_blocks?: ZoneOb[]
  ifvg?: ZoneIfvg[]
  bpr?: ZoneFvgBpr[]
  imbalance?: ZoneImbalance[]
  fibonacci?: NiveauxFibonacci
  tendance?: ResultatTendance
  liquidites?: NiveauLiquidite[]
  range_asie?: RangeAsie[]
  signaux?: SignalIndicateur[]
  atr_valeurs?: PointSerie[]
}

export interface IndicatorsParams {
  asset: string
  tf?: string
  ema?: boolean
  rsi?: boolean
  macd?: boolean
  bollinger?: boolean
  atr?: boolean
  ema_periode?: number
  ema_ma_type?: 'ema' | 'sma'
  rsi_periode?: number
  macd_rapide?: number
  macd_lente?: number
  macd_signal?: number
  bollinger_periode?: number
  bollinger_stddev?: number
  bollinger_ma_type?: string
  atr_periode?: number
  smc_ob?: boolean
  smc_ob_sensitivity?: number
  smc_ob_mitigation?: string
  smc_ifvg?: boolean
  smc_ifvg_show_last?: number
  smc_ifvg_signal_pref?: string
  smc_ifvg_atr_mult?: number
  smc_bpr?: boolean
  smc_bpr_show_last?: number
  smc_bpr_atr_mult?: number
  smc_bpr_fenetre?: number
  smc_bpr_mitigation?: string
  smc_imbalance?: boolean
  smc_imb_show_last?: number
  smc_imb_show_fvg?: boolean
  smc_imb_show_og?: boolean
  smc_imb_mitigation?: string
  smc_fib?: boolean
  smc_tendance?: boolean
  smc_liquidites?: boolean
  smc_liq_swings?: boolean
  smc_liq_sessions?: boolean
  smc_liq_session_asie?: boolean

  smc_liq_dwm?: boolean
  smc_liq_dwm_nb?: number
  smc_liq_asie_range?: boolean
  smc_liq_asie_heure_debut?: number
  smc_liq_asie_heure_fin?: number
  smc_liq_asie_deviations_nb?: number
  smc_liq_asie_nb_sessions?: number
  /** Si true, calcule et retourne les signaux pour indicateurs actifs */
  signaux?: boolean
  limit?: number
}

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
