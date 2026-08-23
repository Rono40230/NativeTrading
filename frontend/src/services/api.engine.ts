/**
 * Méthodes API données & ML historique (ex-Signal Engine — phase 2.8 :
 * endpoints signal-engine supprimés, le runtime v12 est officiel).
 */
import { http } from './http.client'
import type {
  CouvertureDonnees, RequeteCollecte,
  ResultatCollecte, HistoriqueML, ReponsePatternsVolatilite,
} from './api.types'

export const engineApi = {
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
