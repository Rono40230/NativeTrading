<template>
  <div class="relative h-full min-h-0">

    <!-- ── Zone graphique (plein format) ────────────────────────────── -->
    <div class="flex flex-col gap-4 w-full h-full min-h-0">
      <!-- Dernier prix + variation + metriques + sélecteurs -->
      <ChartPrixStats :dernier-prix="dernierPrix" :variation="variation" :stats="stats" :selected-asset="selectedAsset"
        :selected-timeframe="selectedTimeframe" :ws-connecte="marketStore.wsConnecte" :assets="assets"
        :timeframes="timeframes" @changer-asset="changerAsset" @changer-timeframe="changerTimeframe" />

      <!-- Canvas TradingView -->
      <div class="glass-card flex-1 min-h-0" style="min-height: 350px; position: relative;">
        <div v-if="marketStore.erreur"
          class="absolute inset-0 z-10 flex items-center justify-center bg-black/60 text-red-400 text-sm rounded-xl">
          ⚠ {{ marketStore.erreur }}
        </div>
        <div v-if="marketStore.erreurWs && !marketStore.wsConnecte"
          class="absolute bottom-2 left-2 z-10 px-3 py-1 rounded bg-yellow-900/70 text-yellow-300 text-xs border border-yellow-700/40">
          ⚠ {{ marketStore.erreurWs }}
        </div>
        <div v-if="marketStore.chargement"
          class="absolute inset-0 z-10 flex items-center justify-center bg-black/40 text-gray-400 text-sm rounded-xl">
          <span class="animate-pulse">Chargement des bougies...</span>
        </div>
        <div ref="chartContainer" class="w-full h-full" style="position: relative;" />
        <EcoCalTooltip :annonce="tooltipAnnonce" :x="tooltipX" :y="tooltipY" />
        <TendanceMultiTF v-if="settingsStore.indicateurs.kasperTendance" :key="selectedAsset + '_' + selectedTimeframe"
          :asset="selectedAsset" :timeframe="selectedTimeframe"
          :periode-rapide="settingsStore.indicateurs.kasperPeriodeRapide"
          :periode-lente="settingsStore.indicateurs.kasperPeriodeLente"
          :mode-calcul="settingsStore.indicateurs.kasperModeCalcul" />
      </div>

      <!-- Sous-graphique RSI séparé -->
      <div v-if="settingsStore.indicateurs.rsi" ref="rsiContainer" class="glass-card"
        style="height: 140px; position: relative;" />

      <!-- Sous-graphique MACD séparé -->
      <div v-if="settingsStore.indicateurs.macd" ref="macdContainer" class="glass-card"
        style="height: 140px; position: relative;" />

      <!-- Sous-graphique ATR séparé -->
      <div v-if="settingsStore.indicateurs.atr" ref="atrContainer" class="glass-card"
        style="height: 110px; position: relative;" />

      <!-- Panneau indicateurs (techniques + SMC) -->
      <IndicatorPanel v-model="settingsStore.indicateurs" :chargement="marketStore.chargement"
        @appliquer="chargerIndicateurs" @actualiser="actualiser" />

      <!-- Panneau signaux indicateurs -->
      <ChartSignauxPanel :signaux="signauxActifs"
        @update:filtre="onFiltreSignaux"
        @analyser="() => {}" />
    </div>

    <!-- Sidebar IA (toggle + drawer) -->
    <ChartSidebarIA :asset="selectedAsset" :timeframe="selectedTimeframe" :open="sidebarIA"
      @toggle="sidebarIA = !sidebarIA" />

    <!-- Modales (hors flux) -->
    <SignalModal :signal="signalModal" :niveaux="niveauxModal" :asset="selectedAsset" @fermer="signalModal = null" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onUnmounted } from 'vue'
import { useChartStats } from '@/composables/useChartStats'
import { useChartTradingView } from '@/composables/useChartTradingView'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import ChartSignauxPanel from '@/components/common/ChartSignauxPanel.vue'
import { filtreDefaut, type FiltreSignaux } from '@/composables/chartSignauxTypes'
import { useChartIndicators } from '@/composables/useChartIndicators'
import { useSmcCanvas } from '@/composables/useSmcCanvas'
import { useSmcLiqCanvas } from '@/composables/useSmcLiqCanvas'
import { useSmcFibCanvas } from '@/composables/useSmcFibCanvas'
import { useChartEcoCal } from '@/composables/useChartEcoCal'
import EcoCalTooltip from '@/components/common/EcoCalTooltip.vue'

import { useChartOrchestration } from '@/composables/useChartOrchestration'
import { useSignalTradeBox } from '@/composables/useSignalTradeBox'
import { useSignalStore } from '@/stores/signal.store'
import ChartSidebarIA from '@/components/common/ChartSidebarIA.vue'
import IndicatorPanel from '@/components/common/IndicatorPanel.vue'
import TendanceMultiTF from '@/components/common/TendanceMultiTF.vue'
import ChartPrixStats from '@/components/common/ChartPrixStats.vue'
import SignalModal from '@/components/common/SignalModal.vue'
import type { NiveauSlTp } from '@/composables/chartAtrSlTp'
import type { SignalIndicateur } from '@/composables/chartSignauxTypes'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()
const signalStore = useSignalStore()

