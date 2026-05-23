/**
 * Service API pour le moteur de backtest.
 * POST /api/backtest/lancer — rejoue une stratégie sur l'historique
 */
import axios from 'axios'
import type { BacktestResult } from '../generated/BacktestResult'
import type { Recommandation } from '../generated/Recommandation'

export type { BacktestResult } from '../generated/BacktestResult'
export type { TradeBacktest } from '../generated/TradeBacktest'
export type { StatHeure } from '../generated/StatHeure'
export type { StatJour } from '../generated/StatJour'
export type { FenetrePropice } from '../generated/FenetrePropice'
export type { ResultatTrade } from '../generated/ResultatTrade'
export type { Recommandation } from '../generated/Recommandation'
export type { StrategieType } from '../generated/StrategieType'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 60_000 })

// ── Types de requête (non générés — propres au frontend) ─────────────────────

/** Stratégie envoyée en minuscules dans la requête HTTP */
export type StrategieTypeRequest = 'straddle' | 'smc' | 'rockets'

export interface RequeteBacktest {
  asset: string
  timeframe: string
  strategie: StrategieTypeRequest
  debut?: string        // ISO 8601 optionnel
  fin?: string          // ISO 8601 optionnel
  capital?: number      // défaut 10 000
  risque?: number       // fraction ex: 0.02
  nb_jours?: number     // défaut 90
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
