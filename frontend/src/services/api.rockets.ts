/**
 * Méthodes API dédiées aux Rockets.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  RocketSignalSave, RocketSignalHistorique, RocketAnalyseLlm, RocketsConfig,
  RocketsMonitoringData, RocketsCalibrationRow, RocketsSeuilsEffectifs,
} from './api.types'

export const rocketsApi = {
  async sauvegarderRocket(signal: RocketSignalSave): Promise<void> {
    await http.post('/api/rockets/signal', signal)
  },

  async getRocketsScan(): Promise<unknown> {
    const res = await http.get('/api/rockets/scan')
    return res.data
  },

  async historiqueRockets(limite = 50): Promise<RocketSignalHistorique[]> {
    const res = await http.get('/api/rockets/historique', { params: { limite } })
    return res.data
  },

  async rocketsActifs(): Promise<RocketSignalHistorique[]> {
    const res = await http.get('/api/rockets/actifs')
    return res.data
  },

  async syncRockets(): Promise<{ fermes: number; ouverts_nouveaux: number }> {
    const res = await http.post('/api/rockets/sync', null, { timeout: 60000 })
    return res.data
  },

  async annulerRocket(id: number): Promise<void> {
    await http.delete(`/api/rockets/signal/${id}`)
  },

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

  async getRocketsEquity(capital = 10000, risk_pct = 0.015): Promise<{
    capital_initial: number
    risk_pct: number
    nb_trades_saisis: number
    points: { ticker: string; verdict: string; pnl_r: number; equity_cumulee: number; ferme_le: number }[]
  }> {
    const res = await http.get('/api/rockets/equity', { params: { capital, risk_pct } })
    return res.data
  },

  async analyserOpportunites(signaux: {
    ticker: string; phase: string; change1h: number; ratio_volume: number;
    atr_ratio: number; rsi: number; score: number; entree_limite: number;
    entree_stop: number; niveau_invalidation: number; type_entree_rec: string;
    sl: number; tp1: number; tp2: number; tp3_trigger: number; trailing_coeff: number;
  }[]): Promise<{ texte: string }> {
    const res = await http.post('/api/rockets/analyse-opportunites', signaux, { timeout: 120000 })
    return res.data
  },

  async getRocketsSeuilsEffectifs(phase = 'breakout', session = 'London'): Promise<RocketsSeuilsEffectifs> {
    const res = await http.get('/api/rockets/seuils-effectifs', { params: { phase, session } })
    return res.data
  },

  async postFeedbackTrader(body: {
    signal_id: number
    verdict: 'tp1' | 'tp2' | 'tp3' | 'sl' | 'ignore'
    prix_entree_reel?: number
    prix_sortie_reel?: number
    notes?: string
  }): Promise<void> {
    await http.post('/api/rockets/feedback/trader', body)
  },
}
