/**
 * Service API pour le moteur de backtest.
 * POST /api/backtest/lancer — rejoue une stratégie sur l'historique
 */
import axios from 'axios'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 60_000 })

// ── Types ─────────────────────────────────────────────────────────────────────

export type StrategieType = 'straddle' | 'smc' | 'rockets'

export interface RequeteBacktest {
  asset: string
  timeframe: string
  strategie: StrategieType
  debut?: string        // ISO 8601 optionnel
  fin?: string          // ISO 8601 optionnel
  capital?: number      // défaut 10 000
  risque?: number       // fraction ex: 0.02
  nb_jours?: number     // défaut 90
}

export interface TradeBacktest {
  ouvert_a: string
  ferme_a: string | null
  direction: 'Long' | 'Short' | 'Both'
  prix_entree: number
  stop_loss: number
  take_profit_1: number
  take_profit_2: number | null
  take_profit_3: number | null
  resultat: 'Tp1' | 'Tp2' | 'Tp3' | 'StopLoss' | 'NonFerme'
  pnl_r: number
  pnl_usd: number
  heure_ouverture: number
  categorie: string
}

export interface StatHeure {
  heure: number
  nb_trades: number
  win_rate: number
  pnl_r_moyen: number
}

export interface StatJour {
  jour: number
  nom: string
  nb_trades: number
  win_rate: number
  pnl_r_moyen: number
}

export interface FenetrePropice {
  heure: number
  jour_semaine: number | null
  nb_trades: number
  win_rate: number
  pnl_r_total: number
  evenement_type: string | null
}

export interface BacktestResult {
  config: {
    asset: string
    timeframe: string
    strategie: string
    capital_initial: number
    risque_par_trade: number
  }
  nb_trades: number
  win_rate: number
  profit_factor: number
  sharpe: number
  drawdown_max: number
  capital_final: number
  pnl_total_r: number
  pnl_r_moyen: number
  perf_annualisee: number
  capital_min: number
  serie_pertes_max: number
  serie_gains_max: number
  double_sl_rate: number | null
  double_win_rate: number | null
  stats_par_heure: StatHeure[]
  stats_par_jour: StatJour[]
  equity_curve: number[]
  fenetres_propices: FenetrePropice[] | null
  trades: TradeBacktest[]
}

export interface Recommandation {
  titre: string
  explication: string
  impact_estime: string
  param_cible: string
  valeur_actuelle: string
  valeur_suggeree: string
  strategie: string
  priorite: number
}

export interface ReponseBacktest {
  result: BacktestResult
  recommandations: Recommandation[]
  duree_ms: number
}

// ── API ───────────────────────────────────────────────────────────────────────

export const backtestApi = {
  async lancer(req: RequeteBacktest): Promise<ReponseBacktest> {
    const res = await http.post<ReponseBacktest>('/api/backtest/lancer', req)
    return res.data
  },
}
