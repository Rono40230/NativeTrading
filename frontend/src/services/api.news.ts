/**
 * Méthodes API dédiées au flux news, sentiment et calendrier.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  AnnonceCalendrier, SentimentMarche, SentimentComposite,
  TraductionReponse,
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

  async obtenirSentimentMarche(): Promise<SentimentMarche> {
    const res = await http.get('/api/sentiment/marche')
    return res.data
  },

  async obtenirSentimentComposite(): Promise<SentimentComposite | null> {
    try {
      const res = await http.get('/api/sentiment/composite', { timeout: 8000 })
      return res.data
    } catch {
      return null
    }
  },

  async traduire(texte: string, long = false): Promise<TraductionReponse> {
    const res = await http.get('/api/news/traduire', { params: { texte, long }, timeout: 60_000 })
    return res.data
  },
}
