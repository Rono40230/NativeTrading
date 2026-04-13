<template>
  <!-- Barre résumé collapsed -->
  <div
    class="glass-bar px-4 py-2.5 flex items-center gap-3 cursor-pointer hover:bg-white/5 transition-colors"
    @click="modalOuverte = true"
  >
    <p class="text-[11px] font-semibold uppercase tracking-widest text-white shrink-0">📊 SMC</p>
    <span class="text-[10px] text-gray-500 shrink-0">{{ assets.length }} assets</span>
    <div class="flex items-center gap-3 flex-1 min-w-0 overflow-hidden">
      <template v-for="a in topAssets" :key="a.id">
        <span class="text-[10px] font-mono shrink-0" :class="(a.variation ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
          {{ a.id }} {{ a.variation !== null ? ((a.variation >= 0 ? '+' : '') + a.variation.toFixed(2) + '%') : '—' }}
        </span>
      </template>
    </div>
    <div v-if="chargement" class="h-1.5 w-1.5 animate-pulse rounded-full bg-blue-500 shrink-0" />
    <span class="text-[10px] text-gray-600 shrink-0">▸</span>
  </div>

  <!-- Contenu complet en modal -->
  <ModalSurveillance :visible="modalOuverte" titre="📊 Surveillance des Assets — Stratégie SMC" @close="modalOuverte = false">
    <div v-if="chargement && assets.length === 0" class="grid grid-cols-8 gap-2 mb-4">
      <div v-for="n in 15" :key="n" class="rounded-lg border border-white/5 bg-white/5 px-3 py-2 h-[36px] animate-pulse" />
    </div>
    <div v-else class="grid grid-cols-8 gap-2">
      <div
        v-for="a in assets"
        :key="a.id"
        class="rounded-lg border px-2.5 py-1.5 flex items-center gap-1.5 transition-colors hover:brightness-125 cursor-pointer"
        :class="classeCard(a.variation)"
        @click.stop="onCardClick($event, a.id)"
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
  </ModalSurveillance>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hoveredAsset"
        class="fixed z-[9999] w-64 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: tooltipPos.y + 'px', left: tooltipPos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @click.stop
      >
        <div class="flex items-center justify-between mb-3">
          <span class="text-base font-bold text-white">{{ hoveredAsset.id }}</span>
          <span class="text-xs text-gray-400">{{ deviseAsset(hoveredAsset.id) }}</span>
        </div>

        <div class="mb-3">
          <div class="flex items-center justify-between mb-1">
            <p class="text-[10px] text-gray-500">Tendance — {{ TF_LABELS[selectedTF] ?? selectedTF }}</p>
            <div v-if="hoveredAsset.variationsMultiTF" class="flex gap-0.5">
              <button
                v-for="item in tfItems(hoveredAsset)"
                :key="item.label"
                class="text-[9px] px-1.5 py-0.5 rounded transition-colors"
                :class="selectedTF === item.key ? 'bg-white/15 text-white' : 'text-gray-500 hover:text-gray-300'"
                @click.stop="selectedTF = item.key"
              >{{ item.label }}</button>
            </div>
          </div>
          <svg viewBox="0 0 240 50" class="w-full" style="height:48px">
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
              <text x="120" y="27" text-anchor="middle" fill="#4b5563" font-size="10">Données insuffisantes</text>
            </template>
          </svg>
        </div>

        <div class="space-y-1.5 text-[11px]">
          <div class="flex justify-between">
            <span class="text-gray-500">Prix</span>
            <span class="text-white font-mono">{{ hoveredAsset.prix !== null ? formatPrix(hoveredAsset.prix, hoveredAsset.id) : '—' }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Variation {{ TF_SHORT[selectedTF] ?? selectedTF }}</span>
            <span v-if="varSelectedTF(hoveredAsset) !== null" class="font-bold"
              :class="varSelectedTF(hoveredAsset)! >= 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ varSelectedTF(hoveredAsset)! >= 0 ? '+' : '' }}{{ varSelectedTF(hoveredAsset)!.toFixed(2) }}%
            </span>
            <span v-else class="text-gray-600">—</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Variation live</span>
            <span v-if="hoveredAsset.variation !== null" class="font-bold"
              :class="hoveredAsset.variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ hoveredAsset.variation >= 0 ? '+' : '' }}{{ hoveredAsset.variation.toFixed(2) }}%
            </span>
            <span v-else class="text-gray-600">—</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import ModalSurveillance from './ModalSurveillance.vue'

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

const modalOuverte = ref(false)
const topAssets = computed(() =>
  [...props.assets]
    .filter(a => a.variation !== null)
    .sort((a, b) => Math.abs(b.variation ?? 0) - Math.abs(a.variation ?? 0))
    .slice(0, 4)
)

const TF_LABELS: Record<string, string> = {
  h1: '1H', h4: '4H', d1: 'D1', w1: 'W1',
}
const TF_SHORT: Record<string, string> = {
  h1: '1H', h4: '4H', d1: 'D1', w1: 'W1',
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
  const W = 240, H = 48
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

function varSelectedTF(a: AssetAvecPrix): number | null {
  if (!a.variationsMultiTF) return null
  return (a.variationsMultiTF as Record<string, number | null>)[selectedTF.value] ?? null
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

function onCardClick(event: MouseEvent, id: string) {
  const asset = props.assets.find(a => a.id === id)
  if (!asset?.variationsMultiTF) return
  if (hoveredAsset.value?.id === id) { hoveredAsset.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampedX = Math.max(168, Math.min(window.innerWidth - 168, rawX))
  tooltipPos.value = { x: clampedX, y: rect.top - 8 }
  selectedTF.value = 'd1'
  hoveredAsset.value = asset
}

function fermerTooltip() { hoveredAsset.value = null }
onMounted(() => document.addEventListener('click', fermerTooltip))
onUnmounted(() => document.removeEventListener('click', fermerTooltip))
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-bar  { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
</style>
