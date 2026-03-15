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
}

export interface ZoneFvg {
  prix_haut: number
  prix_bas: number
  direction: string
  comble: boolean
}

export interface ZoneIfvg {
  prix_haut: number
  prix_bas: number
  direction: string
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
  equal: boolean
  sweepé: boolean
}

export interface ReponseIndicators {
  ema?: PointSerie[]
  rsi?: PointSerie[]
  atr?: PointSerie[]
  macd?: { macd: PointSerie[]; signal: PointSerie[]; histogramme: PointSerie[] }
  bollinger?: { haute: PointSerie[]; milieu: PointSerie[]; basse: PointSerie[] }
  order_blocks?: ZoneOb[]
  imbalances?: ZoneFvg[]
  ifvg?: ZoneIfvg[]
  fibonacci?: NiveauxFibonacci
  tendance?: ResultatTendance
  liquidites?: NiveauLiquidite[]
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
  rsi_periode?: number
  smc_ob?: boolean
  smc_fvg?: boolean
  smc_ifvg?: boolean
  smc_fib?: boolean
  smc_tendance?: boolean
  smc_liquidites?: boolean
  limit?: number
}

export interface LigneTendanceKasper {
  tf: string
  tendance: 'haussier' | 'baissier' | null
  mm_rapide: number | null
  mm_lente: number | null
}

export interface ReponseTendanceMultiTf {
  asset: string
  mm_rapide_periode: number
  mm_lente_periode: number
  ma_type: string
  lignes: LigneTendanceKasper[]
}

export interface AssetInfo {
  id: string
  nom: string
  type: 'crypto' | 'metal' | 'forex' | 'indice'
}
