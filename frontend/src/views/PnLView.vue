<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <button class="text-gray-400 hover:text-white transition-colors" title="Retour aux créneaux de volatilité" @click="$router.push('/straddle')">← </button>
        <h1 class="text-2xl font-bold">💰 Profit &amp; Loss</h1>
      </div>
      <div class="flex gap-3">
        <select v-model="asset" class="glass-select" @change="lancerBacktest">
          <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
        </select>
        <select v-model="dureeLabel" class="glass-select" @change="lancerBacktest">
          <option v-for="d in dureesDisponibles" :key="d.label" :value="d.label">{{ d.label }}</option>
        </select>
        <span v-if="chargement" class="text-sm text-gray-400">⏳ Calcul...</span>
      </div>
    </div>
    <!-- Avertissement données manquantes -->
    <div v-if="resultats && resultats.total_trades === 0" class="glass-card p-3 border-yellow-500/30 bg-yellow-900/10 flex items-center gap-3 text-sm">
      <span class="text-yellow-400">⚠</span>
      <span class="text-yellow-300 font-semibold">Aucun trade simulé</span>
      <span class="text-yellow-200/60">— Pas assez de bougies en base. Vérifiez la connexion à la source de données.</span>
    </div>

    <!-- Bloc métriques unifié : badges pleine largeur -->
    <div v-if="resultats" class="glass-card px-3 py-2.5 flex items-stretch gap-2">
      <!-- Groupe 1 — KPIs (4 badges) -->
      <div class="flex flex-1 gap-2">
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.roi_pct >= 15 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.roi_pct >= 0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 flex items-center gap-0.5 mb-1">ROI <TooltipInfo texte="Retour sur investissement sur la période simulée." :niveaux="niveaux?.roi" /></span>
          <span class="text-base font-bold leading-none" :class="resultats.roi_pct >= 15 ? 'text-emerald-400' : resultats.roi_pct >= 0 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.roi_pct.toFixed(2) }}%</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.sharpe_ratio >= 1.5 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.sharpe_ratio >= 1.0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 flex items-center gap-0.5 mb-1">Sharpe <TooltipInfo texte="Rapport rendement / risque ajusté (annualisé)." :niveaux="niveaux?.sharpe" /></span>
          <span class="text-base font-bold leading-none" :class="resultats.sharpe_ratio >= 1.5 ? 'text-emerald-400' : resultats.sharpe_ratio >= 1.0 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.sharpe_ratio.toFixed(2) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.win_rate >= 55 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.win_rate >= 45 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 flex items-center gap-0.5 mb-1">Win Rate <TooltipInfo texte="Pourcentage de trades clôturés avec un gain positif." :niveaux="niveaux?.winRate" /></span>
          <span class="text-base font-bold leading-none" :class="resultats.win_rate >= 55 ? 'text-emerald-400' : resultats.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.win_rate.toFixed(1) }}%</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.max_drawdown_pct <= 20 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.max_drawdown_pct <= 30 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 flex items-center gap-0.5 mb-1">Drawdown <TooltipInfo texte="Perte maximale depuis un pic de portefeuille. Au-delà de 20%, le trading s'arrête automatiquement." :niveaux="niveaux?.drawdown" /></span>
          <span class="text-base font-bold leading-none" :class="resultats.max_drawdown_pct <= 20 ? 'text-emerald-400' : resultats.max_drawdown_pct <= 30 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.max_drawdown_pct.toFixed(2) }}%</span>
        </div>
      </div>

      <!-- Séparateur -->
      <div class="w-px bg-white/10 shrink-0 self-stretch" />

      <!-- Groupe 2 — Résumé capital (5 badges) -->
      <div class="flex flex-1 gap-2">
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg bg-white/5 border border-white/10">
          <span class="text-[11px] text-gray-400 mb-1">Cap. initial</span>
          <span class="text-sm font-semibold leading-none text-white">{{ formatEur(resultats.capital_initial) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.capital_final >= resultats.capital_initial ? 'bg-emerald-900/30 border-emerald-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Cap. final</span>
          <span class="text-sm font-semibold leading-none" :class="resultats.capital_final >= resultats.capital_initial ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.capital_final) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg bg-white/5 border border-white/10">
          <span class="text-[11px] text-gray-400 mb-1">Trades</span>
          <span class="text-sm font-semibold leading-none text-white">{{ resultats.total_trades }} <span class="text-emerald-400 text-xs">{{ resultats.winning_trades }}W</span> <span class="text-red-400 text-xs">{{ resultats.losing_trades }}L</span></span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.profit_factor >= 1.5 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.profit_factor >= 1.0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Profit Factor</span>
          <span class="text-sm font-semibold leading-none" :class="resultats.profit_factor >= 1.5 ? 'text-emerald-400' : resultats.profit_factor >= 1.0 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.profit_factor.toFixed(2) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.profit_net >= 0 ? 'bg-emerald-900/30 border-emerald-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Profit net</span>
          <span class="text-sm font-semibold leading-none" :class="resultats.profit_net >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.profit_net) }}</span>
        </div>
      </div>

      <!-- Séparateur -->
      <div class="w-px bg-white/10 shrink-0 self-stretch" />

      <!-- Groupe 3 — Sorties pyramidales (4 badges) -->
      <div class="flex flex-1 gap-2">
        <div v-for="p in pyramidalisation" :key="p.label" class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="p.classes">
          <span class="text-[11px] text-gray-400 mb-1">{{ p.label }}</span>
          <span class="text-sm font-bold leading-none" :class="p.color">{{ p.n }} <span class="text-[11px] text-gray-500 font-normal">{{ resultats.total_trades > 0 ? `${((p.n / resultats.total_trades) * 100).toFixed(0)}%` : '' }}</span></span>
        </div>
      </div>
    </div>

    <!-- Courbe equity (50%) + colonne droite objectifs/straddle empilés (50%) -->
    <div class="flex gap-4 items-stretch">
      <!-- Courbe equity — 50% -->
      <div class="glass-card p-5 w-1/2 min-w-0 flex flex-col">
        <div class="mb-4 flex items-center justify-between gap-3 shrink-0">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider flex items-center">Courbe Equity <TooltipInfo texte="Évolution du capital au fil du temps. Une pente régulièrement croissante traduit une stratégie stable et résiliente sur la durée." /></h2>
          <p class="text-[11px] text-gray-500 text-right">
            Période sélectionnée: {{ dureeLabel }} | Asset: {{ asset }} | TF: {{ timeframe }}
          </p>
        </div>
        <div v-if="chargement" class="text-center text-gray-500 flex-1 flex items-center justify-center">Calcul en cours...</div>
        <div v-else-if="!resultats" class="text-center text-gray-500 flex-1 flex items-center justify-center">
          Lancez un backtest pour voir la courbe equity
        </div>
        <div v-else class="flex-1 min-h-[140px] w-full flex flex-col">
          <div ref="equityChart" class="flex-1 min-h-[140px] w-full" />
        </div>
      </div>

      <!-- Colonne droite — 50% : objectifs + straddle empilés -->
      <div class="w-1/2 min-w-0 flex flex-col gap-4">
        <div class="glass-card p-5 flex flex-col">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center shrink-0">Objectifs Production <TooltipInfo texte="Seuils minimaux requis pour déploiement en production réelle. ✓ = objectif atteint, ✗ = en dessous du seuil cible." /></h2>
          <div v-if="resultats" class="grid grid-cols-2 gap-2">
            <ObjectifLigne label="ROI ≥ 15%" :atteint="resultats.roi_pct >= 15" :valeur="`${resultats.roi_pct.toFixed(1)}%`" />
            <ObjectifLigne label="Sharpe ≥ 1.5" :atteint="resultats.sharpe_ratio >= 1.5" :valeur="resultats.sharpe_ratio.toFixed(2)" />
            <ObjectifLigne label="Win Rate ≥ 55%" :atteint="resultats.win_rate >= 55" :valeur="`${resultats.win_rate.toFixed(1)}%`" />
            <ObjectifLigne label="Drawdown ≤ 20%" :atteint="resultats.max_drawdown_pct <= 20" :valeur="`${resultats.max_drawdown_pct.toFixed(1)}%`" />
          </div>
          <p v-else class="text-gray-500 text-sm">Lancez un backtest</p>
        </div>

        <StraddleParamsPanel
          v-model="straddleParams"
          :has-resultats="!!resultats"
          :chargement-llm="chargementLlm"
          :suggestion="suggestionLlm"
          @optimiser="demanderOptimisation"
          @relancer="lancerBacktest"
        />
      </div>
    </div>

    <!-- Monitoring ML -->
    <MonitoringML />

    <!-- Test A/B Prompts (composant indépendant) -->
    <AbTestTable />
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
import AbTestTable from '@/components/common/AbTestTable.vue'
import { tickMarkFormatterEquity } from '@/composables/chartTimeScale'
import { useBacktestDuree } from '@/composables/useBacktestDuree'
import { useCreneauParams } from '@/composables/useCreneauParams'
import { usePnLNiveaux } from '@/composables/usePnLNiveaux'
import StraddleParamsPanel from '@/components/common/StraddleParamsPanel.vue'
const ObjectifLigne = defineComponent({
  props: { label: String, atteint: Boolean, valeur: String },
  setup: (p) => () => h('div', { class: 'flex justify-between items-center py-1 border-b border-white/5' }, [
    h('span', { class: 'text-sm text-gray-300' }, p.label),
    h('span', { class: `text-sm font-semibold ${p.atteint ? 'text-emerald-400' : 'text-red-400'}` }, `${p.atteint ? '✓' : '✗'} ${p.valeur}`),
  ])
})

const settingsStore = useSettingsStore()
const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const assets = computed(() => assetsStore.assets.length > 0 ? assetsStore.assets.map(a => a.id) : ['BTC', 'ETH'])
const asset = ref(settingsStore.assetActif)
const timeframe = ref('M5')
const { dureeLabel, dureesDisponibles, limiteBougies } = useBacktestDuree(timeframe)
const { modeCreneau, creneauApi } = useCreneauParams()
const chargement = ref(false)
const resultats = ref<BacktestResults | null>(null)
const { niveaux, pyramidalisation } = usePnLNiveaux(resultats)
const equityChart = ref<HTMLElement | null>(null)
let chart: IChartApi | null = null
let roEquity: ResizeObserver | null = null
const straddleParams = ref({ tp_mult_1: 2.0, tp_mult_2: 3.5, tp_mult_3: 5.0, sl_mult: 0.5, seuil_atr: 1.5 })
const suggestionLlm = ref<string | null>(null)
const chargementLlm = ref(false)

function formatEur(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(v)
}

async function lancerBacktest() {
  chargement.value = true
  try {
    resultats.value = await apiService.runBacktest(asset.value, timeframe.value, settingsStore.capitalDepart, limiteBougies.value, creneauApi(), straddleParams.value)
    if (modeCreneau.value?.id && resultats.value) {
      await apiService.patchStraddleCreneau(modeCreneau.value.id, {
        backtest_winrate: resultats.value.win_rate,
        backtest_profit_factor: resultats.value.profit_factor,
      })
    }
    await nextTick()
    afficherCourbe()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Backtest échoué: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function demanderOptimisation() {
  if (!resultats.value) return
  chargementLlm.value = true
  suggestionLlm.value = null
  try {
    const r = resultats.value
    const suggestion = await apiService.demanderAjustements({
      asset: asset.value,
      roi_pct: r.roi_pct,
      win_rate: r.win_rate,
      max_drawdown_pct: r.max_drawdown_pct,
      profit_factor: r.profit_factor,
      sharpe_ratio: r.sharpe_ratio,
      ...straddleParams.value,
    })
    straddleParams.value = {
      tp_mult_1: suggestion.tp_mult_1,
      tp_mult_2: suggestion.tp_mult_2,
      tp_mult_3: suggestion.tp_mult_3,
      sl_mult: suggestion.sl_mult,
      seuil_atr: suggestion.seuil_atr,
    }
    suggestionLlm.value = suggestion.raison
  } catch (e: unknown) {
    alerteStore.afficherErreur(`IA indisponible: ${(e as Error).message}`)
  } finally {
    chargementLlm.value = false
  }
}

function afficherCourbe() {
  if (!equityChart.value || !resultats.value) return
  chart?.remove()
  chart = createChart(equityChart.value, {
    layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
    grid: { vertLines: { color: '#1f2937' }, horzLines: { color: '#1f2937' } },
    timeScale: { timeVisible: true, secondsVisible: false, tickMarkFormatter: tickMarkFormatterEquity },
    width: equityChart.value.clientWidth, height: 256,
  })
  const series = chart.addAreaSeries({
    lineColor: resultats.value.roi_pct >= 0 ? '#10b981' : '#ef4444',
    topColor: resultats.value.roi_pct >= 0 ? '#10b98133' : '#ef444433',
    bottomColor: 'transparent',
  })
  const capitalInitialSerie = chart.addLineSeries({
    color: '#3b82f6',
    lineWidth: 1,
    lineStyle: 2,
    lastValueVisible: false,
    priceLineVisible: false,
  })
  const pointsReels = resultats.value.equity_curve?.map((point) => ({
    time: point.timestamp as unknown as import('lightweight-charts').Time,
    value: point.capital,
  }))
  const n = Math.max(resultats.value.total_trades, 10)
  const pointsFallback = Array.from({ length: n }, (_, i) => ({
    time: (Math.floor(Date.now() / 1000) - (n - i) * 86400) as unknown as import('lightweight-charts').Time,
    value: resultats.value!.capital_initial + (resultats.value!.profit_net * i) / (n - 1),
  }))
  const pts = pointsReels && pointsReels.length >= 2 ? pointsReels : pointsFallback
  series.setData(pts)
  const debut = pts[0]?.time
  const fin = pts[pts.length - 1]?.time
  if (debut && fin) {
    capitalInitialSerie.setData([
      { time: debut, value: resultats.value.capital_initial },
      { time: fin, value: resultats.value.capital_initial },
    ])
  }
  chart.timeScale().fitContent()
}

watch(equityChart, (el) => {
  roEquity?.disconnect()
  if (!el) return
  if (resultats.value) afficherCourbe()
  roEquity = new ResizeObserver(() => chart?.applyOptions({ width: el.clientWidth }))
  roEquity.observe(el)
})
onMounted(() => {
  assetsStore.chargerAssets()
  if (modeCreneau.value) { asset.value = modeCreneau.value.asset; lancerBacktest() }
})
onUnmounted(() => { roEquity?.disconnect() })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.btn-primary { @apply bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-semibold px-4 py-2 rounded-lg transition-all; }
.label { @apply text-xs text-gray-400 font-medium mb-1; }
</style>

