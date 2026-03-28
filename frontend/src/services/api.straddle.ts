/**
 * Méthodes API dédiées au Straddle.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import axios from 'axios'
import type { ReponseAnalyseStraddle, StraddleCreneau } from './api.types'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 15000 })

export const straddleApi = {
  async runStraddleSlotBacktest(
    asset: string,
    heure_debut: string,
    jour_semaine: number | null,
    heure_fin?: string,
    capital?: number,
  ): Promise<{
    total_trades: number
    win_rate: number
    profit_factor: number
    max_drawdown_pct: number
    esperance_pct: number
    payoff_ratio: number
    serie_pertes_max: number
    direction_dominante: string
    amplitude_moyenne: number
  }> {
    const res = await http.post('/api/straddle/backtest', { asset, heure_debut, jour_semaine, heure_fin, capital })
    return res.data
  },

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
}
