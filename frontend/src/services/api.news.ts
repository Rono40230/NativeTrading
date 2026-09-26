/**
 * Méthodes API dédiées au flux news, sentiment et calendrier.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  AnnonceCalendrier, SentimentMarche,
} from './api.types'

export const newsApi = {
  async obtenirCalendrier(days = 7): Promise<AnnonceCalendrier[]> {
    try {
      const res = await http.get('/api/calendar', { params: { days } })
      return res.data
    } catch {
      return []
    }
  },

  /// État honnête de la source (badge + modale : « aucune annonce » ≠
  /// « source injoignable »). Synchronisation : lundi/jeudi, jamais au
  /// redémarrage (décision owner 26/09).
  async etatCalendrier(): Promise<{ dernier_fetch: string | null; nb_futurs: number; source_prete: boolean }> {
    try {
      const res = await http.get('/api/calendar/etat')
      return res.data
    } catch {
      return { dernier_fetch: null, nb_futurs: 0, source_prete: false }
    }
  },

  async obtenirSentimentMarche(): Promise<SentimentMarche> {
    const res = await http.get('/api/sentiment/marche')
    return res.data
  },
}
