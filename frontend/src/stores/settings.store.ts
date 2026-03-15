import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const CLE_CAPITAL = 'trading_capital_depart'
const CLE_ASSET = 'trading_asset_actif'
const CLE_TIMEFRAME = 'trading_timeframe_actif'
const CLE_INDICATEURS = 'trading_indicateurs'
const CAPITAL_DEFAUT = 2000

export interface PrefsIndicateurs {
  ema: boolean
  rsi: boolean
  macd: boolean
  bollinger: boolean
  atr: boolean
  emaPeriode: number
  rsiPeriode: number
  smcOb: boolean
  smcFvg: boolean
  smcIfvg: boolean
  smcFib: boolean
  smcTendance: boolean
  smcLiquidites: boolean
  // Tendance Kasper Bootcamp
  kasperTendance: boolean
  kasperMmRapide: number
  kasperMmLente: number
  kasperMaType: 'ema' | 'sma'
}

const INDICATEURS_DEFAUT: PrefsIndicateurs = {
  ema: true,
  rsi: false,
  macd: false,
  bollinger: false,
  atr: false,
  emaPeriode: 20,
  rsiPeriode: 14,
  smcOb: true,
  smcFvg: true,
  smcIfvg: true,
  smcFib: true,
  smcTendance: true,
  smcLiquidites: true,
  // Tendance Kasper Bootcamp
  kasperTendance: false,
  kasperMmRapide: 9,
  kasperMmLente: 21,
  kasperMaType: 'ema',
}

function chargerIndicateurs(): PrefsIndicateurs {
  try {
    const raw = localStorage.getItem(CLE_INDICATEURS)
    if (raw) return { ...INDICATEURS_DEFAUT, ...JSON.parse(raw) }
  } catch {
    // données corrompues — on repart des valeurs par défaut
  }
  return { ...INDICATEURS_DEFAUT }
}

export const useSettingsStore = defineStore('settings', () => {
  const capitalDepart = ref<number>(
    Number(localStorage.getItem(CLE_CAPITAL)) || CAPITAL_DEFAUT
  )
  const assetActif = ref<string>(localStorage.getItem(CLE_ASSET) || 'BTC')
  const timeframeActif = ref<string>(localStorage.getItem(CLE_TIMEFRAME) || 'M15')
  const indicateurs = ref<PrefsIndicateurs>(chargerIndicateurs())

  watch(capitalDepart, (val) => {
    if (val > 0) localStorage.setItem(CLE_CAPITAL, String(val))
  })
  watch(assetActif, (val) => localStorage.setItem(CLE_ASSET, val))
  watch(timeframeActif, (val) => localStorage.setItem(CLE_TIMEFRAME, val))
  watch(indicateurs, (val) => localStorage.setItem(CLE_INDICATEURS, JSON.stringify(val)), { deep: true })

  function definirCapital(valeur: number) {
    if (valeur > 0) capitalDepart.value = valeur
  }

  function definirAsset(asset: string) {
    assetActif.value = asset
  }

  function definirTimeframe(tf: string) {
    timeframeActif.value = tf
  }

  return {
    capitalDepart,
    assetActif,
    timeframeActif,
    indicateurs,
    definirCapital,
    definirAsset,
    definirTimeframe,
  }
})

