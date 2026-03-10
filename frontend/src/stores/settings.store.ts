import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const CLE_CAPITAL = 'trading_capital_depart'
const CLE_ASSET = 'trading_asset_actif'
const CLE_TIMEFRAME = 'trading_timeframe_actif'
const CAPITAL_DEFAUT = 2000

export const useSettingsStore = defineStore('settings', () => {
  const capitalDepart = ref<number>(
    Number(localStorage.getItem(CLE_CAPITAL)) || CAPITAL_DEFAUT
  )
  const assetActif = ref<string>(localStorage.getItem(CLE_ASSET) || 'BTC')
  const timeframeActif = ref<string>(localStorage.getItem(CLE_TIMEFRAME) || 'M15')

  watch(capitalDepart, (val) => {
    if (val > 0) localStorage.setItem(CLE_CAPITAL, String(val))
  })
  watch(assetActif, (val) => localStorage.setItem(CLE_ASSET, val))
  watch(timeframeActif, (val) => localStorage.setItem(CLE_TIMEFRAME, val))

  function definirCapital(valeur: number) {
    if (valeur > 0) capitalDepart.value = valeur
  }

  function definirAsset(asset: string) {
    assetActif.value = asset
  }

  function definirTimeframe(tf: string) {
    timeframeActif.value = tf
  }

  return { capitalDepart, assetActif, timeframeActif, definirCapital, definirAsset, definirTimeframe }
})