const timeframes = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const selectedAsset = ref(settingsStore.assetActif)
const selectedTimeframe = ref(settingsStore.timeframeActif)
const chartContainer = ref<HTMLElement | null>(null)
const rsiContainer = ref<HTMLElement | null>(null)
const macdContainer = ref<HTMLElement | null>(null)
const atrContainer = ref<HTMLElement | null>(null)

const bougies = computed(() =>
  marketStore.getBougies(selectedAsset.value, selectedTimeframe.value)
)

const { dernierPrix, variation, stats } = useChartStats(bougies)

const {
  initChart, mettreAJourSerie, mettreAJourEnDirect, detruireChart,
  configurerRedimensionnement, arreterRedimensionnement, getChart, getCandlestickSeries,
} = useChartTradingView(chartContainer, bougies)

const { chargerEtAppliquer, reinitialiser, signauxActifs, appliquerMarqueursSignaux, mettreAJourSlTp, obtenirSignalEtNiveaux } = useChartIndicators()
const smcCanvas = useSmcCanvas()
const liqCanvas = useSmcLiqCanvas()
const fibCanvas = useSmcFibCanvas()
const tradeBox = useSignalTradeBox()
const { initialiser: ecoCalInit, chargerAnnonces, detruire: ecoCalDetruire,
  tooltipAnnonce, tooltipX, tooltipY } = useChartEcoCal()


const timestampCurseur = ref<number | null>(null)
const filtreCourant = ref<FiltreSignaux>(filtreDefaut())
const signalModal = ref<SignalIndicateur | null>(null)
const niveauxModal = ref<NiveauSlTp | null>(null)
const sidebarIA = ref(false)

function onFiltreSignaux(f: FiltreSignaux) {
  filtreCourant.value = f
  appliquerMarqueursSignaux(getCandlestickSeries(), f)
}

function configurerCrosshair() {
  getChart()?.subscribeCrosshairMove((param) => {
    const ts = param.time ? (param.time as number) : null
    timestampCurseur.value = ts
    mettreAJourSlTp(getCandlestickSeries(), ts, filtreCourant.value.afficherSlTp)
  })
}

function configurerClick() {
  getChart()?.subscribeClick((param) => {
    if (!param.hoveredObjectId) return
    const r = obtenirSignalEtNiveaux(String(param.hoveredObjectId))
    if (r) { signalModal.value = r.signal; niveauxModal.value = r.niveaux }
  })
}

async function chargerIndicateurs() {
  await nextTick()
  const chart = getChart()
  if (!chart) return
  const serie = getCandlestickSeries()
  if (chartContainer.value && serie) smcCanvas.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value && serie) liqCanvas.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value && serie) fibCanvas.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value && serie) tradeBox.initialiser(chartContainer.value, chart, serie)
  if (chartContainer.value) ecoCalInit(chart, chartContainer.value)
  await chargerEtAppliquer(
    chart, selectedAsset.value, selectedTimeframe.value, settingsStore.indicateurs,
    rsiContainer.value, macdContainer.value, atrContainer.value,
    serie, filtreCourant.value,
    (data) => {
      const derniereB = bougies.value?.[bougies.value.length - 1]
      const tsMs = derniereB ? new Date(derniereB.timestamp).getTime() : null
      const tsSec = tsMs ? Math.floor(tsMs / 1000) : undefined
      smcCanvas.mettreAJourZones(data, settingsStore.indicateurs, tsSec)
      liqCanvas.mettreAJour(data, settingsStore.indicateurs, tsSec)
      fibCanvas.mettreAJour(data.fibonacci, settingsStore.indicateurs, tsSec)
      // Trade Box — dernier signal SMC pour cet asset × TF
      const dernierSignal = signalStore.signaux.find(
        s => s.asset === selectedAsset.value && s.timeframe === selectedTimeframe.value
      ) ?? null
      tradeBox.mettreAJourSignal(dernierSignal)
    },
  )
}

const { assets, changerAsset, changerTimeframe, actualiser } = useChartOrchestration({
  selectedAsset, selectedTimeframe, bougies,
  indicateurs: ref(settingsStore.indicateurs),
  getChart, getCandlestickSeries,
  smcMettreAJourZones: (data, prefs, ts) => {
    smcCanvas.mettreAJourZones(data, prefs, ts)
    liqCanvas.mettreAJour(data, prefs, ts)
    fibCanvas.mettreAJour(data.fibonacci, prefs, ts)
  },
  chargerEtAppliquer, filtreCourant,
  mettreAJourSerie, mettreAJourEnDirect,
  initChart, detruireChart, reinitialiser,
  smcDetruire: smcCanvas.detruire,
  liqDetruire: liqCanvas.detruire,
  fibDetruire: fibCanvas.detruire,
  configurerCrosshair, configurerClick, chargerIndicateurs,
  configurerRedimensionnement, arreterRedimensionnement,
})

// Nettoyage Trade Box au démontage de la vue
onUnmounted(() => tradeBox.detruire())

chargerAnnonces()
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
