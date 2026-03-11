<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">💰 Profit &amp; Loss</h1>
      <div class="flex gap-3">
        <select v-model="asset" class="glass-select" @change="lancerBacktest">
          <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
        </select>
        <select v-model="timeframe" class="glass-select" @change="lancerBacktest">
          <option v-for="tf in timeframes" :key="tf" :value="tf">{{ tf }}</option>
        </select>
        <button class="btn-primary" :disabled="chargement" @click="lancerBacktest">
          {{ chargement ? '⏳ Calcul...' : '▶ Lancer Backtest' }}
        </button>
      </div>
    </div>

    <!-- KPIs -->
    <div v-if="resultats" class="grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div class="glass-card p-4 text-center">
        <p class="label">ROI</p>
        <p class="text-2xl font-bold" :class="resultats.roi_pct >= 0 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.roi_pct.toFixed(2) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label">Sharpe</p>
        <p class="text-2xl font-bold" :class="resultats.sharpe_ratio >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.sharpe_ratio.toFixed(2) }}
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label">Win Rate</p>
        <p class="text-2xl font-bold" :class="resultats.win_rate >= 55 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.win_rate.toFixed(1) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label">Max Drawdown</p>
        <p class="text-2xl font-bold" :class="resultats.max_drawdown_pct <= 20 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.max_drawdown_pct.toFixed(2) }}%
        </p>
      </div>
    </div>

    <!-- Métriques secondaires -->
    <div v-if="resultats" class="glass-card p-5 grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div>
        <p class="label">Capital initial</p>
        <p class="text-white font-semibold">{{ formatEur(resultats.capital_initial) }}</p>
      </div>
      <div>
        <p class="label">Capital final</p>
        <p class="font-semibold" :class="resultats.capital_final >= resultats.capital_initial ? 'text-emerald-400' : 'text-red-400'">
          {{ formatEur(resultats.capital_final) }}
        </p>
      </div>
      <div>
        <p class="label">Trades total</p>
        <p class="text-white font-semibold">{{ resultats.total_trades }}</p>
      </div>
      <div>
        <p class="label">Profit Factor</p>
        <p class="font-semibold" :class="resultats.profit_factor >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.profit_factor.toFixed(2) }}
        </p>
      </div>
    </div>

    <!-- Courbe equity -->
    <div class="glass-card p-5">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Courbe Equity</h2>
      <div v-if="chargement" class="text-center text-gray-500 py-8">Calcul en cours...</div>
      <div v-else-if="!resultats" class="text-center text-gray-500 py-8">
        Lancez un backtest pour voir la courbe equity
      </div>
      <div v-else ref="equityChart" class="h-64" />
    </div>

    <!-- Objectifs -->
    <div v-if="resultats" class="glass-card p-5">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Objectifs Production</h2>
      <div class="space-y-2">
        <ObjectifLigne label="ROI annualisé ≥ 15%" :atteint="resultats.roi_pct >= 15" :valeur="`${resultats.roi_pct.toFixed(1)}%`" />
        <ObjectifLigne label="Sharpe ≥ 1.5" :atteint="resultats.sharpe_ratio >= 1.5" :valeur="resultats.sharpe_ratio.toFixed(2)" />
        <ObjectifLigne label="Win Rate ≥ 55%" :atteint="resultats.win_rate >= 55" :valeur="`${resultats.win_rate.toFixed(1)}%`" />
        <ObjectifLigne label="Max Drawdown ≤ 20%" :atteint="resultats.max_drawdown_pct <= 20" :valeur="`${resultats.max_drawdown_pct.toFixed(1)}%`" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, defineComponent, h } from 'vue'
import { createChart, type IChartApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { BacktestResults } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'
import { useAlerteStore } from '@/stores/alerte.store'

// Inline sub-component pour les lignes d'objectif
const ObjectifLigne = defineComponent({
  props: { label: String, atteint: Boolean, valeur: String },
  setup(props) {
    return () => h('div', { class: 'flex justify-between items-center py-1 border-b border-white/5' }, [
      h('span', { class: 'text-sm text-gray-300' }, props.label),
      h('span', { class: `text-sm font-semibold ${props.atteint ? 'text-emerald-400' : 'text-red-400'}` },
        `${props.atteint ? '✓' : '✗'} ${props.valeur}`)
    ])
  }
})

const settingsStore = useSettingsStore()
const alerteStore = useAlerteStore()
const assets = ['BTC', 'ETH']
const timeframes = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1', 'W1']
const asset = ref(settingsStore.assetActif)
const timeframe = ref(settingsStore.timeframeActif)
const chargement = ref(false)
const resultats = ref<BacktestResults | null>(null)
const equityChart = ref<HTMLElement | null>(null)
let chart: IChartApi | null = null

function formatEur(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(v)
}

async function lancerBacktest() {
  chargement.value = true
  try {
    resultats.value = await apiService.runBacktest(asset.value, timeframe.value, settingsStore.capitalDepart, 500)
    await nextTick()
    afficherCourbe()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Backtest échoué: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

function afficherCourbe() {
  if (!equityChart.value || !resultats.value) return
  chart?.remove()
  chart = createChart(equityChart.value, {
    layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
    grid: { vertLines: { color: '#1f2937' }, horzLines: { color: '#1f2937' } },
    width: equityChart.value.clientWidth,
    height: 256,
  })
  const series = chart.addAreaSeries({
    lineColor: resultats.value.roi_pct >= 0 ? '#10b981' : '#ef4444',
    topColor: resultats.value.roi_pct >= 0 ? '#10b98133' : '#ef444433',
    bottomColor: 'transparent',
  })
  // Courbe equity simulée (capital_initial → capital_final, linéaire pour la démo)
  const n = Math.max(resultats.value.total_trades, 10)
  const pts = Array.from({ length: n }, (_, i) => ({
    time: (Math.floor(Date.now() / 1000) - (n - i) * 86400) as unknown as import('lightweight-charts').Time,
    value: resultats.value!.capital_initial + (resultats.value!.profit_net * i) / (n - 1),
  }))
  series.setData(pts)
}

watch(equityChart, (el) => { if (el && resultats.value) afficherCourbe() })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.btn-primary { @apply bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-semibold px-4 py-2 rounded-lg transition-all; }
.label { @apply text-xs text-gray-400 font-medium mb-1; }
</style>

