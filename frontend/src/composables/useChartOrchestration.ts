import { ref, watch, onMounted, onUnmounted, nextTick, type Ref, type ComputedRef } from 'vue'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import { apiService, type AssetInfo } from '@/services/api.service'
import type { ReponseIndicators } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { FiltreSignaux } from '@/composables/chartSignauxTypes'
import type { Candle } from '@/services/api.types'
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { limitPourTimeframe } from '@/composables/useChartLimite'

interface Opts {
  selectedAsset: Ref<string>
  selectedTimeframe: Ref<string>
  bougies: ComputedRef<Candle[] | undefined>
  indicateurs: Ref<PrefsIndicateurs>
  getChart: () => IChartApi | null
  getCandlestickSeries: () => ISeriesApi<'Candlestick'> | null
  smcMettreAJourZones: (data: ReponseIndicators, prefs: PrefsIndicateurs, ts?: number) => void
  chargerEtAppliquer: (
    chart: IChartApi, asset: string, tf: string, prefs: PrefsIndicateurs,
    rsi: HTMLElement | null, macd: HTMLElement | null, atr: HTMLElement | null,
    serie: ISeriesApi<'Candlestick'> | null, filtre: FiltreSignaux,
    onDonnees?: (data: ReponseIndicators) => void,
  ) => Promise<void>
  filtreCourant: Ref<FiltreSignaux>
  mettreAJourSerie: (force?: boolean) => void
  mettreAJourEnDirect: (bougie: Candle) => void
  initChart: () => void
  detruireChart: () => void
  reinitialiser: () => void
  smcDetruire: () => void
  liqDetruire: () => void
  configurerCrosshair: () => void
  configurerClick: () => void
  chargerIndicateurs: () => Promise<void>
  configurerRedimensionnement: () => void
  arreterRedimensionnement: () => void
}

export function useChartOrchestration(o: Opts) {
  const marketStore = useMarketStore()
  const settingsStore = useSettingsStore()
  const assets = ref<string[]>(['BTC', 'ETH', 'XAUUSD', 'XAGUSD'])
  let intervalZones: ReturnType<typeof setInterval> | null = null

  async function rafraichirZonesSmc() {
    const chart = o.getChart()
    const serie = o.getCandlestickSeries()
    if (!chart || !serie) return
    await o.chargerEtAppliquer(
      chart, o.selectedAsset.value, o.selectedTimeframe.value, o.indicateurs.value,
      null, null, null, serie, o.filtreCourant.value,
      (data) => {
        const derniereB = o.bougies.value?.[o.bougies.value.length - 1]
        const tsMs = derniereB ? new Date(derniereB.timestamp).getTime() : null
        const tsSec = tsMs ? Math.floor(tsMs / 1000) : undefined
        o.smcMettreAJourZones(data, o.indicateurs.value, tsSec)
      },
    )
  }

  function demarrerLiveFeed(asset: string, timeframe: string) {
    marketStore.connecterStream(asset, timeframe)
    if (intervalZones) clearInterval(intervalZones)
    intervalZones = setInterval(() => rafraichirZonesSmc(), 1_000)
  }

  function arreterLiveFeed() {
    marketStore.deconnecterStream()
    if (intervalZones) { clearInterval(intervalZones); intervalZones = null }
  }

  async function chargerEtReinitChart() {
    o.detruireChart()
    o.reinitialiser()
    await marketStore.chargerBougies(
      o.selectedAsset.value, o.selectedTimeframe.value,
      limitPourTimeframe(o.selectedTimeframe.value), true,
    )
    await nextTick()
    o.initChart()
    o.configurerCrosshair()
    o.configurerClick()
    await o.chargerIndicateurs()
  }

  async function changerAsset(asset: string) {
    o.selectedAsset.value = asset
    settingsStore.definirAsset(asset)
    arreterLiveFeed()
    await chargerEtReinitChart()
    demarrerLiveFeed(asset, o.selectedTimeframe.value)
  }

  async function changerTimeframe(tf: string) {
    o.selectedTimeframe.value = tf
    settingsStore.definirTimeframe(tf)
    arreterLiveFeed()
    await chargerEtReinitChart()
    demarrerLiveFeed(o.selectedAsset.value, tf)
  }

  async function actualiser() {
    await chargerEtReinitChart()
  }

  watch(o.bougies, () => { o.mettreAJourSerie(true) }, { deep: false })

  watch(() => marketStore.wsMiseAJour, (update) => {
    if (!update) return
    if (update.asset !== o.selectedAsset.value || update.timeframe !== o.selectedTimeframe.value) return
    o.mettreAJourEnDirect(update.bougie)
  })

  onMounted(async () => {
    try {
      const liste: AssetInfo[] = await apiService.obtenirAssets()
      if (liste.length > 0) assets.value = liste.map((a) => a.id)
    } catch { /* fallback liste statique */ }

    await marketStore.chargerBougies(
      o.selectedAsset.value, o.selectedTimeframe.value,
      limitPourTimeframe(o.selectedTimeframe.value), true,
    )
    o.initChart()
    o.configurerCrosshair()
    o.configurerClick()
    await o.chargerIndicateurs()
    demarrerLiveFeed(o.selectedAsset.value, o.selectedTimeframe.value)
    o.configurerRedimensionnement()
  })

  onUnmounted(() => {
    o.smcDetruire()
    o.liqDetruire()
    o.detruireChart()
    o.arreterRedimensionnement()
    arreterLiveFeed()
  })

  return { assets, changerAsset, changerTimeframe, actualiser }
}
