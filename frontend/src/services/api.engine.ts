/**
 * Méthodes API données & ML historique (ex-Signal Engine — phase 2.8 :
 * endpoints signal-engine supprimés, le runtime v12 est officiel).
 */
import { http } from './http.client'
import type {
  CouvertureDonnees, ReponsePatternsVolatilite,
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

  async obtenirPatternsVolatilite(
    asset = 'BTC',
    timeframe = 'M15',
    mois = 12,
  ): Promise<ReponsePatternsVolatilite> {
    const res = await http.get('/api/volatility/patterns', { params: { asset, timeframe, mois } })
    return res.data
  },
}
