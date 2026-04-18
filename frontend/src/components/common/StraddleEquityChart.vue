<template>
  <div class="glass-card p-3 flex flex-col gap-1">
    <div v-if="chargement" class="flex items-center justify-center text-gray-600 text-xs italic w-full rounded-lg bg-white/3 min-h-[130px] flex-1">Chargement…</div>
    
    <svg v-else ref="svgContainer" :viewBox="`0 0 ${W} ${H}`" class="w-full flex-1 min-h-[130px] rounded-lg bg-white/3 preserve-3d" preserveAspectRatio="none" @mousemove="onMouseMove" @mouseleave="onMouseLeave">
      <!-- Grille Y légère -->
      <line v-for="y in yGridLines" :key="y.val" :x1="PX" :y1="y.coord" :x2="W - PX" :y2="y.coord" stroke="white"
        stroke-opacity="0.05" stroke-width="1" />
      <!-- Ligne zéro (capital initial) -->
      <line :x1="PX" :y1="yZero" :x2="W - PX" :y2="yZero" stroke="white" stroke-opacity="0.15" stroke-width="1"
        stroke-dasharray="4,3" />
      <!-- Label zéro -->
      <text :x="PX + 2" :y="yZero - 3" fill="rgba(255,255,255,0.3)" font-size="8">
        {{ formatEuro(data?.capital_initial ?? 10000) }}
      </text>
      <!-- Remplissage -->
      <path :d="areaPath" :fill="couleur" fill-opacity="0.12" />
      <!-- Ligne de Moyenne Mobile (P=50) -->
      <path v-if="smaPath" :d="smaPath" stroke="#fbbf24" fill="none" stroke-width="0.6" stroke-opacity="0.8" stroke-linejoin="round" />
      <!-- Ligne courbe -->
      <path :d="linePath" :stroke="couleur" fill="none" stroke-width="1.5" stroke-linejoin="round" />
      <!-- Point final -->
      <circle :cx="W - PX" :cy="yDernier" r="3" :fill="couleur" />
      <!-- Axe X : ticks "Trade N" -->
      <line :x1="PX" :y1="H - PY + 2" :x2="W - PX" :y2="H - PY + 2" stroke="white" stroke-opacity="0.1"
        stroke-width="1" />
      <g v-for="tick in xTicks" :key="tick.i">
        <line :x1="tick.x" :y1="H - PY + 2" :x2="tick.x" :y2="H - PY + 5" stroke="white" stroke-opacity="0.2"
          stroke-width="1" />
        <text :x="tick.x" :y="H - 5" fill="rgba(255,255,255,0.3)" font-size="8" text-anchor="middle">
          {{ tick.label }}
        </text>
      </g>
      <!-- Label capital final sur l'axe X -->
      <text :x="W - PX" :y="H - 5" font-size="8" text-anchor="end" font-weight="bold" :style="{ fill: couleur }">
        {{ formatEuro(capitalNet) }}
      </text>

      <!-- Tooltip Interactif (Hover) -->
      <g v-if="hoveredPointInfo" class="transition-opacity duration-200">
        <!-- Ligne verticale -->
        <line :x1="hoveredPointInfo.x" :y1="0" :x2="hoveredPointInfo.x" :y2="H - PY" stroke="white" stroke-opacity="0.3" stroke-width="1" stroke-dasharray="2,2" />
        
        <!-- Point sur la courbe -->
        <circle :cx="hoveredPointInfo.x" :cy="hoveredPointInfo.y" r="3.5" :fill="hoveredPointInfo.isGagnant ? '#10b981' : '#ef4444'" stroke="#fff" stroke-width="1.5" />
        
        <!-- Bloc texte flottant -->
        <g :transform="`translate(${hoveredPointInfo.x > W / 2 ? hoveredPointInfo.x - 70 : hoveredPointInfo.x + 10}, ${Math.max(10, Math.min(H - PY - 40, hoveredPointInfo.y - 15))})`">
          <rect x="0" y="0" width="70" height="34" rx="4" fill="#0f172a" fill-opacity="0.9" stroke="rgba(255,255,255,0.2)" stroke-width="1" />
          <text x="35" y="10" font-size="6" font-family="monospace" fill="rgba(255,255,255,0.6)" text-anchor="middle">
            {{ hoveredPointInfo.date }}
          </text>
          <text x="35" y="19" font-size="7" font-weight="bold" fill="white" text-anchor="middle">
            {{ formatEuro(hoveredPointInfo.val) }}
          </text>
          <text x="35" y="28" font-size="6" font-weight="bold" :fill="hoveredPointInfo.isGagnant ? '#10b981' : '#ef4444'" text-anchor="middle">
            {{ hoveredPointInfo.pnl }} (Trade #{{ hoveredPointInfo.tradeId }})
          </text>
        </g>
      </g>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useStraddlePerf } from '@/composables/useStrategiesPerf'

