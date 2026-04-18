<template>
  <div class="glass-card p-4 flex flex-col gap-3">
    <!-- En-tête -->
    <div class="flex items-center justify-between flex-wrap gap-2">
      <span class="text-xs uppercase font-bold text-white">&#128208; Performance SMC</span>
      <div class="flex items-center gap-3 text-xs">
        <span class="text-gray-500">
          {{ equityData?.nb_trades_saisis ?? 0 }} trades clôturés
        </span>
        <button class="text-gray-600 hover:text-gray-400 transition-colors" @click="chargerTout">↺</button>
      </div>
    </div>

    <!-- Alerte dérive -->
    <div v-if="monitoring?.derive_detectee"
      class="flex items-center gap-2 rounded-lg bg-orange-900/30 border border-orange-500/30 px-3 py-1.5 text-xs text-orange-300">
      <span>⚠️</span>
      <span class="font-semibold">Dérive LLM détectée</span>
      <span class="text-orange-400/70">— Win rate &lt; 45% sur les 20 derniers trades</span>
    </div>

    <div v-if="chargement" class="text-center text-gray-600 text-xs py-4">Chargement…</div>
    <div v-else-if="!equityData || equityData.points.length === 0"
      class="text-center text-gray-600 text-xs py-4">
      Aucun trade clôturé — le bloc se remplira automatiquement.
    </div>
    <template v-else>
      <!-- KPIs fusionnés -->
      <div class="flex gap-4 flex-wrap">
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">Capital simulé</span>
          <span class="font-mono font-bold" :class="capitalFinal >= equityData.capital_initial ? 'text-emerald-400' : 'text-red-400'">
            {{ formatEuro(capitalFinal) }}
            <span class="text-xs ml-1">({{ pctTotal >= 0 ? '+' : '' }}{{ pctTotal.toFixed(1) }}%)</span>
          </span>
        </div>
        <!-- Win Rate depuis monitoring (source de vérité) -->
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">Win Rate</span>
          <span class="font-mono font-bold"
            :class="(monitoring?.win_rate_global ?? 0) >= 0.55 ? 'text-emerald-400' : (monitoring?.win_rate_global ?? 0) >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
            {{ monitoring ? pct(monitoring.win_rate_global) : pctEquity(winRateEquity) }}
          </span>
        </div>
        <!-- Gagnants / Perdants depuis monitoring -->
        <div v-if="monitoring" class="flex flex-col">
          <span class="text-gray-500 text-xs">Gagnants</span>
          <span class="font-mono font-bold text-emerald-400">{{ monitoring.nb_gagnants }}</span>
        </div>
        <div v-if="monitoring" class="flex flex-col">
          <span class="text-gray-500 text-xs">Perdants</span>
          <span class="font-mono font-bold text-red-400">{{ monitoring.nb_perdants }}</span>
        </div>
        <!-- PnL moyen R depuis monitoring -->
        <div v-if="monitoring?.pnl_moyen_r != null" class="flex flex-col">
          <span class="text-gray-500 text-xs">PnL moy.</span>
          <span class="font-mono font-bold"
            :class="monitoring.pnl_moyen_r >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ monitoring.pnl_moyen_r.toFixed(2) }}R
          </span>
        </div>
        <!-- Meilleur / Pire R depuis equity -->
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">Meilleur</span>
          <span class="font-mono text-emerald-300 text-sm">+{{ meilleurt.toFixed(2) }}R</span>
        </div>
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">Pire</span>
          <span class="font-mono text-red-300 text-sm">{{ pireR.toFixed(2) }}R</span>
        </div>
      </div>

      <!-- Courbe SVG -->
      <svg :viewBox="`0 0 ${W} ${H}`" class="w-full rounded-lg bg-white/3" style="height:80px">
        <line :x1="0" :y1="yZero" :x2="W" :y2="yZero"
          stroke="white" stroke-opacity="0.06" stroke-width="1" />
        <path :d="areaPath" :fill="couleur" fill-opacity="0.12" />
        <path :d="linePath" :stroke="couleur" fill="none" stroke-width="2" stroke-linejoin="round" />
      </svg>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useSmcPerf } from '@/composables/useStrategiesPerf'
