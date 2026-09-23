/**
 * Méthodes API dédiées au Straddle.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  StraddleMonitoringData, StraddleCalibrationRow, PrecisionHoraire,
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
}
