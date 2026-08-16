/**
 * Méthodes API dédiées à la Revue de Presse.
 * Importées et spreadées dans apiService (api.service.ts).
 */
import { http } from './http.client'

export interface ArticlePresse {
  hash_titre: string; titre: string; url: string; source_nom: string
  publie_le: string; score: number; theme: string; assets_concernes: string
  impact: string; statut_traduction: string; lu: boolean; ajoute_le: number
  /** Résumé RSS capté à la collecte — socle d'affichage si le scraper échoue. */
  resume_source?: string
}
export interface BriefPresse {
  id: number; genere_le: number; fenetre_de: number; fenetre_a: number
  nb_articles: number; contenu: string
}

export const presseApi = {
  async articles(filtres: Partial<{ theme: string; asset: string; source: string; q: string; lu: string; page: number }> = {}): Promise<ArticlePresse[]> {
    const res = await http.get('/api/presse/articles', { params: filtres })
    return res.data.articles
  },
  async ouvrir(hash: string): Promise<{ article: ArticlePresse; titre_fr: string | null; sentiment: string | null }> {
    const res = await http.post(`/api/presse/articles/${hash}/ouvrir`, null, { timeout: 60_000 })
    return res.data
  },
  async genererBrief(): Promise<{ id: number; contenu: string; nb_articles: number }> {
    const res = await http.post('/api/presse/brief', null, { timeout: 180_000 })
    return res.data
  },
  async briefs(): Promise<BriefPresse[]> {
    const res = await http.get('/api/presse/briefs')
    return res.data
  },
  async sources(): Promise<{ id: number; nom: string; url_rss: string; poids_score: number; categorie: string; actif: boolean }[]> {
    const res = await http.get('/api/presse/sources')
    return res.data
  },
  async ajouterSource(nom: string, url: string, poids: number, categorie: string): Promise<{
    id: number
    description_incluse: boolean
    items_testes: number
    items_avec_description: number
    avertissement: string | null
  }> {
    const res = await http.post('/api/presse/sources', { nom, url_rss: url, poids, categorie }, { timeout: 15_000 })
    return res.data
  },
  async retirerSource(id: number): Promise<void> {
    await http.delete(`/api/presse/sources/${id}`)
  },
}
