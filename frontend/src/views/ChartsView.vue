<template>
  <div class="space-y-4">
    <!-- Controles -->
    <ChartBarreControles
      :assets="assets"
      :timeframes="timeframes"
      :selected-asset="selectedAsset"
      :selected-timeframe="selectedTimeframe"
      :chargement="marketStore.chargement"
      :analyse-en-cours="analyseEnCours"
      @changer-asset="changerAsset"
      @changer-timeframe="changerTimeframe"
      @actualiser="actualiser"
      @analyser="analyserAvecLlava"
    />

    <!-- Panneau indicateurs (toujours visible) -->
    <IndicatorPanel v-model="settingsStore.indicateurs" @appliquer="chargerIndicateurs" />

    <!-- Dernier prix + variation + metriques -->
    <ChartPrixStats
      :dernier-prix="dernierPrix"
      :variation="variation"
      :stats="stats"
      :selected-asset="selectedAsset"
      :selected-timeframe="selectedTimeframe"
      :ws-connecte="marketStore.wsConnecte"
    />

    <!-- Canvas TradingView -->
    <div class="glass-card" style="height: 500px; position: relative;">
      <!-- Overlay erreur de chargement REST (bloquant) -->
      <div v-if="marketStore.erreur" class="absolute inset-0 z-10 flex items-center justify-center bg-black/60 text-red-400 text-sm rounded-xl">
        ⚠ {{ marketStore.erreur }}
      </div>
      <!-- Badge erreur WS (non bloquant — données REST toujours affichées) -->
      <div v-if="marketStore.erreurWs && !marketStore.wsConnecte" class="absolute bottom-2 left-2 z-10 px-3 py-1 rounded bg-yellow-900/70 text-yellow-300 text-xs border border-yellow-700/40">
        ⚠ {{ marketStore.erreurWs }}
      </div>
      <!-- Overlay chargement -->
      <div v-if="marketStore.chargement" class="absolute inset-0 z-10 flex items-center justify-center bg-black/40 text-gray-400 text-sm rounded-xl">
        <span class="animate-pulse">Chargement des bougies...</span>
      </div>
      <!-- Container toujours monté pour éviter la destruction du canvas -->
      <div ref="chartContainer" class="w-full h-full" style="position: relative;" />
      <!-- Tableau Tendance Kasper Bootcamp (overlay coin haut-gauche) -->
      <TendanceMultiTF
        v-if="settingsStore.indicateurs.kasperTendance"
        :key="selectedAsset + '_' + selectedTimeframe"
        :asset="selectedAsset"
        :timeframe="selectedTimeframe"
        :periode-rapide="settingsStore.indicateurs.kasperPeriodeRapide"
        :periode-lente="settingsStore.indicateurs.kasperPeriodeLente"
        :mode-calcul="settingsStore.indicateurs.kasperModeCalcul"
      />
    </div>

    <!-- Sous-graphique RSI séparé -->
    <div
      v-if="settingsStore.indicateurs.rsi"
      ref="rsiContainer"
      class="glass-card"
      style="height: 140px; position: relative;"
    />

    <!-- Sous-graphique MACD séparé -->
    <div
      v-if="settingsStore.indicateurs.macd"
      ref="macdContainer"
      class="glass-card"
      style="height: 140px; position: relative;"
    />

    <!-- Sous-graphique ATR séparé -->
    <div
      v-if="settingsStore.indicateurs.atr"
      ref="atrContainer"
      class="glass-card"
      style="height: 110px; position: relative;"
    />

    <!-- Panneau signaux indicateurs -->
    <ChartSignauxPanel
      :signaux="signauxActifs"
      :timestamp-curseur="timestampCurseur"
      @update:filtre="onFiltreSignaux"
    />

    <!-- Analyse vision llava — modale draggable -->
    <AnalyseIAModal
      :analyse="analyseResultat"
      :modele="analyseModele"
      @fermer="analyseResultat = null"
    />

    <!-- Modale signal — clic sur marker graphique -->
    <SignalModal
      :signal="signalModal"
      :niveaux="niveauxModal"
      @fermer="signalModal = null"
    />

    <!-- Prédiction IA + Score SMC -->
    <PredictionSMCPanel :asset="selectedAsset" :timeframe="selectedTimeframe" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useChartStats } from '@/composables/useChartStats'
