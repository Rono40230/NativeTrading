/**
 * Méthodes API dédiées au monitoring ML SMC Directionnel.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type { SmcMonitoringData, SmcCalibrationRow, SmcBaremes } from './api.types'

// ── Types SMC v12 (replay bar-par-bar du moteur Rust) ─────────────────────────
/** Pivot de structure : HH / HL / LH / LL positionné sur le swing réel. */
export interface PivotV12 {
  ts: number
  type: 'HH' | 'HL' | 'LH' | 'LL'
  price: number
  bar_idx: number
}
/** Niveau cassé (BOS / MSS / CHOCH / sweep).
 *  `pivot_ts` = timestamp du pivot cassé (borne de début de la ligne pivot→cassure).
 *  Pour les sweeps (événement ponctuel), `pivot_ts == ts`. */
export interface NiveauCasseeV12 {
  ts: number
  pivot_ts: number
  dir: 'bull' | 'bear'
  level: number
  bar_idx: number
  /** Bougie du sweep (ancrage étiquette) — sweeps uniquement. */
  candle_high?: number
  candle_low?: number
}
/** Order Block actif (non invalidé). */
export interface ObV12 {
  ts: number
  dir: 'bull' | 'bear'
  top: number
  bot: number
  state: 'vierge' | 'partiel' | 'profond'
  force: number
  bar_idx: number
}
/** Fair Value Gap actif. */
export interface FvgV12 {
  ts: number
  dir: 'bull' | 'bear'
  top: number
  bot: number
  state: 'vierge' | 'partiel'
  bar_idx: number
}
/** Trade généré par le moteur (v11 OB ou BSZones) avec verdict lifecycle. */
export interface SignalV12 {
  ts: number
  dir: 'Long' | 'Short'
  entry: number
  sl: number
  tp1: number
  tp2: number
  tp3: number
  force: number
  source: 'v11' | 'bszones'
  verdict: 'TP3' | 'TP2' | 'TP1' | 'SL' | 'BE' | 'Expire'
}

// ── Indicateurs v12 étendus (sérialisés à plat via #[serde(flatten)]) ─────────
// Les noms de champs correspondent EXACTEMENT au JSON backend (snake_case,
// structs de `smc_v12_out.rs`). Voir .superpowers/sdd/fix-api-complete-report.md.

/** Niveau de liquidité précédent : PDH / PDL / PWH / PWL. */
export interface LiquiditeLevelV12 {
  level: 'pdh' | 'pdl' | 'pwh' | 'pwl'
  price: number | null
  active: boolean
  /** Timestamp où le niveau s'est formé (bord gauche ligne). */
  ts_origine?: number
}
/** Niveau EQH/EQL (dir "high" = EQH, "low" = EQL). */
/** Plage premium/discount (bgcolor par barre, Pine MODULE 4b). */
export interface PremRange { start_ts: number; end_ts: number; dir: 'prem' | 'disc' }

/** Plage de tendance (bgcolor par barre, Pine MODULE 1). */
export interface TrendRange { start_ts: number; end_ts: number; dir: 'bull' | 'bear' }

/** Box de session complète (Pine MODULE 14, heures Paris, 24h). */
export interface SessionBox { start_ts: number; end_ts: number; session: string; high: number; low: number }

export interface EqV12 {
  dir: 'high' | 'low'
  price: number
  touches: number
  swept: boolean
  bar_idx: number
  /** Timestamp du 1er pivot (bord gauche de la ligne, comme le Pine). */
  ts: number
}
/** Propulsion Block actif (MODULE 8c). */
export interface PropulsionV12 { ts: number; dir: 'bull' | 'bear'; top: number; bot: number }

