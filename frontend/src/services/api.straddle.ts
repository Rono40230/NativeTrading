/**
 * Méthodes API dédiées au Straddle.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import axios from 'axios'
import type {
  ReponseAnalyseStraddle, StraddleCreneau,
  StraddleVolatiliteLive, StraddleMonitoringData, StraddleCalibrationRow, PrecisionHoraire,
} from './api.types'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 15000 })

export const straddleApi = {
  async analyserStraddle(asset: string, periode: string): Promise<ReponseAnalyseStraddle> {
    const res = await http.post('/api/straddle/analyser', { asset, periode }, { timeout: 150000 })
    return res.data
  },

  async getStraddleCreneaux(): Promise<StraddleCreneau[]> {
    const res = await http.get('/api/straddle/creneaux')
    return res.data
  },

  async patchStraddleCreneau(
    id: number,
    data: { statut?: string; backtest_winrate?: number; backtest_profit_factor?: number },
  ): Promise<void> {
    await http.patch(`/api/straddle/creneaux/${id}`, data)
  },

  async demanderAjustements(params: {
    asset: string
    roi_pct: number
    win_rate: number
    max_drawdown_pct: number
    profit_factor: number
    sharpe_ratio: number
    tp_mult_1?: number
    tp_mult_2?: number
    tp_mult_3?: number
    sl_mult?: number
    seuil_atr?: number
  }): Promise<{ tp_mult_1: number; tp_mult_2: number; tp_mult_3: number; sl_mult: number; seuil_atr: number; raison: string; modele: string }> {
    const res = await http.post('/api/ia/ajustements', params, { timeout: 120000 })
    return res.data
  },

  async analyserPrecisionCreneau(
    id: number,
    creneau: { asset: string; jour_semaine: number | null; heure_debut: string; heure_fin: string },
  ): Promise<{
    timing_optimal?: string
    fenetre_entree?: string
    whipsaw_minutes?: number
    nb_occurrences?: number
    atr_pic?: number
    ok?: boolean
    message?: string
  }> {
    const res = await http.post(`/api/straddle/creneaux/${id}/precision`, creneau, { timeout: 30000 })
    return res.data
  },

  async getAbTest(): Promise<{ strategie: string; nb_total: number; nb_wins: number; nb_pertes: number; win_rate: number; conviction_moy: number; score_moy: number }[]> {
    const res = await http.get('/api/ia/ab-test')
    return res.data
  },

  // ── ML Straddle adaptatif ──────────────────────────────────────────────────

  async getStraddleVolatiliteLive(): Promise<StraddleVolatiliteLive> {
    const res = await http.get('/api/straddle/volatilite-live', { timeout: 10000 })
    return res.data
  },

  async getStraddleMonitoringML(): Promise<StraddleMonitoringData> {
    const res = await http.get('/api/straddle/monitoring-ml', { timeout: 10000 })
    return res.data
  },

  async getStraddleCalibration(): Promise<StraddleCalibrationRow[]> {
    const res = await http.get('/api/straddle/calibration', { timeout: 10000 })
    return res.data
  },

  async analyserPrecisionHoraire(
    asset: string,
    heure: number,
    jourSemaine: number | null,
  ): Promise<PrecisionHoraire> {
    const res = await http.post('/api/straddle/precision-horaire', {
      asset,
      heure,
      jour_semaine: jourSemaine,
    }, { timeout: 30000 })
    return res.data
  },

  async getStraddleEquity(capital = 10000, risk_pct = 0.015): Promise<{
    capital_initial: number; risk_pct: number; nb_trades_saisis: number
    points: { asset: string; verdict: string; pnl_r: number; equity_cumulee: number; ferme_le: number }[]
  }> {
    const res = await http.get('/api/straddle/equity', { params: { capital, risk_pct } })
    return res.data
  },
}
