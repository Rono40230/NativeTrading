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
        @appliquer="chargerIndicateurs" @actualiser="actualiser">
        <template #apres-smc>
          <button
            class="h-7 px-3 rounded border transition-colors bg-purple-600/20 border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40"
            :disabled="signalStore.analyseIaChargement" @click="lancerAnalyseSmc">{{ signalStore.analyseIaChargement ? '🔍 Analyse...' : '🔍 Analyse SMC' }}</button>
        </template>
      </IndicatorPanel>

      <!-- Panneau signaux indicateurs -->
      <ChartSignauxPanel :signaux="signauxActifs"
        @update:filtre="onFiltreSignaux" />
    </div>

    <!-- Sidebar IA (toggle + drawer) -->
    <ChartSidebarIA :asset="selectedAsset" :timeframe="selectedTimeframe" :open="sidebarIA"
      @toggle="sidebarIA = !sidebarIA" />

    <!-- Modales (hors flux) -->
    <SignalModal :signal="signalModal" :niveaux="niveauxModal" :asset="selectedAsset" @fermer="signalModal = null" />
    <ChartAnalyseSmcModal
      :open="analyseSmcOuverte"
      :asset="selectedAsset"
      :timeframe="selectedTimeframe"
      :score-smc="signalStore.scoreSmc"
      :prix-entree="prixEntreeSnapshot"
      :sl-analyse="signalStore.slAnalyse"
      :tp1-analyse="signalStore.tp1Analyse"
      :tp2-analyse="signalStore.tp2Analyse"
      :chargement="signalStore.analyseIaChargement"
      :analyse-texte="signalStore.analyseIaTexte"
      @close="analyseSmcOuverte = false"
    />
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
import { limitPourTimeframe } from '@/composables/useChartLimite'
import { useSmcV12Overlay } from '@/composables/useSmcV12Overlay'
import { useChartEcoCal } from '@/composables/useChartEcoCal'
import EcoCalTooltip from '@/components/common/EcoCalTooltip.vue'

import { useChartOrchestration } from '@/composables/useChartOrchestration'
import { useSignalTradeBox } from '@/composables/useSignalTradeBox'
import { useSignalStore } from '@/stores/signal.store'
import ChartSidebarIA from '@/components/common/ChartSidebarIA.vue'
import ChartAnalyseSmcModal from '@/components/common/ChartAnalyseSmcModal.vue'
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
const prixEntreeSnapshot = ref<number>(0)

const {
  initChart, mettreAJourSerie, mettreAJourEnDirect, detruireChart,
  configurerRedimensionnement, arreterRedimensionnement, getChart, getCandlestickSeries,
} = useChartTradingView(chartContainer, bougies)

const { chargerEtAppliquer, reinitialiser, signauxActifs, appliquerMarqueursSignaux, mettreAJourSlTp, obtenirSignalEtNiveaux, dernierAtrValeur } = useChartIndicators()
const v12Overlay = useSmcV12Overlay()
const tradeBox = useSignalTradeBox()
const { initialiser: ecoCalInit, chargerAnnonces, detruire: ecoCalDetruire,
  tooltipAnnonce, tooltipX, tooltipY } = useChartEcoCal()


const timestampCurseur = ref<number | null>(null)
const filtreCourant = ref<FiltreSignaux>(filtreDefaut())
const signalModal = ref<SignalIndicateur | null>(null)
const niveauxModal = ref<NiveauSlTp | null>(null)
const sidebarIA = ref(false)
const analyseSmcOuverte = ref(false)

function onFiltreSignaux(f: FiltreSignaux) {
  filtreCourant.value = f
  appliquerMarqueursSignaux(getCandlestickSeries(), f)
}

async function lancerAnalyseSmc() {
  analyseSmcOuverte.value = true
  prixEntreeSnapshot.value = dernierPrix.value ?? 0
  const confianceMl = signalStore.prediction?.confiance ?? 0
  await signalStore.chargerAnalyseIA(selectedAsset.value, selectedTimeframe.value, prixEntreeSnapshot.value, confianceMl, dernierAtrValeur.value)
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
  if (chartContainer.value && serie) v12Overlay.initialiser(chart, serie, chartContainer.value)
  if (chartContainer.value && serie) tradeBox.initialiser(chartContainer.value, chart, serie)
  if (chartContainer.value) ecoCalInit(chart, chartContainer.value)
  // Overlay SMC v12 : fetch du replay moteur (indépendant des indicateurs classiques).
  const derniereB = bougies.value?.[bougies.value.length - 1]
  const tsSecV12 = derniereB ? Math.floor(new Date(derniereB.timestamp).getTime() / 1000) : undefined
  // Même fenêtre que le chart (5 000 = TV Basic) : parité de l'âge max des
  // zones avec TradingView — avant, l'analyse ne voyait que 500 bougies
  // (~5 jours M15) et perdait les OB plus anciens encore vivants.
  void v12Overlay.charger(selectedAsset.value, selectedTimeframe.value, limitPourTimeframe(selectedTimeframe.value), tsSecV12)
  await chargerEtAppliquer(
    chart, selectedAsset.value, selectedTimeframe.value, settingsStore.indicateurs,
    rsiContainer.value, macdContainer.value, atrContainer.value,
    serie, filtreCourant.value,
    (data) => {
      const derniereB2 = bougies.value?.[bougies.value.length - 1]
      const tsMs = derniereB2 ? new Date(derniereB2.timestamp).getTime() : null
      const tsSec = tsMs ? Math.floor(tsMs / 1000) : undefined
      v12Overlay.setDernierTs(tsSec)
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
  smcMettreAJourZones: (_data, _prefs, ts) => {
    v12Overlay.setDernierTs(ts)
  },
  chargerEtAppliquer, filtreCourant,
  mettreAJourSerie, mettreAJourEnDirect,
  initChart, detruireChart, reinitialiser,
  configurerCrosshair, configurerClick, chargerIndicateurs,
  configurerRedimensionnement, arreterRedimensionnement,
})

// Nettoyage Trade Box + overlay v12 au démontage de la vue
onUnmounted(() => {
  tradeBox.detruire()
  v12Overlay.detruire()
})

chargerAnnonces()
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
