/**
 * Méthodes API dédiées au Signal Engine, données et ML historique.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  StatutSignalEngine, CouvertureDonnees, RequeteCollecte,
  ResultatCollecte, HistoriqueML, ReponsePatternsVolatilite,
} from './api.types'

export const engineApi = {
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

  async obtenirCouvertureDonnees(): Promise<{
    couverture: CouvertureDonnees[]
    taille_db_octets?: number
    bougies_aujourd_hui?: number
  }> {
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
}