import { apiService } from '@/services/api.service'
import type { SmcMonitoringData } from '@/services/api.types'

const { data: equityData, chargement: chargementEquity, charger: chargerEquity } = useSmcPerf()

const monitoring = ref<SmcMonitoringData | null>(null)
const chargementMonitoring = ref(false)
const chargement = computed(() => chargementEquity.value || chargementMonitoring.value)

async function chargerMonitoring() {
  chargementMonitoring.value = true
  try {
    monitoring.value = await apiService.getSmcMonitoringML()
  } catch {
    // silence
  } finally {
    chargementMonitoring.value = false
  }
}

function chargerTout() { chargerEquity(); chargerMonitoring() }

let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => { chargerMonitoring(); timer = setInterval(chargerMonitoring, 30_000) })
onUnmounted(() => { if (timer) clearInterval(timer) })

// ── Calculs equity ────────────────────────────────────────────
const W = 400
const H = 80
const PAD = 8

const capitalFinal = computed(() =>
  equityData.value?.points.at(-1)?.equity_cumulee ?? equityData.value?.capital_initial ?? 10000
)
const pctTotal = computed(() => {
  if (!equityData.value) return 0
  return (capitalFinal.value - equityData.value.capital_initial) / equityData.value.capital_initial * 100
})
const winRateEquity = computed(() => {
  if (!equityData.value?.points.length) return 0
  const wins = equityData.value.points.filter(p => p.verdict.startsWith('tp') || p.verdict === 'gagnant').length
  return wins / equityData.value.points.length
})
const meilleurt = computed(() => equityData.value?.points.reduce((m, p) => Math.max(m, p.pnl_r), 0) ?? 0)
const pireR = computed(() => equityData.value?.points.reduce((m, p) => Math.min(m, p.pnl_r), 0) ?? 0)
const couleur = computed(() => capitalFinal.value >= (equityData.value?.capital_initial ?? 10000) ? '#10b981' : '#ef4444')

function yCoord(equity: number, minE: number, maxE: number): number {
  const range = maxE - minE || 1
  return PAD + (1 - (equity - minE) / range) * (H - PAD * 2)
}

const yZero = computed(() => {
  if (!equityData.value?.points.length) return H / 2
  const vals = equityData.value.points.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, equityData.value.capital_initial)
  const maxE = Math.max(...vals, equityData.value.capital_initial)
  return yCoord(equityData.value.capital_initial, minE, maxE)
})

const linePath = computed(() => {
  if (!equityData.value?.points.length) return ''
  const pts = equityData.value.points
  const vals = pts.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, equityData.value.capital_initial)
  const maxE = Math.max(...vals, equityData.value.capital_initial)
  return pts.map((p, i) => {
    const x = PAD + (i / (pts.length - 1 || 1)) * (W - PAD * 2)
    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${yCoord(p.equity_cumulee, minE, maxE).toFixed(1)}`
  }).join(' ')
})

const areaPath = computed(() => {
  if (!equityData.value?.points.length) return ''
  const pts = equityData.value.points
  const vals = pts.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, equityData.value.capital_initial)
  const maxE = Math.max(...vals, equityData.value.capital_initial)
  const line = pts.map((p, i) => {
    const x = PAD + (i / (pts.length - 1 || 1)) * (W - PAD * 2)
    return `${x.toFixed(1)},${yCoord(p.equity_cumulee, minE, maxE).toFixed(1)}`
  })
  return `M ${PAD},${yZero.value} L ${line.join(' L ')} L ${PAD + W - PAD * 2},${yZero.value} Z`
})

// ── Formatage ─────────────────────────────────────────────────
function formatEuro(v: number) {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 }).format(v)
}
function pct(v: number) { return (v * 100).toFixed(1) + '%' }
function pctEquity(v: number) { return Math.round(v * 100) + '%' }
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
