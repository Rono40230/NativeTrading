import { defineStore } from 'pinia'
import { ref } from 'vue'
import { assetParamsApi } from '@/services/api.asset_params'
import type { AssetParams } from '@/services/api.types'

export const useAssetParamsStore = defineStore('assetParams', () => {
  const liste  = ref<AssetParams[]>([])
  const loaded = ref(false)
  const saving = ref(false)

  async function charger() {
    if (loaded.value) return
    try {
      liste.value  = await assetParamsApi.getAssetParams()
      loaded.value = true
    } catch (err) {
      console.error('[assetParams] Chargement échoué', err)
    }
  }

  async function sauvegarder(lignes: AssetParams[]): Promise<boolean> {
    saving.value = true
    try {
      await assetParamsApi.putAssetParams(lignes)
      liste.value  = lignes
      return true
    } catch (err) {
      console.error('[assetParams] Sauvegarde échouée', err)
      return false
    } finally {
      saving.value = false
    }
  }

  return { liste, loaded, saving, charger, sauvegarder }
})
