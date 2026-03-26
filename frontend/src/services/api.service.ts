import axios from 'axios'

export type {
  Candle, BacktestResults, PredictionML, ReponseEntrainement,
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, Signal, ScoreSmc, PointSerie,
  ZoneOb, ZoneIfvg, NiveauxFibonacci, ResultatTendance,
  NiveauLiquidite, ReponseIndicators, IndicatorsParams, SignalIndicateur,
  LigneTendanceKasper, ReponseTendanceMultiTf, AssetInfo, AnnonceCalendrier,
  SentimentMarche, EntiteSentiment, ArticleNews, AlertesNews, NiveauAlerte, ContenuArticle, TraductionReponse,
  StatutSignalEngine, CouvertureDonnees, RequeteCollecte, ResultatCollecte, ResultatCollecteItem,
  HistoriqueEntrainement, HistoriqueML, PatternHoraire, ReponsePatternsVolatilite,
  StraddleCreneau, ReponseAnalyseStraddle,
} from './api.types'

import type {
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, PredictionML, BacktestResults, ScoreSmc,
  ReponseEntrainement, ReponseIndicators, IndicatorsParams,
  ReponseTendanceMultiTf, AssetInfo, Signal, Candle,
  ModeCalculTendance, AnnonceCalendrier, SentimentMarche, AlertesNews, ContenuArticle, TraductionReponse,
  StatutSignalEngine, CouvertureDonnees, RequeteCollecte, ResultatCollecte, HistoriqueML, ReponsePatternsVolatilite,
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
      timeout: 60000, // pagination Binance : jusqu'à 5 requêtes × ~3s chacune
    })
    return res.data
  },

  async getPrixActuel(ticker: string): Promise<number | null> {
    try {
      const res = await http.get('/api/prix-actuel', { params: { ticker }, timeout: 5000 })
      return res.data?.prix ?? null
    } catch {
      return null
    }
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
    emaRapide = 9,
    emaLente = 21,
    modeCalcul: ModeCalculTendance = 'bougie_cloturee'
  ): Promise<ReponseTendanceMultiTf> {
    const res = await http.get('/api/tendance/multi-tf', {
      params: { asset, ema_rapide: emaRapide, ema_lente: emaLente, mode_calcul: modeCalcul },
    })
    return res.data
  },

  async obtenirCalendrier(days = 7): Promise<AnnonceCalendrier[]> {
    try {
      const res = await http.get('/api/calendar', { params: { days } })
      return res.data
    } catch {
      return []
    }
  },

  async obtenirSentimentMarche(): Promise<SentimentMarche> {
    const res = await http.get('/api/sentiment/marche')
    return res.data
  },

  async obtenirAlertes(): Promise<AlertesNews> {
    const res = await http.get('/api/news/alertes', { timeout: 20_000 })
    return res.data
  },

  async obtenirContenuArticle(url: string): Promise<ContenuArticle> {
    const res = await http.get('/api/news/contenu', { params: { url }, timeout: 20_000 })
    return res.data
  },

  async traduire(texte: string, long = false): Promise<TraductionReponse> {
    const res = await http.get('/api/news/traduire', { params: { texte, long }, timeout: 60_000 })
    return res.data
  },

  async signalEngineStatut(): Promise<StatutSignalEngine> {
    const res = await http.get('/api/signal-engine/status')
    return res.data
  },

  async signalEngineDemarrer(): Promise<{ statut: string; message: string }> {
    const res = await http.post('/api/signal-engine/start')
    return res.data
  },

  async signalEngineArreter(): Promise<{ statut: string; message: string }> {
    const res = await http.post('/api/signal-engine/stop')
    return res.data
  },

  async obtenirCouvertureDonnees(): Promise<{ couverture: CouvertureDonnees[] }> {
    const res = await http.get('/api/data/coverage')
    return res.data
  },

  async collecterDonnees(params: RequeteCollecte): Promise<ResultatCollecte> {
    const res = await http.post('/api/data/collect', params, { timeout: 300_000 })
    return res.data
  },

  async obtenirHistoriqueML(limit = 30): Promise<HistoriqueML> {
    const res = await http.get('/api/ml/history', { params: { limit } })
    return res.data
  },

  async obtenirPatternsVolatilite(
    asset = 'BTC',
    timeframe = 'M15',
    mois = 12,
  ): Promise<ReponsePatternsVolatilite> {
    const res = await http.get('/api/volatility/patterns', { params: { asset, timeframe, mois } })
    return res.data
  },

  async ajouterAsset(
    id: string,
    nom: string,
    type: AssetInfo['type'],
    source: 'binance' | 'ib',
  ): Promise<void> {
    try {
      await http.post('/api/assets', { id, nom, type, source })
    } catch (err: any) {
      const message = err?.response?.data?.error
      if (err?.response?.status === 409) {
        throw new Error(message ?? 'Cet asset est déjà dans la liste.')
      }
      throw new Error(message ?? err?.message ?? "Erreur lors de l'ajout.")
    }
  },

  async supprimerAsset(id: string): Promise<void> {
    await http.delete(`/api/assets/${encodeURIComponent(id)}`)
  },

  async sauvegarderRocket(signal: import('./api.types').RocketSignalSave): Promise<void> {
    await http.post('/api/rockets/signal', signal)
  },

  async getRocketsScan(): Promise<unknown> {
    const res = await http.get('/api/rockets/scan')
    return res.data
  },

  async historiqueRockets(limite = 50): Promise<import('./api.types').RocketSignalHistorique[]> {
    const res = await http.get('/api/rockets/historique', { params: { limite } })
    return res.data
  },

  async syncRockets(): Promise<{ fermes: number; ouverts_nouveaux: number }> {
    const res = await http.post('/api/rockets/sync', null, { timeout: 60000 })
    return res.data
  },

  async lancerAnalyseLlmRockets(): Promise<import('./api.types').RocketAnalyseLlm> {
    const res = await http.post('/api/rockets/analyse-llm', null, { timeout: 120000 })
    return res.data
  },

  async getDerniereAnalyseLlmRockets(): Promise<import('./api.types').RocketAnalyseLlm | null> {
    try {
      const res = await http.get('/api/rockets/analyse-llm')
      return res.status === 204 ? null : res.data
    } catch {
      return null
    }
  },

  async getRocketsConfig(): Promise<import('./api.types').RocketsConfig> {
    const res = await http.get('/api/rockets/config')
    return res.data
  },

  async putRocketsConfig(cfg: import('./api.types').RocketsConfig): Promise<void> {
    await http.put('/api/rockets/config', cfg)
  },

  // ── Straddle ──────────────────────────────────────────────────────────────
  async runStraddleSlotBacktest(
    asset: string,
    heure_debut: string,
    jour_semaine: number | null,
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
    const res = await http.post('/api/straddle/backtest', { asset, heure_debut, jour_semaine, capital })
    return res.data
  },
  async analyserStraddle(
    asset: string,
    periode: string,
  ): Promise<import('./api.types').ReponseAnalyseStraddle> {
    const res = await http.post('/api/straddle/analyser', { asset, periode }, { timeout: 150000 })
    return res.data
  },

  async getStraddleCreneaux(): Promise<import('./api.types').StraddleCreneau[]> {
    const res = await http.get('/api/straddle/creneaux')
    return res.data
  },

  async patchStraddleCreneau(
    id: number,
    data: { statut?: string; backtest_winrate?: number; backtest_profit_factor?: number },
  ): Promise<void> {
    await http.patch(`/api/straddle/creneaux/${id}`, data)
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
