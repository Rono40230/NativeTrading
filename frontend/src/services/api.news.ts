/**
 * Méthodes API dédiées au flux news, sentiment et calendrier.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'
import type {
  AnnonceCalendrier, SentimentMarche, SentimentComposite, AlertesNews,
  ContenuArticle, TraductionReponse, FearGreedData,
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

  async obtenirFearGreed(): Promise<FearGreedData | null> {
    try {
      const res = await http.get('/api/news/fear-greed', { timeout: 6000 })
      return res.data
    } catch {
      return null
    }
  },

  async obtenirArticlesLus(): Promise<string[]> {
    try {
      const res = await http.get<{ urls: string[] }>('/api/news/lus')
      return res.data.urls
    } catch {
      return []
    }
  },

  async marquerArticleLu(url: string): Promise<void> {
    try {
      await http.post('/api/news/lu', { url })
    } catch {
      // Non-bloquant : la persistance lus est best-effort
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
}
