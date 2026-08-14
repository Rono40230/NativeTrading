import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { AssetInfo } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

export const useAssetsStore = defineStore('assets', () => {
  const assets = ref<AssetInfo[]>([])
  const chargement = ref(false)

  async function chargerAssets() {
    chargement.value = true
    try {
      const tous = await apiService.obtenirAssets()
      assets.value = tous.filter(a => a.actif)
    } catch (e: unknown) {
      useAlerteStore().afficherErreur(`Assets: ${(e as Error).message}`)
    } finally {
      chargement.value = false
    }
  }

  async function ajouterAsset(
    id: string,
    nom: string,
    type: AssetInfo['type'],
    source: 'binance',
  ) {
    await apiService.ajouterAsset(id, nom, type, source)
    await chargerAssets()
  }

  async function supprimerAsset(id: string) {
    await apiService.supprimerAsset(id)
    await chargerAssets()
  }

  return { assets, chargement, chargerAssets, ajouterAsset, supprimerAsset }
})
