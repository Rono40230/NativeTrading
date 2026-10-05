/**
 * Méthodes API dédiées aux Rockets.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  RocketsMonitoringData, RocketsCalibrationRow,
} from './api.types'

export const rocketsApi = {
  // ── ML Rockets adaptatif ──────────────────────────────────────────────────

  async getRocketsMonitoringML(): Promise<RocketsMonitoringData> {
    const res = await http.get('/api/rockets/monitoring-ml', { timeout: 10000 })
    return res.data
  },

  async getRocketsCalibration(): Promise<RocketsCalibrationRow[]> {
    const res = await http.get('/api/rockets/calibration', { timeout: 10000 })
    return res.data
  },

}
