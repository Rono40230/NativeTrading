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
  /** Unix secondes — timestamp du pivot haut (ancrage gauche) */
  timestamp_haut: number
  /** Unix secondes — timestamp du pivot bas (ancrage gauche) */
  timestamp_bas: number
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

export interface ResultatBos {
  direction: 'Long' | 'Short'
  niveau_casse: number
  prix_cassure: number
}

export interface ResultatChoch {
  direction: 'Long' | 'Short'
  niveau_casse: number
  prix_cassure: number
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
  bos?: ResultatBos
  choch?: ResultatChoch
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
  smc_bos?: boolean
  smc_choch?: boolean
  limit?: number
}
