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
        <p class="label flex items-center justify-center">ROI <TooltipInfo texte="Retour sur investissement total du backtest. Objectif ≥ 15% pour la mise en production réelle." /></p>
        <p class="text-2xl font-bold" :class="resultats.roi_pct >= 0 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.roi_pct.toFixed(2) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Sharpe <TooltipInfo texte="Rapport rendement / risque annualisé. ≥ 1.5 = excellent, 1.0–1.5 = correct, < 1.0 = insuffisant." /></p>
        <p class="text-2xl font-bold" :class="resultats.sharpe_ratio >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.sharpe_ratio.toFixed(2) }}
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Win Rate <TooltipInfo texte="Pourcentage de trades gagnants sur le total des positions clôturées. Objectif ≥ 55%." /></p>
        <p class="text-2xl font-bold" :class="resultats.win_rate >= 55 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.win_rate.toFixed(1) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Max Drawdown <TooltipInfo texte="Perte maximale depuis un pic de capital. Au-delà de 20%, le trading s'arrête automatiquement." /></p>
        <p class="text-2xl font-bold" :class="resultats.max_drawdown_pct <= 20 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.max_drawdown_pct.toFixed(2) }}%
        </p>
      </div>
    </div>

    <!-- Métriques secondaires -->
    <div v-if="resultats" class="glass-card p-5 grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div>
        <p class="label flex items-center">Capital initial <TooltipInfo texte="Capital de départ utilisé pour simuler ce backtest (configurable dans les paramètres)." /></p>
        <p class="text-white font-semibold">{{ formatEur(resultats.capital_initial) }}</p>
      </div>
      <div>
        <p class="label flex items-center">Capital final <TooltipInfo texte="Valeur totale du portefeuille après l'ensemble des trades simulés sur la période." /></p>
        <p class="font-semibold" :class="resultats.capital_final >= resultats.capital_initial ? 'text-emerald-400' : 'text-red-400'">
          {{ formatEur(resultats.capital_final) }}
        </p>
      </div>
      <div>
        <p class="label flex items-center">Trades total <TooltipInfo texte="Nombre de positions ouvertes et fermées pendant la période de backtest analysée." /></p>
        <p class="text-white font-semibold">{{ resultats.total_trades }}</p>
      </div>
      <div>
        <p class="label flex items-center">Profit Factor <TooltipInfo texte="Ratio gains bruts / pertes brutes. ≥ 1.5 = performant, 1.0–1.5 = neutre, < 1.0 = stratégie perdante." /></p>
        <p class="font-semibold" :class="resultats.profit_factor >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.profit_factor.toFixed(2) }}
        </p>
      </div>
    </div>

    <!-- Courbe equity + Objectifs sur la même ligne (2/3 + 1/3) -->
    <div class="flex gap-4">
      <!-- Courbe equity — 3/4 -->
      <div class="glass-card p-5 flex-[3] min-w-0">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 flex items-center">Courbe Equity <TooltipInfo texte="Évolution du capital au fil du temps. Une pente régulièrement croissante traduit une stratégie stable et résiliente sur la durée." /></h2>
        <div v-if="chargement" class="text-center text-gray-500 py-8">Calcul en cours...</div>
        <div v-else-if="!resultats" class="text-center text-gray-500 py-8">
          Lancez un backtest pour voir la courbe equity
        </div>
        <div v-else ref="equityChart" class="h-52 w-full" />
      </div>

      <!-- Objectifs — 1/4 -->
      <div class="glass-card p-5 flex-[1] min-w-0">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center">Objectifs Production <TooltipInfo texte="Seuils minimaux requis pour déploiement en production réelle. ✓ = objectif atteint, ✗ = en dessous du seuil cible." /></h2>
        <div v-if="resultats" class="space-y-2">
          <ObjectifLigne label="ROI ≥ 15%" :atteint="resultats.roi_pct >= 15" :valeur="`${resultats.roi_pct.toFixed(1)}%`" />
          <ObjectifLigne label="Sharpe ≥ 1.5" :atteint="resultats.sharpe_ratio >= 1.5" :valeur="resultats.sharpe_ratio.toFixed(2)" />
          <ObjectifLigne label="Win Rate ≥ 55%" :atteint="resultats.win_rate >= 55" :valeur="`${resultats.win_rate.toFixed(1)}%`" />
          <ObjectifLigne label="Drawdown ≤ 20%" :atteint="resultats.max_drawdown_pct <= 20" :valeur="`${resultats.max_drawdown_pct.toFixed(1)}%`" />
        </div>
        <p v-else class="text-gray-500 text-sm pt-2">Lancez un backtest</p>
      </div>
    </div>

    <!-- Monitoring ML -->
    <MonitoringML />

    <!-- Test A/B Prompts -->
    <div v-if="abStats.length" class="glass-card p-5">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">⚖️ Comparaison A/B par stratégie</h2>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-gray-500 text-xs border-b border-white/10">
              <th class="text-left pb-2">Stratégie</th>
              <th class="text-right pb-2">Signaux</th>
              <th class="text-right pb-2">Wins</th>
              <th class="text-right pb-2">Win Rate</th>
              <th class="text-right pb-2">Conviction IA</th>
              <th class="text-right pb-2">Score SMC</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in abStats" :key="s.strategie" class="border-b border-white/5 hover:bg-white/5">
              <td class="py-2 font-medium text-white">{{ s.strategie }}</td>
              <td class="text-right text-gray-300">{{ s.nb_total }}</td>
              <td class="text-right text-emerald-400">{{ s.nb_wins }}</td>
              <td class="text-right font-semibold" :class="s.win_rate >= 55 ? 'text-emerald-400' : s.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">
                {{ s.win_rate.toFixed(1) }}%
              </td>
              <td class="text-right text-blue-400">{{ s.conviction_moy > 0 ? s.conviction_moy.toFixed(0) : '—' }}</td>
              <td class="text-right text-purple-400">{{ s.score_moy.toFixed(1) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted, defineComponent, h } from 'vue'
import { createChart, type IChartApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { BacktestResults } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { useAssetsStore } from '@/stores/assets.store'
import TooltipInfo from '@/components/common/TooltipInfo.vue'
import MonitoringML from '@/components/common/MonitoringML.vue'

// Inline sub-component pour les lignes d'objectif
const ObjectifLigne = defineComponent({
  props: { label: String, atteint: Boolean, valeur: String },
  setup(props) {
    return () =>
      h('div', { class: 'flex justify-between items-center py-1 border-b border-white/5' }, [
        h('span', { class: 'text-sm text-gray-300' }, props.label),
        h(
          'span',
          { class: `text-sm font-semibold ${props.atteint ? 'text-emerald-400' : 'text-red-400'}` },
          `${props.atteint ? '✓' : '✗'} ${props.valeur}`,
        ),
      ])
  },
})

const settingsStore = useSettingsStore()
const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const assets = computed(() =>
  assetsStore.assets.length > 0
    ? assetsStore.assets.map(a => a.id)
    : ['BTC', 'ETH']
)
const timeframes = ['M1', 'M5', 'M15', 'H1', 'H4', 'D1', 'W1']
const asset = ref(settingsStore.assetActif)
const timeframe = ref(settingsStore.timeframeActif)
const chargement = ref(false)
const resultats = ref<BacktestResults | null>(null)
const equityChart = ref<HTMLElement | null>(null)
let chart: IChartApi | null = null
let roEquity: ResizeObserver | null = null

function formatEur(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(v)
}

async function lancerBacktest() {
  chargement.value = true
  try {
    resultats.value = await apiService.runBacktest(
      asset.value,
      timeframe.value,
      settingsStore.capitalDepart,
      500,
    )
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

watch(equityChart, (el) => {
  if (el && resultats.value) afficherCourbe()
})

watch(equityChart, (el, old) => {
  roEquity?.disconnect()
  if (!el) return
  roEquity = new ResizeObserver(() => {
    chart?.applyOptions({ width: el.clientWidth })
  })
  roEquity.observe(el)
  if (old) roEquity.disconnect()
})

const abStats = ref<{ strategie: string; nb_total: number; nb_wins: number; nb_pertes: number; win_rate: number; conviction_moy: number; score_moy: number }[]>([])

onMounted(() => {
  assetsStore.chargerAssets()
  apiService.getAbTest().then(d => { abStats.value = d }).catch(() => {})
})
onUnmounted(() => {
  roEquity?.disconnect()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.btn-primary { @apply bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-semibold px-4 py-2 rounded-lg transition-all; }
.label { @apply text-xs text-gray-400 font-medium mb-1; }
</style>

