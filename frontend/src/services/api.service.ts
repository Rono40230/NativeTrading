import axios from 'axios'

export type {
  Candle, BacktestResults, PredictionML, ReponseEntrainement,
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, Signal, ScoreSmc, PointSerie,
  ZoneOb, ZoneFvg, ZoneIfvg, NiveauxFibonacci, ResultatTendance,
  NiveauLiquidite, ReponseIndicators, IndicatorsParams,
  LigneTendanceKasper, ReponseTendanceMultiTf, AssetInfo,
} from './api.types'

import type {
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, PredictionML, BacktestResults, ScoreSmc,
  ReponseEntrainement, ReponseIndicators, IndicatorsParams,
  ReponseTendanceMultiTf, AssetInfo, Signal, Candle,
} from './api.types'

const BASE_URL = 'http://localhost:8080'

const http = axios.create({
  baseURL: BASE_URL,
  timeout: 15000,
})

export const apiService = {
  async healthCheck(): Promise<{ status: string }> {
    const res = await http.get('/health')
    return res.data
  },

  async obtenirAssets(): Promise<AssetInfo[]> {
    const res = await http.get('/api/assets')
    return res.data
  },

  async getCandles(asset: string, timeframe = 'M15', limit = 200, force = false): Promise<Candle[]> {
    const res = await http.get('/api/candles', {
      params: { asset, timeframe, limit, ...(force ? { force: true } : {}) },
    })
    return res.data
  },

  async getSignaux(limit = 20): Promise<Signal[]> {
    const res = await http.get('/api/signaux', { params: { limit } })
    return res.data
  },

  async predictML(asset: string, timeframe = 'M15'): Promise<PredictionML> {
    const res = await http.get('/api/ml/predict', { params: { asset, timeframe } })
    return res.data
  },

  async runBacktest(
    asset: string,
    timeframe = 'M15',
    capital = 2000,
    limit = 500
  ): Promise<BacktestResults> {
    const res = await http.post('/api/backtest', { asset, timeframe, capital, limit })
    return res.data
  },

  async analyseSmc(asset: string, timeframe = 'M15', limit = 200): Promise<ScoreSmc> {
    const res = await http.get('/api/smc/analyse', { params: { asset, timeframe, limit } })
    return res.data
  },

  async entrainerML(asset = 'BTC', timeframe = 'M15', limit = 1000): Promise<ReponseEntrainement> {
    const res = await http.post('/api/ml/train', null, { params: { asset, timeframe, limit }, timeout: 180000 })
    return res.data
  },

  async statutML(): Promise<{ modele_pret: boolean; lstm_pret: boolean }> {
    const res = await http.get('/api/ml/status')
    return res.data
  },

  exportSignauxUrl(limit = 500): string {
    return `${BASE_URL}/api/signaux/export?limit=${limit}`
  },

  async statutIA(): Promise<StatutIA> {
    const res = await http.get('/api/ia/status')
    return res.data
  },

  async analyserIA(requete: RequeteAnalyseIA): Promise<ReponseAnalyseIA> {
    const res = await http.post('/api/ia/analyse', requete, { timeout: 120000 })
    return res.data
  },

  async chatIA(
    messages: { role: string; contenu: string }[]
  ): Promise<ReponseChatIA> {
    const res = await http.post('/api/ia/chat', { messages }, { timeout: 120000 })
    return res.data
  },

  async analyserChart(
    asset: string,
    images: ImageAvecTF[],
    notes?: string,
  ): Promise<ReponseChartIA> {
    const res = await http.post(
      '/api/ia/chart',
      { asset, images, ...(notes ? { notes } : {}) },
      { timeout: 180000 },
    )
    return res.data
  },

  async obtenirConfig(cle: string): Promise<{ cle: string; valeur: string } | null> {
    try {
      const res = await http.get('/api/config', { params: { cle } })
      return res.data
    } catch {
      return null
    }
  },

  async sauvegarderConfig(cle: string, valeur: string): Promise<boolean> {
    try {
      await http.post('/api/config', { cle, valeur })
      return true
    } catch {
      return false
    }
  },

  async ibStatus(): Promise<{ connecte: boolean; adresse: string; erreur?: string }> {
    try {
      const res = await http.get('/api/ib/status')
      return res.data
    } catch (err: any) {
      // 503 : IB Gateway inaccessible — retourner les données du body si présentes
      if (err?.response?.data) return err.response.data
      return { connecte: false, adresse: '', erreur: err?.message ?? 'Erreur réseau' }
    }
  },

  async getIndicators(params: IndicatorsParams): Promise<ReponseIndicators> {
    const res = await http.get('/api/indicators', { params })
    return res.data
  },

  async obtenirTendanceMultiTf(
    asset: string,
    mmRapide = 9,
    mmLente = 21,
    maType: 'ema' | 'sma' = 'ema'
  ): Promise<ReponseTendanceMultiTf> {
    const res = await http.get('/api/tendance/multi-tf', {
      params: { asset, mm_rapide: mmRapide, mm_lente: mmLente, ma_type: maType },
    })
    return res.data
  },
}
