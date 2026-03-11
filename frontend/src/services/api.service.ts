import axios from 'axios'

const BASE_URL = 'http://localhost:8080'

const http = axios.create({
  baseURL: BASE_URL,
  timeout: 15000,
})

export interface Candle {
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface BacktestResults {
  total_trades: number
  winning_trades: number
  losing_trades: number
  win_rate: number
  capital_initial: number
  capital_final: number
  roi_pct: number
  profit_net: number
  sharpe_ratio: number
  max_drawdown_pct: number
  profit_factor: number
}

export interface PredictionML {
  asset: string
  direction: string
  confiance: number
  est_confiant: boolean
  modele_pret: boolean
}

export interface ReponseEntrainement {
  success: boolean
  accuracy_rf: number
  accuracy_lstm: number
  nb_echantillons: number
  duree_ms: number
  message: string
}

export interface RequeteAnalyseIA {
  asset: string
  timeframe: string
  direction: string
  score_smc: number
  prix_entree: number
  stop_loss: number
  take_profit: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  confiance_ml: number
}

export interface ReponseAnalyseIA {
  analyse: string
  modele: string
}

export interface ReponseChatIA {
  reponse: string
  modele: string
}

export interface ReponseChartIA {
  analyse: string
  modele: string
}

export interface StatutIA {
  ollama_disponible: boolean
  modele: string
  url: string
}

export interface Signal {
  id: string
  asset: string
  timeframe: string
  direction: string
  score: number
  prix_entree: number
  stop_loss: number
  take_profit: string
  strategie: string
  cree_le: number
}

export interface ScoreSmc {
  total: number
  tendance: number
  order_block: number
  imbalance: number
  ifvg: number
  fibonacci: number
  direction: string
  confluence: boolean
}

export const apiService = {
  async healthCheck(): Promise<{ status: string }> {
    const res = await http.get('/health')
    return res.data
  },

  async getCandles(asset: string, timeframe = 'M15', limit = 200): Promise<Candle[]> {
    const res = await http.get('/api/candles', {
      params: { asset, timeframe, limit },
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
    timeframe: string,
    image_base64: string
  ): Promise<ReponseChartIA> {
    const res = await http.post('/api/ia/chart', { asset, timeframe, image_base64 }, { timeout: 180000 })
    return res.data
  },
}
