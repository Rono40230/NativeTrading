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
/** Niveau cassé (BOS / MSS / CHOCH / sweep). */
export interface NiveauCasseeV12 {
  ts: number
  dir: 'bull' | 'bear'
  level: number
  bar_idx: number
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
export interface SmcV12Analyse {
  asset: string
  timeframe: string
  nb_bougies: number
  pivots: PivotV12[]
  bos: NiveauCasseeV12[]
  mss: NiveauCasseeV12[]
  chochs: NiveauCasseeV12[]
  sweeps: NiveauCasseeV12[]
  obs: ObV12[]
  fvgs: FvgV12[]
  signals: SignalV12[]
  tendance: 'haussiere' | 'baissiere' | 'neutre'
  atr14: number
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
