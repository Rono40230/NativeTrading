/**
 * Méthodes API dédiées aux paramètres de sizing par asset.
 */
import axios from 'axios'
import type { AssetParams } from './api.types'

const http = axios.create({ baseURL: 'http://localhost:8080', timeout: 15000 })

export const assetParamsApi = {
  async getAssetParams(): Promise<AssetParams[]> {
    const res = await http.get('/api/assets/params')
    return res.data
  },

  async putAssetParams(liste: AssetParams[]): Promise<void> {
    await http.put('/api/assets/params', liste)
  },
}
