/**
 * Méthodes API dédiées au monitoring ML SMC Directionnel.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import axios from 'axios'
import type { SmcMonitoringData, SmcCalibrationRow } from './api.types'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 15000 })

export const apiSmcMethods = {
  async getSmcMonitoringML(): Promise<SmcMonitoringData> {
    const res = await http.get('/api/smc/monitoring-ml', { timeout: 10000 })
    return res.data
  },

  async getSmcCalibration(): Promise<SmcCalibrationRow[]> {
    const res = await http.get('/api/smc/calibration', { timeout: 10000 })
    return res.data
  },
}
