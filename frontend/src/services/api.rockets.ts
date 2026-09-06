/**
 * Méthodes API dédiées aux Rockets.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  RocketAnalyseLlm, RocketsConfig,
  RocketsMonitoringData, RocketsCalibrationRow,
} from './api.types'

export const rocketsApi = {
  async lancerAnalyseLlmRockets(): Promise<RocketAnalyseLlm> {
    const res = await http.post('/api/rockets/analyse-llm', null, { timeout: 120000 })
    return res.data
  },

  async getDerniereAnalyseLlmRockets(): Promise<RocketAnalyseLlm | null> {
    try {
      const res = await http.get('/api/rockets/analyse-llm')
      return res.status === 204 ? null : res.data
    } catch {
      return null
    }
  },

  async getRocketsConfig(): Promise<RocketsConfig> {
    const res = await http.get('/api/rockets/config')
    return res.data
  },

  async putRocketsConfig(cfg: RocketsConfig): Promise<void> {
    await http.put('/api/rockets/config', cfg)
  },

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
