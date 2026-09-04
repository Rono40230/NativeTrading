import { http } from './http.client'
import { rocketsApi } from './api.rockets'
import { straddleApi } from './api.straddle'
import { apiSmcMethods } from './api.smc'
import { newsApi } from './api.news'
import { engineApi } from './api.engine'
import { workerApi } from './api.worker'
import { presseApi } from './api.presse'
import { alertesApi } from './api.alertes'

export type {
  Candle, PredictionML, ReponseEntrainement,
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, Signal, ScoreSmc, PointSerie,
  ZoneOb, ZoneIfvg, NiveauxFibonacci, ResultatTendance,
  NiveauLiquidite, ReponseIndicators, IndicatorsParams,
  LigneTendanceKasper, ReponseTendanceMultiTf, AssetInfo, AnnonceCalendrier,
  SentimentMarche, EntiteSentiment, TraductionReponse,
  StatutSignalEngine, CouvertureDonnees, RequeteCollecte, ResultatCollecte, ResultatCollecteItem,
  HistoriqueEntrainement, HistoriqueML, PatternHoraire, ReponsePatternsVolatilite,
  StraddleCreneau, ReponseAnalyseStraddle, FearGreedData,
  SentimentComposite,
  RequeteSignalIA, ReponseSignalIA,
} from './api.types'

import type {
  RequeteAnalyseIA, ReponseAnalyseIA, ReponseChatIA, ReponseChartIA,
  ImageAvecTF, StatutIA, PredictionML, ScoreSmc,
  ReponseIndicators, IndicatorsParams,
  ReponseTendanceMultiTf, AssetInfo, Signal, Candle,
  ModeCalculTendance,
} from './api.types'
import type { StraddleParams as ParamsStraddle } from '@/generated/ParamsStraddle'
import type { SmcParams as ParamsSmc } from '@/generated/ParamsSmc'

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

  async getPrixAssets(assets: string[]): Promise<Record<string, number>> {
    try {
      const res = await http.get('/api/prix', {
        params: { assets: assets.join(',') },
        timeout: 10000,
      })
      return res.data as Record<string, number>
    } catch {
      return {}
    }
  },

  async getSignaux(limit = 20): Promise<Signal[]> {
    const res = await http.get('/api/signaux', { params: { limit } })
    return res.data
  },

  /** MFE des trades SL : { [id]: { mfe_r, meilleur_prix } } (ids absents = pas de bougies). */
  async getMfeSignaux(ids: string[]): Promise<Record<string, { mfe_r: number | null; meilleur_prix: number | null }>> {
    if (!ids.length) return {}
    try {
      const res = await http.post('/api/signaux/mfe', { ids })
      return res.data
    } catch {
      return {}
    }
  },

  /** Lot recalculé de chaque trade : capital composé au moment de l'émission
   *  × risque % / (stop en pips × valeur du pip) — colonne Lot (SMC/straddle). */
  async getLotsSignaux(ids: string[]): Promise<Record<string, number>> {
    if (!ids.length) return {}
    try {
      const res = await http.post('/api/signaux/lots', { ids })
      return res.data
    } catch {
      return {}
    }
  },

  async predictML(asset: string, timeframe = 'M15'): Promise<PredictionML> {
    const res = await http.get('/api/ml/predict', { params: { asset, timeframe } })
    return res.data
  },

  async analyseSmc(asset: string, timeframe = 'M15', limit = 200): Promise<ScoreSmc> {
    const res = await http.get('/api/smc/analyse', { params: { asset, timeframe, limit } })
    return res.data
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
    messages: { role: string; contenu: string }[],
    forcerOllama = false
  ): Promise<ReponseChatIA> {
    const res = await http.post('/api/ia/chat', { messages, forcer_ollama: forcerOllama }, { timeout: 300000 })
    return res.data
  },

  async genererDiagramme(sujet: string): Promise<ReponseChatIA> {
    const res = await http.post('/api/ia/diagram', { sujet }, { timeout: 300000 })
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

  async ajouterAsset(
    id: string,
    nom: string,
    type: AssetInfo['type'],
    source: 'binance' | 'mt5',
    symbolBybit?: string,
    symbolMt5?: string,
  ): Promise<void> {
    try {
      await http.post('/api/assets', {
        id, nom, type, source,
        symbol_bybit: symbolBybit || undefined,
        symbol_mt5: symbolMt5 || undefined,
      })
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

  async getStraddleParams(): Promise<ParamsStraddle> {
    const res = await http.get('/api/straddle/params')
    return res.data
  },

  async putStraddleParams(params: ParamsStraddle): Promise<void> {
    await http.put('/api/straddle/params', params)
  },

  async getSmcParams(): Promise<ParamsSmc> {
    const res = await http.get('/api/smc/params')
    return res.data
  },

  async putSmcParams(params: ParamsSmc): Promise<void> {
    await http.put('/api/smc/params', params)
  },

  // ── Prompts IA (CRUD) ──────────────────────────────────────────────────────
  // Structure dynamique { catégorie: { clé: { id, contenu, ... } } }.
  async getPrompts(): Promise<Record<string, any>> {
    const res = await http.get('/api/prompts')
    return res.data
  },

  async putPrompt(id: string, contenu: string): Promise<void> {
    await http.put(`/api/prompts/${id}`, { contenu })
  },

  async deletePrompt(id: string): Promise<void> {
    await http.delete(`/api/prompts/${id}`)
  },

  // ── ML : feature importance par stratégie ──────────────────────────────────
  async getMlFeatureImportance(strategie: string): Promise<{
    feature_idx: number
    feature_nom: string
    importance: number
  }[]> {
    const res = await http.get(`/api/ml/feature-importance/${strategie}`)
    return res.data
  },

  ...engineApi,
  ...rocketsApi,
  ...straddleApi,
  ...newsApi,
  ...workerApi,
  ...presseApi,
  ...alertesApi,
  ...apiSmcMethods,
}
