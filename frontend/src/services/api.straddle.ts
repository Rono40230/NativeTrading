/**
 * Méthodes API dédiées au Straddle.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  ReponseAnalyseStraddle,
  StraddleMonitoringData, StraddleCalibrationRow, PrecisionHoraire,
} from './api.types'

export const straddleApi = {
  async analyserStraddle(asset: string, periode: string): Promise<ReponseAnalyseStraddle & { message?: string }> {
    const res = await http.post('/api/straddle/analyser', { asset, periode }, { timeout: 150000 })
    return res.data
  },

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
