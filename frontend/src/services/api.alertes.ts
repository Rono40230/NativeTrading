/**
 * Méthodes API des alertes de prix — importées et spreadées dans apiService.
 */
import { http } from './http.client'

export interface AlertePrix {
  id: number
  asset: string
  prix: number
  sens: 'au_dessus' | 'en_dessous'
  note: string | null
  active: boolean
  cree_le: number
  declenchee_le: number | null
}

export const alertesApi = {
  async lister(): Promise<AlertePrix[]> {
    const res = await http.get('/api/alertes-prix')
    return res.data
  },

  async creer(asset: string, prix: number, sens: 'au_dessus' | 'en_dessous', note?: string): Promise<number> {
    const res = await http.post('/api/alertes-prix', { asset, prix, sens, note })
    return res.data.id
  },

  async supprimer(id: number): Promise<void> {
    await http.delete(`/api/alertes-prix/${id}`)
  },

  /// Réajustement par glisser-déposer sur le graphique (owner 20/10) —
  /// le sens est recalculé par l'appelant face au prix live au drop.
  async maj(id: number, prix: number, sens: 'au_dessus' | 'en_dessous'): Promise<void> {
    await http.put(`/api/alertes-prix/${id}`, { prix, sens })
  },
}
