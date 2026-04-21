/**
 * Service API pour le module ML Insights (Phase 8).
 */
import axios from 'axios'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 15_000 })

export interface StatsGlobales {
  nb_trades:   number
  nb_gagnants: number
  win_rate:    number
  pnl_r_moyen: number
}

export interface TrancheStat {
  tranche:   string
  nb_trades: number
  win_rate:  number
}

export interface SmcAnalyse {
  global:         StatsGlobales
  par_score:      TrancheStat[]
  par_kill_zone:  TrancheStat[]
  ml_correlation: TrancheStat[]
}

export interface RocketsAnalyse {
  global:         StatsGlobales
  par_phase:      TrancheStat[]
  conviction_llm: TrancheStat[]
}

export interface StraddleAnalyse {
  global:         StatsGlobales
  par_categorie:  TrancheStat[]
  score_llm:      TrancheStat[]
}

export interface AnalyseGlobale {
  smc?:      SmcAnalyse
  rockets?:  RocketsAnalyse
  straddle?: StraddleAnalyse
}

export interface SuggestionParams {
  strategie:           string
  param_name:          string
  valeur_actuelle:     number
  valeur_suggeree:     number
  gain_winrate_estime: number
  confiance:           number
  justification:       string
  nb_samples_base:     number
}

export interface SuggestionLogEntry {
  id:                  number
  strategie:           string
  param_name:          string
  valeur_avant:        number
  valeur_apres:        number
  gain_winrate_estime: number
  confiance:           number
  nb_samples_base:     number
  appliquee_le:        string
}

export interface SuggestionsResponse {
  suggestions: SuggestionParams[]
  historique:  SuggestionLogEntry[]
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

  async getSuggestions(): Promise<SuggestionsResponse> {
    const res = await http.get('/api/ml/suggestions')
    return res.data
  },

  async appliquerSuggestion(s: SuggestionParams): Promise<void> {
    await http.post('/api/ml/suggestions/appliquer', {
      strategie:           s.strategie,
      param_name:          s.param_name,
      valeur_actuelle:     s.valeur_actuelle,
      valeur_suggeree:     s.valeur_suggeree,
      gain_winrate_estime: s.gain_winrate_estime,
      confiance:           s.confiance,
      nb_samples_base:     s.nb_samples_base,
    })
  },

  async postRetrain(): Promise<{ job_id: string; status: string }> {
    const res = await http.post('/api/ml/retrain')
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
