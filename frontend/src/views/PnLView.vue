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

    <!-- KPIs -->
    <div v-if="resultats" class="grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">ROI <TooltipInfo texte="Retour sur investissement sur la période simulée." :niveaux="niveaux?.roi" /></p>
        <p class="text-2xl font-bold" :class="resultats.roi_pct >= 0 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.roi_pct.toFixed(2) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Sharpe <TooltipInfo texte="Rapport rendement / risque ajusté (annualisé)." :niveaux="niveaux?.sharpe" /></p>
        <p class="text-2xl font-bold" :class="resultats.sharpe_ratio >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.sharpe_ratio.toFixed(2) }}
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Win Rate <TooltipInfo texte="Pourcentage de trades clôturés avec un gain positif." :niveaux="niveaux?.winRate" /></p>
        <p class="text-2xl font-bold" :class="resultats.win_rate >= 55 ? 'text-emerald-400' : 'text-yellow-400'">
          {{ resultats.win_rate.toFixed(1) }}%
        </p>
      </div>
      <div class="glass-card p-4 text-center">
        <p class="label flex items-center justify-center">Max Drawdown <TooltipInfo texte="Perte maximale depuis un pic de portefeuille. Au-delà de 20%, le trading s'arrête automatiquement." :niveaux="niveaux?.drawdown" /></p>
        <p class="text-2xl font-bold" :class="resultats.max_drawdown_pct <= 20 ? 'text-emerald-400' : 'text-red-400'">
          {{ resultats.max_drawdown_pct.toFixed(2) }}%
        </p>
      </div>
    </div>

    <!-- Métriques secondaires + Pyramidalisation sur la même ligne -->
    <div v-if="resultats" class="flex gap-4">
      <!-- Bloc capital -->
      <div class="glass-card p-5 flex-[5] min-w-0">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Résumé</h2>
        <div class="grid grid-cols-5 gap-2">
          <div class="text-center p-2 rounded-lg border border-white/10 bg-white/5">
            <p class="text-xs text-gray-400 mb-1">Capital initial</p>
            <p class="text-sm font-bold text-white">{{ formatEur(resultats.capital_initial) }}</p>
          </div>
          <div class="text-center p-2 rounded-lg border" :class="resultats.capital_final >= resultats.capital_initial ? 'bg-emerald-900/20 border-emerald-500/20' : 'bg-red-900/20 border-red-500/20'">
            <p class="text-xs text-gray-400 mb-1">Capital final</p>
            <p class="text-sm font-bold" :class="resultats.capital_final >= resultats.capital_initial ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.capital_final) }}</p>
          </div>
          <div class="text-center p-2 rounded-lg border border-white/10 bg-white/5">
            <p class="text-xs text-gray-400 mb-1">Trades</p>
            <p class="text-sm font-bold text-white">{{ resultats.total_trades }}<span class="text-xs text-gray-500 ml-1">({{ resultats.nb_straddles }})</span></p>
            <p class="text-xs"><span class="text-emerald-400">{{ resultats.winning_trades }}W</span> <span class="text-red-400">{{ resultats.losing_trades }}L</span></p>
          </div>
          <div class="text-center p-2 rounded-lg border" :class="resultats.profit_factor >= 1.5 ? 'bg-emerald-900/20 border-emerald-500/20' : 'bg-yellow-900/20 border-yellow-500/20'">
            <p class="text-xs text-gray-400 mb-1">Profit Factor</p>
            <p class="text-sm font-bold" :class="resultats.profit_factor >= 1.5 ? 'text-emerald-400' : 'text-yellow-400'">{{ resultats.profit_factor.toFixed(2) }}</p>
          </div>
          <div class="text-center p-2 rounded-lg border" :class="resultats.profit_net >= 0 ? 'bg-emerald-900/20 border-emerald-500/20' : 'bg-red-900/20 border-red-500/20'">
            <p class="text-xs text-gray-400 mb-1">Profit net</p>
            <p class="text-sm font-bold" :class="resultats.profit_net >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.profit_net) }}</p>
          </div>
        </div>
      </div>

      <!-- Bloc sorties pyramidales -->
      <div v-if="pyramidalisation.some(p => p.n > 0)" class="glass-card p-5 flex-[4] min-w-0">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 flex items-center">
          Sorties
          <TooltipInfo texte="Répartition des sorties pyramidales. TP3 = trade complet, TP2 = ⅔ fermés, TP1 = ⅓ seulement (BE activé), SL = perte." />
        </h2>
        <div class="grid grid-cols-4 gap-2">
          <div v-for="p in pyramidalisation" :key="p.label" class="text-center p-2 rounded-lg border" :class="p.classes">
            <p class="text-xs text-gray-400 mb-1">{{ p.label }}</p>
            <p class="text-sm font-bold" :class="p.color">{{ p.n }}</p>
            <p class="text-xs text-gray-500 mt-1">{{ resultats.total_trades > 0 ? ((p.n / resultats.total_trades) * 100).toFixed(0) : 0 }}%</p>
          </div>
        </div>
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

    <!-- Optimisation LLM — Paramètres Straddle -->
    <StraddleParamsPanel
      v-model="straddleParams"
      :has-resultats="!!resultats"
      :chargement-llm="chargementLlm"
      :suggestion="suggestionLlm"
      @optimiser="demanderOptimisation"
      @relancer="lancerBacktest"
    />

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
import { useBacktestDuree } from '@/composables/useBacktestDuree'
import { useCreneauParams } from '@/composables/useCreneauParams'
import { usePnLNiveaux } from '@/composables/usePnLNiveaux'
import StraddleParamsPanel from '@/components/common/StraddleParamsPanel.vue'

// Inline sub-component pour les lignes d'objectif
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

// ── Paramètres Straddle éditables (LLM ou manuels) ────────────────────────
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
    // Mise à jour du tableau de volatilité si on vient d'un créneau identifié
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
    width: equityChart.value.clientWidth, height: 256,
  })
  const series = chart.addAreaSeries({
    lineColor: resultats.value.roi_pct >= 0 ? '#10b981' : '#ef4444',
    topColor: resultats.value.roi_pct >= 0 ? '#10b98133' : '#ef444433',
    bottomColor: 'transparent',
  })
  const n = Math.max(resultats.value.total_trades, 10)
  const pts = Array.from({ length: n }, (_, i) => ({
    time: (Math.floor(Date.now() / 1000) - (n - i) * 86400) as unknown as import('lightweight-charts').Time,
    value: resultats.value!.capital_initial + (resultats.value!.profit_net * i) / (n - 1),
  }))
  series.setData(pts)
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

