<template>
  <div class="glass-card p-3 flex flex-col gap-1">
    <div class="flex items-center justify-between">
      <span class="text-white text-xs font-semibold">📈 Courbe de capital simulé — Rockets</span>
      <span class="text-gray-500 text-xs">{{ data?.points.length ?? 0 }} trades · capital initial {{ formatEuro(data?.capital_initial ?? 10000) }}</span>
    </div>
    <div v-if="chargement" class="text-center text-gray-600 text-xs py-3">Chargement…</div>
    <div v-else-if="!data || data.points.length === 0" class="text-center text-gray-600 text-xs py-3">
      Aucune donnée
    </div>
    <svg v-else :viewBox="`0 0 ${W} ${H}`" class="w-full rounded-lg bg-white/3" style="height:130px">
      <!-- Grille Y légère -->
      <line v-for="y in yGridLines" :key="y.val"
        :x1="PX" :y1="y.coord" :x2="W - PX" :y2="y.coord"
        stroke="white" stroke-opacity="0.05" stroke-width="1" />
      <!-- Ligne zéro (capital initial) -->
      <line :x1="PX" :y1="yZero" :x2="W - PX" :y2="yZero"
        stroke="white" stroke-opacity="0.15" stroke-width="1" stroke-dasharray="4,3" />
      <!-- Label zéro -->
      <text :x="PX + 2" :y="yZero - 3" fill="rgba(255,255,255,0.3)" font-size="8">
        {{ formatEuro(data.capital_initial) }}
      </text>
      <!-- Remplissage -->
      <path :d="areaPath" :fill="couleur" fill-opacity="0.12" />
      <!-- Ligne courbe -->
      <path :d="linePath" :stroke="couleur" fill="none" stroke-width="1.5" stroke-linejoin="round" />
      <!-- Point final -->
      <circle :cx="W - PX" :cy="yDernier" r="3" :fill="couleur" />
      <!-- Axe X : ticks "Trade N" -->
      <line :x1="PX" :y1="H - PY + 2" :x2="W - PX" :y2="H - PY + 2"
        stroke="white" stroke-opacity="0.1" stroke-width="1" />
      <g v-for="tick in xTicks" :key="tick.i">
        <line :x1="tick.x" :y1="H - PY + 2" :x2="tick.x" :y2="H - PY + 5"
          stroke="white" stroke-opacity="0.2" stroke-width="1" />
        <text :x="tick.x" :y="H - 2" fill="rgba(255,255,255,0.3)" font-size="8" text-anchor="middle">
          {{ tick.label }}
        </text>
      </g>
      <!-- Label capital final sur l'axe X -->
      <text :x="W - PX" :y="H - 2" font-size="8" text-anchor="end" font-weight="bold"
        :style="{ fill: couleur }">
        {{ formatEuro(capitalNet) }}
      </text>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRocketsPerf } from '@/composables/useRocketsPerf'

const { data, chargement } = useRocketsPerf()

// Dimensions SVG — PX = padding horizontal minimal, PY = padding bas (axe X)
const W = 1000
const H = 130
const PX = 4   // quasi pleine largeur
const PY = 14  // espace pour labels axe X

const FRAIS_RT = 0.002
const fraisEstimes = computed(() => {
  if (!data.value) return 0
  return data.value.points.length * data.value.capital_initial * data.value.risk_pct * FRAIS_RT
})
const capitalNet = computed(() =>
  (data.value?.points.at(-1)?.equity_cumulee ?? data.value?.capital_initial ?? 10000) - fraisEstimes.value
)
const couleur = computed(() =>
  capitalNet.value >= (data.value?.capital_initial ?? 10000) ? '#10b981' : '#ef4444'
)

const allEquities = computed(() =>
  data.value ? [data.value.capital_initial, ...data.value.points.map(p => p.equity_cumulee)] : []
)
const minEquity = computed(() => Math.min(...allEquities.value))
const maxEquity = computed(() => Math.max(...allEquities.value))

// Zone de dessin (au-dessus de l'axe X)
const chartH = computed(() => H - PY - 4)

function yCoord(equity: number): number {
  const top = 6
  const range = maxEquity.value - minEquity.value || 1
  return top + (1 - (equity - minEquity.value) / range) * (chartH.value - top)
}
function xCoord(i: number, total: number): number {
  return PX + (i / (total - 1 || 1)) * (W - PX * 2)
}

const yZero = computed(() => yCoord(data.value?.capital_initial ?? 10000))
const yDernier = computed(() => {
  const last = data.value?.points.at(-1)?.equity_cumulee
  return last !== undefined ? yCoord(last) : chartH.value / 2
})

// Ticks axe X — nb de trades, espacement adaptatif
const xTicks = computed(() => {
  const n = data.value?.points.length ?? 0
  if (n < 2) return []
  const step = n <= 20 ? 5 : n <= 50 ? 10 : n <= 100 ? 20 : 25
  const ticks = []
  for (let i = step - 1; i < n; i += step) {
    ticks.push({ i, x: xCoord(i, n), label: `#${i + 1}` })
  }
  return ticks
})

// Grille Y : 3 lignes intermédiaires
const yGridLines = computed(() => {
  const min = minEquity.value, max = maxEquity.value
  const step = (max - min) / 4
  return [1, 2, 3].map(k => ({ val: min + step * k, coord: yCoord(min + step * k) }))
})

const linePath = computed(() => {
  if (!data.value?.points.length) return ''
  const pts = data.value.points
  return pts.map((p, i) => {
    const x = xCoord(i, pts.length)
    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${yCoord(p.equity_cumulee).toFixed(1)}`
  }).join(' ')
})

const areaPath = computed(() => {
  if (!data.value?.points.length) return ''
  const pts = data.value.points
  const line = pts.map((p, i) => `${xCoord(i, pts.length).toFixed(1)},${yCoord(p.equity_cumulee).toFixed(1)}`)
  const x0 = xCoord(0, pts.length).toFixed(1)
  const x1 = xCoord(pts.length - 1, pts.length).toFixed(1)
  const z = yZero.value.toFixed(1)
  return `M ${x0},${z} L ${line.join(' L ')} L ${x1},${z} Z`
})

function formatEuro(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 }).format(v)
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
