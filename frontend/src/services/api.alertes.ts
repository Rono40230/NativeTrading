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
}
