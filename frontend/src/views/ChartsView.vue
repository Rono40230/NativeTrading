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
      <div ref="chartContainer" class="w-full h-full" />
      <!-- Tableau Tendance Kasper Bootcamp (overlay coin haut-gauche) -->
      <TendanceMultiTF
        v-if="settingsStore.indicateurs.kasperTendance"
        :key="selectedAsset + '_' + selectedTimeframe"
        :asset="selectedAsset"
        :timeframe="selectedTimeframe"
        :mm-rapide="settingsStore.indicateurs.kasperMmRapide"
        :mm-lente="settingsStore.indicateurs.kasperMmLente"
        :ma-type="settingsStore.indicateurs.kasperMaType"
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

    <!-- Prédiction IA + Score SMC -->
    <PredictionSMCPanel :asset="selectedAsset" :timeframe="selectedTimeframe" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useChartTradingView } from '@/composables/useChartTradingView'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import ChartSignauxPanel from '@/components/common/ChartSignauxPanel.vue'
import { filtreDefaut, type FiltreSignaux } from '@/composables/chartSignauxTypes'
import { useChartAnalyse } from '@/composables/useChartAnalyse'
import { useChartIndicators } from '@/composables/useChartIndicators'
import { apiService, type AssetInfo } from '@/services/api.service'
import PredictionSMCPanel from '@/components/common/PredictionSMCPanel.vue'
import IndicatorPanel from '@/components/common/IndicatorPanel.vue'
import TendanceMultiTF from '@/components/common/TendanceMultiTF.vue'
import ChartBarreControles from '@/components/common/ChartBarreControles.vue'
import ChartPrixStats from '@/components/common/ChartPrixStats.vue'
import AnalyseIAModal from '@/components/common/AnalyseIAModal.vue'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()

const assets = ref<string[]>(['BTC', 'ETH', 'XAUUSD', 'XAGUSD'])
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

const dernierPrix = computed(() => {
  const b = bougies.value
  return b.length > 0 ? b[b.length - 1].close : null
})

const variation = computed(() => {
  const b = bougies.value
  if (b.length < 2) return 0
  const avant = b[b.length - 2].close
  const apres = b[b.length - 1].close
  return ((apres - avant) / avant) * 100
})

const stats = computed(() => {
  const b = bougies.value
  if (b.length === 0) return null
  const high = Math.max(...b.map((c) => c.high))
  const low = Math.min(...b.map((c) => c.low))
  const volumeMoy = b.reduce((s, c) => s + c.volume, 0) / b.length
  const dernier = b[b.length - 1]
  const range = high - low
  const positionRange = range > 0 ? ((dernier.close - low) / range) * 100 : 50
  const volRelatif = volumeMoy > 0 ? dernier.volume / volumeMoy : 1
  const vwapDen = b.reduce((s, c) => s + c.volume, 0)
  const vwap = vwapDen > 0 ? b.reduce((s, c) => s + ((c.high + c.low + c.close) / 3) * c.volume, 0) / vwapDen : dernier.close
  return { count: b.length, high, low, volumeMoy, range, positionRange, volRelatif, vwap }
})

const {
  initChart,
  mettreAJourSerie,
  mettreAJourEnDirect,
  detruireChart,
  configurerRedimensionnement,
  arreterRedimensionnement,
  getChart,
  getCandlestickSeries,
} = useChartTradingView(chartContainer, bougies)

const { analyseEnCours, analyseResultat, analyseModele, analyserAvecLlava } =
  useChartAnalyse(getChart, selectedAsset, selectedTimeframe)

const { chargerEtAppliquer, reinitialiser, signauxActifs, appliquerMarqueursSignaux, mettreAJourSlTp } = useChartIndicators()

const timestampCurseur = ref<number | null>(null)
const filtreCourant = ref<FiltreSignaux>(filtreDefaut())

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

async function chargerIndicateurs() {
  await nextTick()
  const chart = getChart()
  if (!chart) return
  await chargerEtAppliquer(
    chart,
    selectedAsset.value,
    selectedTimeframe.value,
    settingsStore.indicateurs,
    rsiContainer.value,
    macdContainer.value,
    atrContainer.value,
    getCandlestickSeries(),
    filtreCourant.value,
  )
}

async function changerAsset(asset: string) {
  selectedAsset.value = asset
  settingsStore.definirAsset(asset)
  arreterLiveFeed()
  await chargerEtReinitChart()
  demarrerLiveFeed(asset, selectedTimeframe.value)
}

async function changerTimeframe(tf: string) {
  selectedTimeframe.value = tf
  settingsStore.definirTimeframe(tf)
  arreterLiveFeed()
  await chargerEtReinitChart()
  demarrerLiveFeed(selectedAsset.value, tf)
}

async function chargerData() {
  await marketStore.chargerBougies(selectedAsset.value, selectedTimeframe.value, 500)
}

async function chargerEtReinitChart() {
  detruireChart()
  reinitialiser() // Invalide les références aux séries de l'ancien chart

  await marketStore.chargerBougies(selectedAsset.value, selectedTimeframe.value, 500)

  // Attendre que Vue ait rendu le container (toujours monté)
  await nextTick()
  initChart()
  configurerCrosshair()
  await chargerIndicateurs()
}

async function actualiser() {
  await chargerEtReinitChart()
}

/** Démarre le live feed WebSocket (crypto via Binance, métaux via Finnhub) */
function demarrerLiveFeed(asset: string, timeframe: string) {
  marketStore.connecterStream(asset, timeframe)
}

function arreterLiveFeed() {
  marketStore.deconnecterStream()
}

// Rechargement complet (changement asset/timeframe) — les indicateurs se rechargent via chargerEtReinitChart
watch(bougies, () => {
  mettreAJourSerie(true)
}, { deep: false })

// Mise à jour live via WebSocket (sans recalcul complet)
watch(() => marketStore.wsMiseAJour, (update) => {
  if (!update) return
  if (update.asset !== selectedAsset.value || update.timeframe !== selectedTimeframe.value) return
  mettreAJourEnDirect(update.bougie)
})

onMounted(async () => {
  // Charger la liste des assets depuis l'API (enrichit automatiquement les nouveaux assets)
  try {
    const liste: AssetInfo[] = await apiService.obtenirAssets()
    if (liste.length > 0) assets.value = liste.map((a) => a.id)
  } catch {
    // fallback : liste statique déjà initialisée
  }

  await chargerData()
  initChart()
  configurerCrosshair()
  await chargerIndicateurs()
  demarrerLiveFeed(selectedAsset.value, selectedTimeframe.value)

  configurerRedimensionnement()
})

onUnmounted(() => {
  detruireChart()
  arreterRedimensionnement()
  arreterLiveFeed()
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
