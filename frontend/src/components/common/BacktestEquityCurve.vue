<template>
  <div class="glass-card rounded-xl border border-white/10 bg-white/5 p-3">
    <h3 class="text-xs font-semibold text-gray-300 mb-2">Courbe d'équité</h3>
    <div class="relative h-[72px] w-full">
      <svg
        v-if="points.length > 1"
        class="w-full h-full overflow-visible"
        preserveAspectRatio="none"
        viewBox="0 0 100 72"
      >
        <defs>
          <linearGradient :id="gradId" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" :stop-color="estPositif ? '#10b981' : '#ef4444'" stop-opacity="0.3" />
            <stop offset="100%" :stop-color="estPositif ? '#10b981' : '#ef4444'" stop-opacity="0.02" />
          </linearGradient>
        </defs>
        <polygon :points="areaPoints" :fill="`url(#${gradId})`" />
        <polyline
          :points="polylinePoints"
          fill="none"
          :stroke="estPositif ? '#10b981' : '#ef4444'"
          stroke-width="1.5"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
        <!-- Zones de survol invisibles par trade (tooltip au hover) -->
        <circle
          v-for="(pt, i) in tradePoints"
          :key="i"
          :cx="pt[0]"
          :cy="pt[1]"
          r="3"
          fill="transparent"
          class="cursor-crosshair"
          @mouseenter="onHover(i)"
          @mouseleave="tradeActif = null"
        />
        <!-- Point final mis en évidence -->
        <circle :cx="lastX" :cy="lastY" r="2.5" :fill="estPositif ? '#10b981' : '#ef4444'" />
      </svg>
      <p v-else class="text-xs text-gray-600 italic flex items-center h-full justify-center">
        Pas de données
      </p>

      <!-- Tooltip trade actif -->
      <Transition name="fade">
        <div
          v-if="tradeActif"
          class="absolute z-50 pointer-events-none"
          :style="{ left: tradeActif.xPct + '%', top: tradeActif.yPct + '%', transform: 'translate(-50%, calc(-100% - 8px))' }"
        >
          <div class="rounded-lg border border-white/20 bg-[#0a0e27]/95 backdrop-blur-md p-2 shadow-xl min-w-[130px]">
            <div class="flex items-center gap-1.5 mb-1.5">
              <span class="text-[10px] font-semibold" :class="couleurTexte(trades[tradeActif.index])">
                {{ emojiResultat(trades[tradeActif.index]) }}
              </span>
              <span class="text-[10px] text-gray-300 ml-auto">
                {{ trades[tradeActif.index].direction }} · #{{ tradeActif.index + 1 }}
              </span>
            </div>
            <div class="space-y-0.5 text-[10px] text-gray-400">
              <div class="flex justify-between gap-3">
                <span>R</span>
                <span :class="couleurTexte(trades[tradeActif.index])" class="font-semibold">
                  {{ trades[tradeActif.index].pnl_r >= 0 ? '+' : '' }}{{ trades[tradeActif.index].pnl_r.toFixed(2) }}R
                </span>
              </div>
              <div class="flex justify-between gap-3">
                <span>P&L</span>
                <span :class="couleurTexte(trades[tradeActif.index])" class="font-semibold">
                  {{ trades[tradeActif.index].pnl_usd >= 0 ? '+' : '' }}{{ formatUsd(trades[tradeActif.index].pnl_usd) }}
                </span>
              </div>
              <div class="flex justify-between gap-3">
                <span>Capital</span>
                <span class="text-white">{{ formatUsd(equityCurve[tradeActif.index]) }}</span>
              </div>
              <div class="text-gray-500 text-[9px] pt-0.5 border-t border-white/10 mt-0.5">
                {{ formatDate(trades[tradeActif.index].ouvert_a) }}
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
    <!-- Annotations min/max -->
    <div class="flex justify-between text-[10px] text-gray-500 mt-1">
      <span>{{ formatUsd(capitalMin) }}</span>
      <span :class="estPositif ? 'text-emerald-400' : 'text-red-400'">
        {{ estPositif ? '▲' : '▼' }} {{ formatUsd(capitalFinal) }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { TradeBacktest } from '@/services/api.backtest'

const props = defineProps<{
  equityCurve: number[]
  capitalInitial: number
  trades: TradeBacktest[]
}>()

const W = 100
const H = 72
const PAD = 4

// ID unique par instance pour éviter les collisions de gradient entre Straddle et SMC
const gradId = 'eqGrad_' + Math.random().toString(36).slice(2, 7)

const capitalMin = computed(() => Math.min(props.capitalInitial, ...props.equityCurve))
const capitalFinal = computed(() => props.equityCurve.at(-1) ?? props.capitalInitial)
const estPositif = computed(() => capitalFinal.value >= props.capitalInitial)

const allValues = computed(() => [props.capitalInitial, ...props.equityCurve])

const points = computed((): Array<[number, number]> => {
  const vals = allValues.value
  if (vals.length < 2) return []
  const min = Math.min(...vals)
  const max = Math.max(...vals)
  const range = max - min || 1
  return vals.map((v, i) => [
    PAD + (i / (vals.length - 1)) * (W - 2 * PAD),
    PAD + (1 - (v - min) / range) * (H - 2 * PAD),
  ])
})

// On exclut le point 0 (capital initial = pas de trade associé)
const tradePoints = computed(() => points.value.slice(1))

const polylinePoints = computed(() => points.value.map(([x, y]) => `${x},${y}`).join(' '))

const areaPoints = computed(() => {
  if (!points.value.length) return ''
  const first = points.value[0]
  const last = points.value[points.value.length - 1]
  return [`${first[0]},${H}`, ...points.value.map(([x, y]) => `${x},${y}`), `${last[0]},${H}`].join(' ')
})

const lastX = computed(() => points.value.at(-1)?.[0] ?? 0)
const lastY = computed(() => points.value.at(-1)?.[1] ?? 0)

// ── Interactions ──────────────────────────────────────────────────────────────

const tradeActif = ref<{ index: number; xPct: number; yPct: number } | null>(null)

function onHover(i: number) {
  const pt = tradePoints.value[i]
  if (!pt) return
  tradeActif.value = { index: i, xPct: (pt[0] / W) * 100, yPct: (pt[1] / H) * 100 }
}

function couleurTexte(t?: TradeBacktest): string {
  if (!t) return 'text-gray-400'
  if (t.resultat === 'StopLoss') return 'text-red-400'
  if (t.resultat === 'NonFerme') return 'text-gray-400'
  return 'text-emerald-400'
}

function emojiResultat(t?: TradeBacktest): string {
  if (!t) return '—'
  const map: Record<string, string> = { Tp1: '✅ TP1', Tp2: '✅ TP2', Tp3: '✅ TP3', StopLoss: '❌ SL', NonFerme: '⏸ Ouvert' }
  return map[t.resultat] ?? t.resultat
}

function formatUsd(v: number): string {
  return '$' + v.toLocaleString('fr-FR', { maximumFractionDigits: 0 })
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' })
}
</script>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.1s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
