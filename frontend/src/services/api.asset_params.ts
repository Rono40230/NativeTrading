/**
 * Méthodes API dédiées aux paramètres de sizing par asset.
 */
import { http } from './http.client'
import type { AssetParams } from './api.types'

export const assetParamsApi = {
  async getAssetParams(): Promise<AssetParams[]> {
    const res = await http.get('/api/assets/params')
    return res.data
  },

  async putAssetParams(liste: AssetParams[]): Promise<void> {
    await http.put('/api/assets/params', liste)
  },
}
