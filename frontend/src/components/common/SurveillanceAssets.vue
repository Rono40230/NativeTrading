<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
        📊 Surveillance ASSETS
      </p>
      <div v-if="chargement" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
    </div>

    <!-- Squelette chargement -->
    <div v-if="chargement && assets.length === 0" class="grid grid-cols-6 gap-2">
      <div
        v-for="n in 15"
        :key="n"
        class="rounded-lg border border-white/5 bg-white/5 px-3 py-2 h-[36px] animate-pulse"
      />
    </div>

    <!-- Grille 5 colonnes, scroll après 3 lignes -->
    <div v-else class="grid grid-cols-6 gap-2 overflow-y-auto scroll-zone" style="max-height: calc(3 * 44px + 2 * 8px)">
      <div
        v-for="a in assets"
        :key="a.id"
        class="rounded-lg border px-2.5 py-1.5 flex items-center gap-1.5 transition-colors hover:brightness-125 cursor-default"
        :class="classeCard(a.variation)"
        @mouseenter="onCardEnter($event, a.id)"
        @mouseleave="onCardLeave"
      >
        <span class="text-[11px] font-bold text-white truncate flex-1 min-w-0">{{ a.id }}</span>
        <span class="text-[10px] shrink-0">{{ iconeVariation(a.variation) }}</span>
        <span v-if="a.chargement" class="text-[9px] text-gray-500 animate-pulse shrink-0">…</span>
        <span v-else-if="a.prix !== null" class="text-[10px] font-semibold text-slate-200 shrink-0">{{ formatPrix(a.prix, a.id) }}</span>
        <span v-else class="text-[9px] text-gray-500 shrink-0">—</span>
        <span v-if="a.variation !== null" class="text-[9px] font-bold shrink-0" :class="a.variation >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ a.variation >= 0 ? '+' : '' }}{{ a.variation.toFixed(2) }}%</span>
        <span v-else class="text-[9px] text-gray-600 shrink-0">—</span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hoveredAsset"
        class="fixed z-[9999] w-80 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: tooltipPos.y + 'px', left: tooltipPos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @mouseenter="onTooltipEnter"
        @mouseleave="onTooltipLeave"
      >
        <div class="flex items-start justify-between mb-3">
          <div>
            <span class="text-base font-bold text-white">{{ hoveredAsset.id }}</span>
            <span class="text-xs text-gray-500 ml-1.5">{{ deviseAsset(hoveredAsset.id) }}</span>
          </div>
          <div class="text-right">
            <span class="text-base font-bold text-white">{{ hoveredAsset.prix !== null ? formatPrix(hoveredAsset.prix, hoveredAsset.id) : '—' }}</span>
            <span v-if="hoveredAsset.variation !== null" class="block text-xs font-semibold"
              :class="hoveredAsset.variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ hoveredAsset.variation >= 0 ? '+' : '' }}{{ hoveredAsset.variation.toFixed(2) }}%
            </span>
          </div>
        </div>

        <div class="mb-3">
          <p class="text-[10px] text-gray-500 mb-1">Tendance — {{ TF_LABELS[selectedTF] ?? selectedTF }}</p>
          <svg viewBox="0 0 280 60" class="w-full" style="height:56px">
            <template v-if="(hoveredAsset.clotures[selectedTF] ?? []).length >= 2">
              <polyline
                :points="sparklinePath(hoveredAsset.clotures[selectedTF])"
                fill="none"
                :stroke="varColor(hoveredAsset)"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
            </template>
            <template v-else>
              <text x="140" y="32" text-anchor="middle" fill="#4b5563" font-size="10">Données insuffisantes</text>
            </template>
          </svg>
        </div>

        <div v-if="hoveredAsset.variationsMultiTF" class="border-t border-white/10 pt-3">
          <p class="text-[10px] text-gray-500 mb-2">Variations par période</p>
          <div class="grid grid-cols-6 gap-2">
            <div
              v-for="item in tfItems(hoveredAsset)"
              :key="item.label"
              class="flex flex-col items-center rounded-md py-1.5 cursor-pointer transition-all duration-100"
              :class="selectedTF === item.key ? 'ring-1 ring-white/20 bg-white/14' : 'bg-white/6'"
              @click="selectedTF = item.key"
            >
              <span class="text-[10px] text-gray-500 leading-tight">{{ item.label }}</span>
              <span v-if="item.val !== null" class="text-xs font-bold leading-tight mt-0.5"
                :class="item.val >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ item.val >= 0 ? '+' : '' }}{{ item.val.toFixed(2) }}%
              </span>
              <span v-else class="text-xs text-gray-600 leading-tight mt-0.5">—</span>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'

