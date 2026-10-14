/**
 * Service API pour le module ML Insights (Phase 8).
 */
import { http } from './http.client'

export interface StatsGlobales {
  nb_trades:   number
  nb_gagnants: number
  win_rate:    number
  pnl_r_moyen: number
}

// Écart 25 (13/10) : les tranches par score/kill-zone/confiance/phase ont été
// purgées — elles lisaient les tables feedback ML mortes (toujours vides).

export interface SmcAnalyse {
  global: StatsGlobales
}

export interface RocketsAnalyse {
  global: StatsGlobales
}

export interface StraddleAnalyse {
  global: StatsGlobales
}

export interface AnalyseGlobale {
  smc?:      SmcAnalyse
  rockets?:  RocketsAnalyse
  straddle?: StraddleAnalyse
}

export interface RetainJobState {
  job_id:        string | null
  en_cours:      boolean
  accuracy_avant: number
  accuracy_apres: number | null
  wf_score_apres: number | null
  gap_train_wf:  number | null
  overfitting:   boolean
  rolled_back:   boolean
  message:       string
  demarre_le:    number | null
  termine_le:    number | null
  nb_combinaisons_total: number
  nb_combinaisons_done:  number
  combinaison_en_cours:  string
}

export const mlInsightsApi = {
  async getStats(): Promise<AnalyseGlobale> {
    const res = await http.get('/api/ml/feedback/stats')
    return res.data
  },

  async getRetrainStatus(jobId: string): Promise<RetainJobState> {
    const res = await http.get(`/api/ml/retrain/status/${jobId}`)
    return res.data
  },

  async getRetrainLast(): Promise<RetainJobState> {
    const res = await http.get('/api/ml/retrain/last')
    return res.data
  },
}
