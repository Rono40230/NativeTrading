/**
 * Méthodes API dédiées au monitoring ML SMC Directionnel.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type { SmcMonitoringData, SmcCalibrationRow, SmcBaremes } from './api.types'

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
}
