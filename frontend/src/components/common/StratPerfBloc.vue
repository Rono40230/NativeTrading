<template>
  <div class="glass-card p-4 flex flex-col gap-3">
    <!-- En-tête -->
    <div class="flex items-center justify-between flex-wrap gap-2">
      <span class="text-white font-semibold text-sm">{{ titre }}</span>
      <div class="flex items-center gap-3 text-xs">
        <span class="text-gray-500 flex items-center gap-1">{{ data?.nb_trades_saisis ?? 0 }} trades clôturés
          <TooltipIcon>Source : feedback stratégie — trades ayant alimenté la boucle IA</TooltipIcon>
        </span>
        <button class="text-gray-600 hover:text-gray-400 transition-colors" @click="charger">↺</button>
      </div>
    </div>

    <div v-if="chargement" class="text-center text-gray-600 text-xs py-4">Chargement…</div>

    <div v-else-if="!data || data.points.length === 0" class="text-center text-gray-600 text-xs py-4">
      Aucun trade clôturé — le bloc se remplira automatiquement.
    </div>

    <template v-else>
      <!-- KPIs -->
      <div class="flex gap-4 flex-wrap">
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">Capital simulé</span>
          <span class="font-mono font-bold" :class="capitalFinal >= data.capital_initial ? 'text-emerald-400' : 'text-red-400'">
            {{ formatEuro(capitalFinal) }}
            <span class="text-xs ml-1">({{ pctTotal >= 0 ? '+' : '' }}{{ pctTotal.toFixed(1) }}%)</span>
          </span>
        </div>
        <div class="flex flex-col">
          <span class="text-gray-500 text-xs">WR</span>
          <span class="font-mono font-bold" :class="winRate >= 0.5 ? 'text-emerald-400' : 'text-red-400'">{{ Math.round(winRate * 100) }}%</span>
        </div>
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
        <line :x1="0" :y1="yZero" :x2="W" :y2="yZero" stroke="white" stroke-opacity="0.06" stroke-width="1" />
        <path :d="areaPath" :fill="couleur" fill-opacity="0.12" />
        <path :d="linePath" :stroke="couleur" fill="none" stroke-width="2" stroke-linejoin="round" />
      </svg>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { StratEquityData } from '@/composables/useStrategiesPerf'
import TooltipIcon from './TooltipIcon.vue'

const props = defineProps<{
  titre: string
  data: StratEquityData | null
  chargement: boolean
  charger: () => void
}>()

const W = 400
const H = 80
const PAD = 8

const capitalFinal = computed(() =>
  props.data?.points.at(-1)?.equity_cumulee ?? props.data?.capital_initial ?? 10000
)
const pctTotal = computed(() => {
  if (!props.data) return 0
  return (capitalFinal.value - props.data.capital_initial) / props.data.capital_initial * 100
})
const winRate = computed(() => {
  if (!props.data?.points.length) return 0
  const wins = props.data.points.filter(p => p.verdict.startsWith('tp') || p.verdict === 'gagnant').length
  return wins / props.data.points.length
})
const meilleurt = computed(() =>
  props.data?.points.reduce((m, p) => Math.max(m, p.pnl_r), 0) ?? 0
)
const pireR = computed(() =>
  props.data?.points.reduce((m, p) => Math.min(m, p.pnl_r), 0) ?? 0
)
const couleur = computed(() =>
  capitalFinal.value >= (props.data?.capital_initial ?? 10000) ? '#10b981' : '#ef4444'
)

function yCoord(equity: number, minE: number, maxE: number): number {
  const range = maxE - minE || 1
  return PAD + (1 - (equity - minE) / range) * (H - PAD * 2)
}

const yZero = computed(() => {
  if (!props.data?.points.length) return H / 2
  const vals = props.data.points.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, props.data.capital_initial)
  const maxE = Math.max(...vals, props.data.capital_initial)
  return yCoord(props.data.capital_initial, minE, maxE)
})

const linePath = computed(() => {
  if (!props.data?.points.length) return ''
  const pts = props.data.points
  const vals = pts.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, props.data.capital_initial)
  const maxE = Math.max(...vals, props.data.capital_initial)
  return pts.map((p, i) => {
    const x = PAD + (i / (pts.length - 1 || 1)) * (W - PAD * 2)
    const y = yCoord(p.equity_cumulee, minE, maxE)
    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`
  }).join(' ')
})

const areaPath = computed(() => {
  if (!props.data?.points.length) return ''
  const pts = props.data.points
  const vals = pts.map(p => p.equity_cumulee)
  const minE = Math.min(...vals, props.data.capital_initial)
  const maxE = Math.max(...vals, props.data.capital_initial)
  const line = pts.map((p, i) => {
    const x = PAD + (i / (pts.length - 1 || 1)) * (W - PAD * 2)
    const y = yCoord(p.equity_cumulee, minE, maxE)
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  const xFirst = PAD
  const xLast = PAD + (W - PAD * 2)
  return `M ${xFirst},${yZero.value} L ${line.join(' L ')} L ${xLast},${yZero.value} Z`
})

function formatEuro(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 }).format(v)
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