import { useChartTradingView } from '@/composables/useChartTradingView'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import ChartSignauxPanel from '@/components/common/ChartSignauxPanel.vue'
import { filtreDefaut, type FiltreSignaux } from '@/composables/chartSignauxTypes'
import { useChartAnalyse } from '@/composables/useChartAnalyse'
import { useChartIndicators } from '@/composables/useChartIndicators'
import { useSmcCanvas } from '@/composables/useSmcCanvas'
import { useSessionsCanvas } from '@/composables/useSessionsCanvas'
import { useChartOrchestration } from '@/composables/useChartOrchestration'
import PredictionSMCPanel from '@/components/common/PredictionSMCPanel.vue'
import IndicatorPanel from '@/components/common/IndicatorPanel.vue'
import TendanceMultiTF from '@/components/common/TendanceMultiTF.vue'
import ChartBarreControles from '@/components/common/ChartBarreControles.vue'
import ChartPrixStats from '@/components/common/ChartPrixStats.vue'
import AnalyseIAModal from '@/components/common/AnalyseIAModal.vue'
import SignalModal from '@/components/common/SignalModal.vue'
import type { NiveauSlTp } from '@/composables/chartAtrSlTp'
import type { SignalIndicateur } from '@/composables/chartSignauxTypes'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()

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

const { analyseEnCours, analyseResultat, analyseModele, analyserAvecLlava } =
  useChartAnalyse(getChart, selectedAsset, selectedTimeframe)

const { chargerEtAppliquer, reinitialiser, signauxActifs, appliquerMarqueursSignaux, mettreAJourSlTp, obtenirSignalEtNiveaux } = useChartIndicators()
const smcCanvas = useSmcCanvas()
const sessionsCanvas = useSessionsCanvas()

const timestampCurseur = ref<number | null>(null)
const filtreCourant = ref<FiltreSignaux>(filtreDefaut())
const signalModal = ref<SignalIndicateur | null>(null)
const niveauxModal = ref<NiveauSlTp | null>(null)

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
    const r = obtenirSignalEtNiveaux(param.hoveredObjectId)
    if (r) { signalModal.value = r.signal; niveauxModal.value = r.niveaux }
  })
}

async function chargerIndicateurs() {
  await nextTick()
  const chart = getChart()
  if (!chart) return
  const serie = getCandlestickSeries()
  if (chartContainer.value && serie) smcCanvas.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value) sessionsCanvas.initialiser(chart, chartContainer.value, settingsStore.indicateurs)
  await chargerEtAppliquer(
    chart, selectedAsset.value, selectedTimeframe.value, settingsStore.indicateurs,
    rsiContainer.value, macdContainer.value, atrContainer.value,
    serie, filtreCourant.value,
    (data) => {
      const derniereB = bougies.value?.[bougies.value.length - 1]
      const tsMs = derniereB ? new Date(derniereB.timestamp).getTime() : null
      smcCanvas.mettreAJourZones(data, settingsStore.indicateurs, tsMs ? Math.floor(tsMs / 1000) : undefined)
    },
  )
}

const { assets, changerAsset, changerTimeframe, actualiser } = useChartOrchestration({
  selectedAsset, selectedTimeframe, bougies,
  indicateurs: ref(settingsStore.indicateurs),
  getChart, getCandlestickSeries,
  smcMettreAJourZones: smcCanvas.mettreAJourZones,
  chargerEtAppliquer, filtreCourant,
  mettreAJourSerie, mettreAJourEnDirect,
  initChart, detruireChart, reinitialiser,
  smcDetruire: smcCanvas.detruire,
  sessionsDetruire: sessionsCanvas.detruire,
  configurerCrosshair, configurerClick, chargerIndicateurs,
  configurerRedimensionnement, arreterRedimensionnement,
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