const { data, chargement } = useStraddlePerf()

const svgContainer = ref<SVGSVGElement | null>(null)
const hoverIndex = ref<number | null>(null)

function onMouseMove(e: MouseEvent) {
  if (!svgContainer.value || !data.value?.points?.length) return
  const len = data.value.points.length
  if (len < 2) return
  
  const rect = svgContainer.value.getBoundingClientRect()
  
  // Projection des coordonnées souris sur le repère de la viewBox (1000px de large)
  const ratioX = (e.clientX - rect.left) / rect.width
  const viewBoxX = ratioX * W
  
  // Interpolation inverse de la formule xCoord
  const pw = W - PX * 2
  let idx = Math.round(((viewBoxX - PX) / pw) * (len - 1))
  idx = Math.max(0, Math.min(len - 1, idx))
  
  hoverIndex.value = idx
}

function onMouseLeave() {
  hoverIndex.value = null
}

const hoveredPointInfo = computed(() => {
  if (hoverIndex.value === null || !data.value?.points?.length) return null
  const pt = data.value.points[hoverIndex.value]
  if (!pt) return null
  
  const dateStr = new Intl.DateTimeFormat('fr-FR', {
    day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit'
  }).format(new Date(pt.ferme_le * 1000))
  
  return {
    x: xCoord(hoverIndex.value, data.value.points.length),
    y: yCoord(pt.equity_cumulee),
    val: pt.equity_cumulee,
    tradeId: hoverIndex.value + 1,
    date: dateStr,
    pnl: pt.pnl_r >= 0 ? `+${pt.pnl_r.toFixed(1)}R` : `${pt.pnl_r.toFixed(1)}R`,
    isGagnant: pt.pnl_r >= 0
  }
})

// Dimensions SVG — PX = padding horizontal minimal, PY = padding bas (axe X)
const W = 1000
const H = 130
const PX = 4   // quasi pleine largeur
const PY = 18  // espace pour labels axe X

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
  data.value && data.value.points.length > 0 ? [data.value.capital_initial, ...data.value.points.map(p => p.equity_cumulee)] : [data.value?.capital_initial ?? 10000, data.value?.capital_initial ?? 10000]
)
const minEquity = computed(() => { const v = Math.min(...allEquities.value); const maxV = Math.max(...allEquities.value); return v === maxV ? v - 0.01 : v })
const maxEquity = computed(() => { const v = Math.min(...allEquities.value); const maxV = Math.max(...allEquities.value); return v === maxV ? maxV + 0.01 : maxV })

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
  const last = (data.value && data.value.points.length > 0) ? data.value.points.at(-1)?.equity_cumulee : (data.value?.capital_initial ?? 10000)
  return last !== undefined ? yCoord(last) : chartH.value / 2
})

// Ticks axe X — nb de trades, espacement adaptatif
const xTicks = computed(() => {
  const n = (!data.value || data.value.points.length === 0) ? 2 : data.value.points.length
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
  const pts = !data.value || data.value.points.length === 0 ? [{ equity_cumulee: data.value?.capital_initial ?? 10000 }, { equity_cumulee: data.value?.capital_initial ?? 10000 }] : data.value.points.length === 1 ? [{ equity_cumulee: data.value.capital_initial }, data.value.points[0]] : data.value.points
  return pts.map((p, i) => {
    const x = xCoord(i, pts.length)
    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${yCoord(p.equity_cumulee).toFixed(1)}`
  }).join(' ')
})

const smaPath = computed(() => {
  const pts = !data.value || data.value.points.length === 0 ? [] : data.value.points.length === 1 ? [data.value.points[0]] : data.value.points
  if (pts.length < 5) return '' // Pas assez de points pour la moyenne
  const P = 50 // Période de la moyenne (50 trades)
  const smaPoints = []
  
  for (let i = 0; i < pts.length; i++) {
    const slice = pts.slice(Math.max(0, i - P + 1), i + 1)
    const avg = slice.reduce((sum, p) => sum + p.equity_cumulee, 0) / slice.length
    smaPoints.push(avg)
  }
  
  return smaPoints.map((val, i) => {
    const x = xCoord(i, pts.length)
    return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${yCoord(val).toFixed(1)}`
  }).join(' ')
})

const areaPath = computed(() => {
  const pts = !data.value || data.value.points.length === 0 ? [{ equity_cumulee: data.value?.capital_initial ?? 10000 }, { equity_cumulee: data.value?.capital_initial ?? 10000 }] : data.value.points.length === 1 ? [{ equity_cumulee: data.value.capital_initial }, data.value.points[0]] : data.value.points
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
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
