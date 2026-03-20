<template>
  <!-- Layout 2 colonnes : contenu principal | (sentiment + calendrier) -->
  <div class="flex gap-3 items-start">

    <!-- Contenu principal -->
    <div class="flex-1 min-w-0 space-y-4">
      <!-- Capital + Statut Système -->
      <div class="flex flex-col gap-3">
        <div class="flex gap-4 items-stretch">
          <div class="glass-card p-5 shrink-0">
            <p class="label">Capital</p>
            <p class="kpi-value">{{ formatUsd(capital) }}</p>
            <p class="label mt-1 text-xs">Portefeuille initial</p>
            <router-link to="/settings" class="text-xs text-emerald-400 hover:underline mt-1 block">Modifier →</router-link>
          </div>
          <DashboardSystemStatus
            :backend-ok="backendOk"
            :btc-prix="btcPrix"
            :ib-gateway-ok="ibGatewayOk"
            :ml-pret="mlPret"
          />
        </div>
        <DashboardPrixStrip :assets="assetsDisplay" />
      </div>

      <!-- Horloges sessions de marché -->
      <MarketClocks />

      <!-- Métriques performance (backtest BTC) -->
      <div v-if="metriques" class="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div class="glass-card p-4"><p class="label">Win Rate</p><p class="kpi-value text-emerald-400">{{ metriques.win_rate.toFixed(1) }}%</p></div>
        <div class="glass-card p-4"><p class="label">ROI Backtest</p><p class="kpi-value" :class="metriques.roi_pct >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ metriques.roi_pct.toFixed(1) }}%</p></div>
        <div class="glass-card p-4"><p class="label">Total Trades</p><p class="kpi-value">{{ metriques.total_trades }}</p></div>
        <div class="glass-card p-4"><p class="label">Max Drawdown</p><p class="kpi-value text-red-400">{{ metriques.max_drawdown_pct.toFixed(1) }}%</p></div>
      </div>

      <!-- Derniers signaux -->
      <DashboardSignaux />
    </div>

    <!-- Colonne droite : Sentiment + Calendrier -->
    <aside class="w-64 shrink-0 sticky top-0 space-y-3">
      <SentimentMarche />
      <EconomicCalendar />
    </aside>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useSignalStore } from '@/stores/signal.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useMarketStore } from '@/stores/market.store'
import { apiService } from '@/services/api.service'
import type { BacktestResults, Candle } from '@/services/api.service'
import MarketClocks from '@/components/common/MarketClocks.vue'
import EconomicCalendar from '@/components/common/EconomicCalendar.vue'
import SentimentMarche from '@/components/common/SentimentMarche.vue'
import DashboardSystemStatus from '@/components/common/DashboardSystemStatus.vue'
import DashboardPrixStrip from '@/components/common/DashboardPrixStrip.vue'
import DashboardSignaux from '@/components/common/DashboardSignaux.vue'

type VariationsMultiTF = { h1: number | null; h4: number | null; d1: number | null; w1: number | null; m1: number | null }
type AssetAvecPrix = { id: string; prix: number | null; variation: number | null; variationsMultiTF: VariationsMultiTF | null; clotures: Record<string, number[]>; chargement: boolean }

const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const marketStore = useMarketStore()

const capital = computed(() => settingsStore.capitalDepart)
const mlPret = computed(() => signalStore.prediction?.modele_pret ?? false)
const backendOk = ref(false)
const ibGatewayOk = ref<boolean | null>(null)
const metriques = ref<BacktestResults | null>(null)
const assetsAvecPrix = ref<AssetAvecPrix[]>([])

const assetsDisplay = computed(() =>
  assetsAvecPrix.value.map(a => ({
    ...a,
    prix: marketStore.prixLive[a.id] ?? a.prix,
    variation: marketStore.variationLive[a.id] !== undefined ? marketStore.variationLive[a.id] : a.variation,
    chargement: a.chargement && marketStore.prixLive[a.id] === undefined,
  }))
)
const btcPrix = computed(() => marketStore.prixLive['BTC'] ?? assetsAvecPrix.value.find(a => a.id === 'BTC')?.prix ?? null)

let intervalPrix: ReturnType<typeof setInterval> | null = null

async function chargerPrixActifs() {
  function calcVar(candles: Candle[], idxOld = -2, idxNew = -1): number | null {
    const a = candles.at(idxNew)?.close
    const b = candles.at(idxOld)?.close
    return a != null && b != null && b !== 0 ? ((a - b) / b) * 100 : null
  }
  try {
    const liste = await apiService.obtenirAssets()
    assetsAvecPrix.value = liste.map(a => ({ id: a.id, prix: null, variation: null, variationsMultiTF: null, clotures: {}, chargement: true }))
    await Promise.allSettled(liste.map(async (a, i) => {
      try {
        const [bM15, bH1, bH4, bD1, bW1] = await Promise.allSettled([
          apiService.getCandles(a.id, 'M15', 2),
          apiService.getCandles(a.id, 'H1', 48),
          apiService.getCandles(a.id, 'H4', 30),
          apiService.getCandles(a.id, 'D1', 32),
          apiService.getCandles(a.id, 'W1', 20),
        ])
        const m15 = bM15.status === 'fulfilled' ? bM15.value : []
        const h1  = bH1.status  === 'fulfilled' ? bH1.value  : []
        const h4  = bH4.status  === 'fulfilled' ? bH4.value  : []
        const d1  = bD1.status  === 'fulfilled' ? bD1.value  : []
        const w1  = bW1.status  === 'fulfilled' ? bW1.value  : []
        const curr = m15.at(-1)?.close ?? null
        const prev = m15.at(-2)?.close ?? null
        assetsAvecPrix.value[i] = {
          id: a.id,
          prix: curr,
          variation: curr != null && prev != null && prev !== 0 ? ((curr - prev) / prev) * 100 : null,
          variationsMultiTF: {
            h1: calcVar(h1), h4: calcVar(h4), d1: calcVar(d1), w1: calcVar(w1),
            m1: d1.length >= 2 ? calcVar(d1, 0, -1) : null,
          },
          clotures: { h1: h1.map(c => c.close), h4: h4.map(c => c.close), d1: d1.map(c => c.close), w1: w1.map(c => c.close) },
          chargement: false,
        }
      } catch { assetsAvecPrix.value[i] = { id: a.id, prix: null, variation: null, variationsMultiTF: null, clotures: {}, chargement: false } }
    }))
  } catch { /* silencieux */ }
}

function formatUsd(v: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 }).format(v)
}

onMounted(async () => {
  try { await apiService.healthCheck(); backendOk.value = true } catch { backendOk.value = false }
  try { const ib = await apiService.ibStatus(); ibGatewayOk.value = ib.connecte } catch { ibGatewayOk.value = false }
  await Promise.allSettled([
    chargerPrixActifs(),
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
  const tousLesAssets = assetsAvecPrix.value.map(a => a.id)
  if (tousLesAssets.length > 0) marketStore.connecterPrixLiveAssets(tousLesAssets)
  intervalPrix = setInterval(chargerPrixActifs, 60000)
})

onUnmounted(() => {
  if (intervalPrix !== null) clearInterval(intervalPrix)
  marketStore.deconnecterPrixLiveAssets()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.label { @apply text-xs text-gray-400 font-medium; }
.kpi-value { @apply text-2xl font-bold text-white mt-1; }
</style>
