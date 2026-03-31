<template>
  <div class="flex flex-col gap-5">
    <!-- En-tête -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold text-white">🧪 Backtest sur un créneau horaire</h1>
        <p class="text-sm text-gray-400 mt-1">Valide statistiquement les créneaux identifiés par le LLM.</p>
      </div>
      <RouterLink to="/straddle" class="text-yellow-400 text-sm hover:underline">← Retour créneaux</RouterLink>
    </div>

    <!-- Paramètres -->
    <div class="glass-card p-4 flex flex-wrap gap-4 items-end">
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Asset</label>
        <select v-model="params.asset" class="glass-select">
          <option v-for="a in assetsDisponibles" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Heure début UTC</label>
        <input v-model="params.heure_debut" type="text" placeholder="14:00" class="glass-input" />
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Heure fin UTC</label>
        <input v-model="params.heure_fin" type="text" placeholder="16:00" class="glass-input" />
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Jour (optionnel)</label>
        <select v-model="params.jour_semaine" class="glass-select">
          <option :value="null">Tous les jours</option>
          <option v-for="(j, i) in JOURS" :key="i" :value="i">{{ j }}</option>
        </select>
      </div>
      <button class="btn-primary disabled:opacity-50" :disabled="chargement || !params.heure_debut" @click="lancerBacktest">
        {{ chargement ? '⏳ Backtest…' : '▶ Lancer le backtest' }}
      </button>
    </div>

    <!-- Avertissement aucun trade -->
    <div v-if="resultats && resultats.total_trades === 0" class="glass-card p-3 border-yellow-500/30 bg-yellow-900/10 flex items-center gap-3 text-sm">
      <span class="text-yellow-400">⚠</span>
      <span class="text-yellow-300 font-semibold">Aucun trade simulé</span>
      <span class="text-yellow-200/60">— Pas assez de bougies H1 en base pour ce créneau. Lancez une collecte de données.</span>
    </div>

    <!-- Métriques unifiées -->
    <div v-if="resultats" class="glass-card px-3 py-2.5 flex items-stretch gap-2">
      <div class="flex flex-1 gap-2">
        <div v-for="b in badgesG1" :key="b.label" class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="b.bg">
          <span class="text-[11px] text-gray-400 mb-1">{{ b.label }}</span>
          <span class="text-base font-bold leading-none" :class="b.color">{{ b.valeur }}</span>
        </div>
      </div>
      <div class="w-px bg-white/10 shrink-0 self-stretch" />
      <div class="flex flex-1 gap-2">
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg bg-white/5 border border-white/10">
          <span class="text-[11px] text-gray-400 mb-1">Cap. initial</span>
          <span class="text-sm font-semibold text-white">{{ formatEur(resultats.capital_initial) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.capital_final >= resultats.capital_initial ? 'bg-emerald-900/30 border-emerald-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Cap. final</span>
          <span class="text-sm font-semibold" :class="resultats.capital_final >= resultats.capital_initial ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.capital_final) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg bg-white/5 border border-white/10">
          <span class="text-[11px] text-gray-400 mb-1">Trades</span>
          <span class="text-sm font-semibold text-white">{{ resultats.total_trades }} <span class="text-emerald-400 text-xs">{{ resultats.winning_trades }}W</span> <span class="text-red-400 text-xs">{{ resultats.losing_trades }}L</span></span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.profit_factor >= 1.5 ? 'bg-emerald-900/30 border-emerald-500/20' : resultats.profit_factor >= 1.0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Profit Factor</span>
          <span class="text-sm font-semibold" :class="resultats.profit_factor >= 1.5 ? 'text-emerald-400' : resultats.profit_factor >= 1.0 ? 'text-yellow-400' : 'text-red-400'">{{ resultats.profit_factor.toFixed(2) }}</span>
        </div>
        <div class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="resultats.profit_net >= 0 ? 'bg-emerald-900/30 border-emerald-500/20' : 'bg-red-900/30 border-red-500/20'">
          <span class="text-[11px] text-gray-400 mb-1">Profit net</span>
          <span class="text-sm font-semibold" :class="resultats.profit_net >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ formatEur(resultats.profit_net) }}</span>
        </div>
      </div>
      <div class="w-px bg-white/10 shrink-0 self-stretch" />
      <div class="flex flex-1 gap-2">
        <div v-for="p in pyramidalisation" :key="p.label" class="flex flex-col items-center justify-center flex-1 px-2 py-2 rounded-lg border" :class="p.classes">
          <span class="text-[11px] text-gray-400 mb-1">{{ p.label }}</span>
          <span class="text-sm font-bold leading-none" :class="p.color">{{ p.n }} <span class="text-[11px] text-gray-500 font-normal">{{ resultats.total_trades > 0 ? `${((p.n / resultats.total_trades) * 100).toFixed(0)}%` : '' }}</span></span>
        </div>
      </div>
    </div>

    <!-- Courbe equity + Objectifs + Paramètres Straddle -->
    <div v-if="resultats" class="flex gap-4 items-stretch">
      <div class="glass-card p-5 w-1/2 min-w-0 flex flex-col">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 shrink-0">Courbe Equity</h2>
        <div ref="equityChart" class="flex-1 min-h-[200px] w-full" />
      </div>
      <div class="w-1/2 min-w-0 flex flex-col gap-4">
        <div class="glass-card p-5">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Objectifs Production</h2>
          <div class="grid grid-cols-2 gap-2">
            <ObjectifLigne label="ROI ≥ 15%" :atteint="resultats.roi_pct >= 15" :valeur="`${resultats.roi_pct.toFixed(1)}%`" />
            <ObjectifLigne label="Sharpe ≥ 1.5" :atteint="resultats.sharpe_ratio >= 1.5" :valeur="resultats.sharpe_ratio.toFixed(2)" />
            <ObjectifLigne label="Win Rate ≥ 55%" :atteint="resultats.win_rate >= 55" :valeur="`${resultats.win_rate.toFixed(1)}%`" />
            <ObjectifLigne label="Drawdown ≤ 20%" :atteint="resultats.max_drawdown_pct <= 20" :valeur="`${resultats.max_drawdown_pct.toFixed(1)}%`" />
          </div>
        </div>
        <StraddleParamsPanel
          class="flex-1"
          v-model="straddleParams"
          :has-resultats="true"
          :chargement-llm="false"
          :suggestion="null"
          @relancer="lancerBacktest"
          @params-saved="rechargerParamsEtRelancer"
        />
      </div>
    </div>

    <!-- État vide -->
    <div v-else class="glass-card p-10 text-center text-gray-500">
      <p class="text-3xl mb-2">🧪</p>
      <p class="text-sm">Configurez les paramètres et lancez le backtest.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, defineComponent, h, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { apiService } from '@/services/api.service'
import type { BacktestResults } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { useAssetsStore } from '@/stores/assets.store'
import StraddleParamsPanel from '@/components/common/StraddleParamsPanel.vue'
import { usePnLNiveaux } from '@/composables/usePnLNiveaux'
import { useEquityChart } from '@/composables/useEquityChart'

const ObjectifLigne = defineComponent({
  props: { label: String, atteint: Boolean, valeur: String },
  setup: (p) => () => h('div', { class: 'flex justify-between items-center py-1 border-b border-white/5' }, [
    h('span', { class: 'text-sm text-gray-300' }, p.label),
    h('span', { class: `text-sm font-semibold ${p.atteint ? 'text-emerald-400' : 'text-red-400'}` }, `${p.atteint ? '✓' : '✗'} ${p.valeur}`),
  ])
})

const route = useRoute()
const settingsStore = useSettingsStore()
const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const JOURS = ['Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi', 'Dimanche']

const assetsDisponibles = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['XAUUSD', 'XAGUSD', 'EURUSD', 'GBPUSD', 'USDJPY', 'BTC', 'ETH']
  return liste.filter(a => a.type !== 'crypto' || ['BTC', 'ETH'].includes(a.id)).map(a => a.id)
})

