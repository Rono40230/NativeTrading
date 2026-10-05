/**
 * Méthodes API dédiées au Straddle.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  StraddleMonitoringData, StraddleCalibrationRow,
} from './api.types'

export const straddleApi = {
  // ── ML Straddle adaptatif ──────────────────────────────────────────────────

  async getStraddleMonitoringML(): Promise<StraddleMonitoringData> {
    const res = await http.get('/api/straddle/monitoring-ml', { timeout: 10000 })
    return res.data
  },

  async getStraddleCalibration(): Promise<StraddleCalibrationRow[]> {
    const res = await http.get('/api/straddle/calibration', { timeout: 10000 })
    return res.data
  },

}