type VariationsMultiTF = { h1: number | null; h4: number | null; d1: number | null; w1: number | null; m1: number | null }
type AssetAvecPrix = {
  id: string
  prix: number | null
  variation: number | null
  variationsMultiTF: VariationsMultiTF | null
  clotures: Record<string, number[]>
  chargement: boolean
}

const props = defineProps<{ assets: AssetAvecPrix[]; chargement?: boolean }>()

const TF_LABELS: Record<string, string> = {
  h1: '1H (48 bougies)', h4: '4H (30 bougies)', d1: 'D1 (32 bougies)', w1: 'W1 (20 bougies)',
}
const DEVISES: Record<string, string> = {
  BTC: 'USD', ETH: 'USD',
  XAUUSD: 'USD', XAGUSD: 'USD', EURUSD: 'USD',
  GBPJPY: 'JPY', CADJPY: 'JPY', NZDJPY: 'JPY',
  USDCAD: 'CAD', USDJPY: 'JPY', DAX: 'EUR', SP500: 'USD',
}

function deviseAsset(id: string): string { return DEVISES[id] ?? '' }

function classeCard(variation: number | null): string {
  if (variation === null) return 'border-white/10 bg-white/5'
  if (variation >= 2) return 'border-emerald-500/40 bg-emerald-500/10'
  if (variation >= 0) return 'border-emerald-500/20 bg-emerald-500/[0.05]'
  if (variation >= -2) return 'border-red-500/20 bg-red-500/[0.05]'
  return 'border-red-500/40 bg-red-500/10'
}

function iconeVariation(variation: number | null): string {
  if (variation === null) return '—'
  if (variation >= 2) return '🟢'
  if (variation >= 0) return '▲'
  if (variation >= -2) return '▼'
  return '🔴'
}

function formatPrix(prix: number, id: string): string {
  const decimales = id === 'BTC' ? 0
    : DEVISES[id] === 'JPY' ? 2
    : prix > 1000 ? 0
    : prix > 1 ? 2
    : 4
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: decimales,
    maximumFractionDigits: decimales,
  }).format(prix)
}

function sparklinePath(closes: number[]): string {
  if (closes.length < 2) return ''
  const W = 280, H = 56
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

function tfItems(a: AssetAvecPrix) {
  const m = a.variationsMultiTF!
  return [
    { label: '1H', key: 'h1', val: m.h1 },
    { label: '4H', key: 'h4', val: m.h4 },
    { label: 'D',  key: 'd1', val: m.d1 },
    { label: 'W',  key: 'w1', val: m.w1 },
    { label: '1M', key: 'd1', val: m.m1 },
  ]
}

function varColor(a: AssetAvecPrix): string {
  const val = a.variationsMultiTF ? (a.variationsMultiTF as Record<string, number | null>)[selectedTF.value] ?? 0 : 0
  return val >= 0 ? '#10b981' : '#ef4444'
}

const hoveredAsset = ref<AssetAvecPrix | null>(null)
const tooltipPos = ref({ x: 0, y: 0 })
const selectedTF = ref<string>('d1')
let leaveTimer: ReturnType<typeof setTimeout> | null = null

function onCardEnter(event: MouseEvent, id: string) {
  const asset = props.assets.find(a => a.id === id)
  if (!asset?.variationsMultiTF) return
  if (leaveTimer !== null) { clearTimeout(leaveTimer); leaveTimer = null }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampedX = Math.max(168, Math.min(window.innerWidth - 168, rawX))
  tooltipPos.value = { x: clampedX, y: rect.top - 8 }
  if (hoveredAsset.value?.id !== id) selectedTF.value = 'd1'
  hoveredAsset.value = asset
}
function onCardLeave() { leaveTimer = setTimeout(() => { hoveredAsset.value = null }, 120) }
function onTooltipEnter() { if (leaveTimer !== null) { clearTimeout(leaveTimer); leaveTimer = null } }
function onTooltipLeave() { leaveTimer = setTimeout(() => { hoveredAsset.value = null }, 120) }
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
</style>