const params = ref({
  asset: (route.query.asset as string) || 'XAUUSD',
  heure_debut: (route.query.heure as string) || '14:00',
  heure_fin: '16:00',
  jour_semaine: route.query.jour ? Number(route.query.jour) : null as number | null,
})

const chargement = ref(false)
const resultats = ref<BacktestResults | null>(null)
const { pyramidalisation } = usePnLNiveaux(resultats)
const { equityChart, afficherCourbe, cleanup } = useEquityChart(resultats)
const straddleParams = ref({ atr_periode: 14, seuil_atr: 1.5, tp_mult_1: 2.0, tp_mult_2: 3.5, tp_mult_3: 5.0, sl_mult: 0.5, trailing_atr: 1.5, be_atr: 0, vente_partielle: 1 })

let debounceTimer: ReturnType<typeof setTimeout> | null = null
watch(straddleParams, () => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => lancerBacktest(), 500)
}, { deep: true })

const badgesG1 = computed(() => {
  if (!resultats.value) return []
  const r = resultats.value
  return [
    { label: 'ROI', valeur: `${r.roi_pct.toFixed(2)}%`, color: r.roi_pct >= 15 ? 'text-emerald-400' : r.roi_pct >= 0 ? 'text-yellow-400' : 'text-red-400', bg: r.roi_pct >= 15 ? 'bg-emerald-900/30 border-emerald-500/20' : r.roi_pct >= 0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20' },
    { label: 'Sharpe', valeur: r.sharpe_ratio.toFixed(2), color: r.sharpe_ratio >= 1.5 ? 'text-emerald-400' : r.sharpe_ratio >= 1.0 ? 'text-yellow-400' : 'text-red-400', bg: r.sharpe_ratio >= 1.5 ? 'bg-emerald-900/30 border-emerald-500/20' : r.sharpe_ratio >= 1.0 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20' },
    { label: 'Win Rate', valeur: `${r.win_rate.toFixed(1)}%`, color: r.win_rate >= 55 ? 'text-emerald-400' : r.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400', bg: r.win_rate >= 55 ? 'bg-emerald-900/30 border-emerald-500/20' : r.win_rate >= 45 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20' },
    { label: 'Drawdown', valeur: `${r.max_drawdown_pct.toFixed(2)}%`, color: r.max_drawdown_pct <= 20 ? 'text-emerald-400' : r.max_drawdown_pct <= 30 ? 'text-yellow-400' : 'text-red-400', bg: r.max_drawdown_pct <= 20 ? 'bg-emerald-900/30 border-emerald-500/20' : r.max_drawdown_pct <= 30 ? 'bg-yellow-900/30 border-yellow-500/20' : 'bg-red-900/30 border-red-500/20' },
  ]
})

function formatEur(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(v)
}

async function lancerBacktest() {
  if (!params.value.heure_debut) return
  chargement.value = true
  try {
    resultats.value = await apiService.runBacktest(
      params.value.asset, 'H1', settingsStore.capitalDepart, 365,
      { timing_optimal: params.value.heure_debut, jour_semaine: params.value.jour_semaine },
      straddleParams.value,
    )
    await nextTick()
    afficherCourbe()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Backtest échoué: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function rechargerParamsEtRelancer() {
  try {
    const p = await apiService.getStraddleParams()
    straddleParams.value = {
      atr_periode: p.atr_periode ?? straddleParams.value.atr_periode,
      seuil_atr: p.atr_seuil ?? straddleParams.value.seuil_atr,
      tp_mult_1: p.tp_mult_1 ?? straddleParams.value.tp_mult_1,
      tp_mult_2: p.tp_mult_2 ?? straddleParams.value.tp_mult_2,
      tp_mult_3: p.tp_mult_3 ?? straddleParams.value.tp_mult_3,
      sl_mult: p.sl_mult ?? straddleParams.value.sl_mult,
      trailing_atr: p.trailing_atr ?? straddleParams.value.trailing_atr,
    }
  } catch { /* garde les valeurs actuelles */ }
}

onMounted(() => {
  if (route.query.fin) params.value.heure_fin = route.query.fin as string
  assetsStore.chargerAssets()
  rechargerParamsEtRelancer()
})
onUnmounted(cleanup)
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white; }
.glass-input { @apply bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white w-24; }
.btn-primary { @apply px-5 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-semibold transition-all; }
</style>