/** Breaker block actif. */
export interface BreakerV12 {
  ts: number
  dir: 'bull' | 'bear'
  top: number
  bot: number
  bar_idx: number
}
/** Imbalance active (state = vierge | partiel | profond). */
export interface ImbalanceV12 {
  ts: number
  dir: 'bull' | 'bear'
  top: number
  bot: number
  state: string
  bar_idx: number
}
/** Zone OTE active (≤1 par sens, sans ts — étendue sur toute la largeur). */
export interface OteV12 {
  dir: 'bull' | 'bear'
  top: number
  bot: number
  /** Timestamp de la bar du BOS — bord gauche de la box (Pine _oteBullBox). */
  ts: number
}
/** Zone-cœur (intersection OB ∩ OTE ∩ FVG). */
export interface ZoneCoeurV12 {
  /** Timestamp de création de la box live. */
  ts: number
  dir: 'bull' | 'bear'
  top: number
  bot: number
  ob_bar: number
  /** Bougie d'origine de l'OB parent — bord gauche de la box (Pine obBullBar). */
  ob_ts: number
}
/** État final Premium/Discount (équilibrium ICT + dealing range). */
export interface PremiumDiscountV12 {
  pd_range_h: number | null
  pd_range_l: number | null
  equilibrium: number | null
  in_premium: boolean
  in_discount: boolean
}
/** Order Block HTF (MTF H1/H4/W1/MN). */
export interface HtfObV12 {
  timeframe: 'H1' | 'H4' | 'W1' | 'MN'
  dir: 'bull' | 'bear'
  top: number
  bot: number
  ts: number
}
/** Plage de session Kill Zone (compression run-length). */
export interface SessionRangeV12 {
  start_ts: number
  end_ts: number
  session: 'asie' | 'londres' | 'ny'
}
/** Niveaux Asian High/Low (range de la session Asie du jour le plus récent). */
export interface AsianHlV12 {
  high: number
  low: number
  invalidated_up: boolean
  invalidated_down: boolean
  /** 1re bougie de la session Asie (bord gauche des lignes). */
  start_ts: number
}
/** Gap NDOG/NWOG actif. */
export interface GapV12 {
  ts: number
  gtype: 'ndog' | 'nwog'
  top: number
  bot: number
  mitigated: boolean
  bar_idx: number
}
/** Plage contiguë de volume fort (compression run-length). */
export interface VolRangeV12 {
  start_ts: number
  end_ts: number
}
/** Plage contiguë d'impulsion (compression run-length). */
export interface ImpRangeV12 {
  start_ts: number
  end_ts: number
  impulsion: 'bull' | 'bear'
}

export interface SmcV12Analyse {
  asset: string
  timeframe: string
  nb_bougies: number
  pivots: PivotV12[]
  bos: NiveauCasseeV12[]
  mss: NiveauCasseeV12[]
  chochs: NiveauCasseeV12[]
  sweeps: NiveauCasseeV12[]
  trend_ranges?: TrendRange[]
  prem_ranges?: PremRange[]
  session_boxes?: SessionBox[]
  obs: ObV12[]
  fvgs: FvgV12[]
  signals: SignalV12[]
  tendance: 'haussiere' | 'baissiere' | 'neutre'
  atr14: number
  // ── Indicateurs étendus (optionnels : absents si backend non mis à jour) ──
  liquidites?: LiquiditeLevelV12[]
  eqs?: EqV12[]
  breakers?: BreakerV12[]
  propulsions?: PropulsionV12[]
  imbalances?: ImbalanceV12[]
  otes?: OteV12[]
  zone_coeur?: ZoneCoeurV12[]
  premium_discount?: PremiumDiscountV12
  mtf_obs?: HtfObV12[]
  sessions?: SessionRangeV12[]
  asian_hl?: AsianHlV12 | null
  gaps?: GapV12[]
  vol_fort?: VolRangeV12[]
  impulsions?: ImpRangeV12[]
}

export const apiSmcMethods = {
  async getSmcMonitoringML(): Promise<SmcMonitoringData> {
    const res = await http.get('/api/smc/monitoring-ml', { timeout: 10000 })
    return res.data
  },

  async getSmcCalibration(): Promise<SmcCalibrationRow[]> {
    const res = await http.get('/api/smc/calibration', { timeout: 10000 })
    return res.data
  },

  async getSmcEquity(capital = 10000, risk_pct = 0.015): Promise<{
    capital_initial: number; risk_pct: number; nb_trades_saisis: number
    points: { asset: string; verdict: string; pnl_r: number; equity_cumulee: number; ferme_le: number; duree_min: number }[]
  }> {
    const res = await http.get('/api/smc/equity', { params: { capital, risk_pct } })
    return res.data
  },

  async getSmcBaremes(): Promise<SmcBaremes> {
    const res = await http.get('/api/smc/baremes')
    return res.data
  },

  /** Replay complet du moteur SMC v12 (pivots/BOS/MSS/OB/FVG/signals). */
  async getSmcV12Analyse(asset: string, timeframe: string, limit = 500): Promise<SmcV12Analyse> {
    const res = await http.get('/api/smc/v12/analyse', {
      params: { asset, timeframe, limit },
      timeout: 30000,
    })
    return res.data
  },
}
