<template>
  <div class="space-y-4">
    <!-- Contrôles -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="flex rounded-lg overflow-hidden border border-white/10">
        <button
          v-for="a in assets"
          :key="a"
          class="px-4 py-2 text-sm font-medium transition-colors"
          :class="selectedAsset === a ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
          @click="changerAsset(a)"
        >
          {{ a }}
        </button>
      </div>

      <div class="flex rounded-lg overflow-hidden border border-white/10">
        <button
          v-for="tf in timeframes"
          :key="tf"
          class="px-3 py-2 text-sm font-medium transition-colors"
          :class="selectedTimeframe === tf ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
          @click="changerTimeframe(tf)"
        >
          {{ tf }}
        </button>
      </div>

      <button
        class="ml-auto px-4 py-2 text-sm rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 transition-colors"
        :disabled="marketStore.chargement"
        @click="actualiser"
      >
        {{ marketStore.chargement ? '⏳ Chargement...' : '🔄 Actualiser' }}
      </button>

      <button
        class="px-4 py-2 text-sm rounded-lg bg-purple-600/20 border border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40 transition-colors"
        :disabled="analyseEnCours"
        @click="analyserAvecLlava"
      >
        {{ analyseEnCours ? '🔍 Analyse...' : '🔍 Analyser (IA)' }}
      </button>
    </div>

    <!-- Dernier prix + variation -->
    <div v-if="dernierPrix" class="flex items-baseline gap-3">
      <span class="text-3xl font-bold">{{ formatPrix(dernierPrix) }}</span>
      <span class="text-sm" :class="variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
        {{ variation >= 0 ? '+' : '' }}{{ variation.toFixed(2) }}%
      </span>
      <span class="text-xs text-gray-500">{{ selectedAsset.includes('USD') ? selectedAsset : `${selectedAsset}/USDT` }} · {{ selectedTimeframe }}</span>
      <span v-if="marketStore.wsConnecte" class="flex items-center gap-1 text-xs ml-2"
        :class="['BTC','ETH'].includes(selectedAsset) ? 'text-emerald-400' : 'text-blue-400'">
        <span class="w-1.5 h-1.5 rounded-full animate-pulse inline-block"
          :class="['BTC','ETH'].includes(selectedAsset) ? 'bg-emerald-400' : 'bg-blue-400'" />
        {{ ['BTC','ETH'].includes(selectedAsset) ? 'LIVE' : 'LIVE 5s' }}
      </span>
    </div>

    <!-- Canvas TradingView -->
    <div class="glass-card" style="height: 480px; position: relative;">
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
    </div>

    <!-- Statistiques bougies -->
    <div v-if="stats" class="grid grid-cols-2 gap-3 lg:grid-cols-4">
      <div class="glass-card p-4">
        <p class="label">Bougies</p>
        <p class="stat-value">{{ stats.count }}</p>
      </div>
      <div class="glass-card p-4">
        <p class="label">Volume moyen</p>
        <p class="stat-value">{{ formatVolume(stats.volumeMoy) }}</p>
      </div>
      <div class="glass-card p-4">
        <p class="label">Plus haut</p>
        <p class="stat-value text-emerald-400">{{ formatPrix(stats.high) }}</p>
      </div>
      <div class="glass-card p-4">
        <p class="label">Plus bas</p>
        <p class="stat-value text-red-400">{{ formatPrix(stats.low) }}</p>
      </div>
    </div>

    <!-- Analyse vision llava -->
    <div v-if="analyseResultat" class="glass-card p-4 border-purple-500/30">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-semibold text-purple-400">🤖 Analyse visuelle IA — {{ analyseModele }}</span>
        <button class="text-gray-500 hover:text-white text-xs px-2" @click="analyseResultat = null">✕</button>
      </div>
      <p class="text-sm text-gray-200 leading-relaxed whitespace-pre-wrap">{{ analyseResultat }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useChartTradingView } from '@/composables/useChartTradingView'
import { useMarketStore } from '@/stores/market.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useChartAnalyse } from '@/composables/useChartAnalyse'

const marketStore = useMarketStore()
const settingsStore = useSettingsStore()

const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD']
const timeframes = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1', 'W1']
const selectedAsset = ref(settingsStore.assetActif)
const selectedTimeframe = ref(settingsStore.timeframeActif)
const chartContainer = ref<HTMLElement | null>(null)

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
  return { count: b.length, high, low, volumeMoy }
})

const {
  initChart,
  mettreAJourSerie,
  mettreAJourEnDirect,
  detruireChart,
  configurerRedimensionnement,
  arreterRedimensionnement,
  getChart,
} = useChartTradingView(chartContainer, bougies)

const { analyseEnCours, analyseResultat, analyseModele, analyserAvecLlava } =
  useChartAnalyse(getChart, selectedAsset, selectedTimeframe, dernierPrix, stats)

function formatPrix(v: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(v)
}

function formatVolume(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(2)}M`
  if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`
  return v.toFixed(2)
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
  await marketStore.chargerBougies(selectedAsset.value, selectedTimeframe.value, 200)
}

async function chargerEtReinitChart() {
  // Détruire l'ancien graphique avant de changer les données
  detruireChart()

  await marketStore.chargerBougies(selectedAsset.value, selectedTimeframe.value, 200)

  // Attendre que Vue ait rendu le container (toujours monté)
  await nextTick()
  initChart()
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

// Rechargement complet (changement asset/timeframe)
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
  await chargerData()
  initChart()
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
.label {
  @apply text-xs text-gray-400 font-medium;
}
.stat-value {
  @apply text-xl font-bold text-white mt-1;
}
</style>
