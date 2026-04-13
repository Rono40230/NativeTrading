<template>
  <div class="rocket-bar px-4 py-2.5 flex flex-col gap-1.5 cursor-pointer hover:bg-white/5 transition-colors h-full" @click="modalSurveillance = true">
    <!-- Header -->
    <div class="flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-orange-400">🚀 Rockets</p>
      <span class="text-[9px] text-gray-600">{{ countdown }}s ▸</span>
    </div>

    <!-- Chargement -->
    <template v-if="chargement">
      <span class="text-[10px] text-orange-400 animate-pulse">Scan {{ progression }}%</span>
    </template>

    <!-- Top 5 liste -->
    <template v-else-if="signaux.length > 0">
      <div v-for="s in top5" :key="s.symbol" class="flex items-center gap-1.5">
        <span class="text-[10px] font-bold text-white w-14 truncate shrink-0">{{ s.ticker }}</span>
        <span class="text-[10px] font-semibold shrink-0" :class="s.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ s.change1h >= 0 ? '+' : '' }}{{ s.change1h.toFixed(2) }}%</span>
        <span class="text-[9px] ml-auto shrink-0">{{ icone(s.phase) }}</span>
      </div>
    </template>

    <!-- Vide -->
    <template v-else>
      <span class="text-[10px] text-gray-500 italic">Aucun candidat</span>
    </template>

    <!-- Footer -->
    <div class="mt-auto shrink-0">
      <span class="text-[9px] text-gray-600">{{ signaux.length }} candidats</span>
    </div>
  </div>

  <ModalSurveillance :visible="modalSurveillance" titre="🚀 Surveillance Cryptos — Stratégie Rockets" @close="modalSurveillance = false">
    <div class="flex flex-wrap items-center justify-between gap-2 mb-4">
      <div class="flex items-center gap-3 text-[9px] font-medium">
        <span class="text-blue-400">🌀 Compression</span>
        <span class="text-yellow-400">⚡ Pré-lancement</span>
        <span class="text-emerald-400">🚀 Breakout</span>
        <span class="text-gray-600">{{ labelCandidats }}</span>
      </div>
      <div class="flex items-center gap-2">
        <span v-if="erreur" class="text-[10px] text-red-400">Erreur Binance</span>
        <button v-if="signaux.length > 0" class="text-[10px] font-semibold text-orange-300 hover:text-orange-100 border border-orange-500/40 rounded-lg px-2.5 py-1 transition-all hover:bg-orange-500/10" @click.stop="modalOuverte = true">Opportunités ▸</button>
      </div>
    </div>

    <div v-if="signaux.length === 0" class="flex items-center justify-center py-10 text-xs">
      <span :class="chargement ? 'text-orange-400 animate-pulse' : 'text-gray-500'">{{ chargement ? `Scan en cours… ${progression}%` : 'Aucun signal Rocket détecté pour l\'instant' }}</span>
    </div>
    <div v-else class="grid grid-cols-4 gap-3">
      <RocketCard v-for="s in signaux" :key="s.symbol" :s="s" @click="onCardClick($event, s)" />
    </div>
  </ModalSurveillance>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hovered"
        class="fixed z-[9999] w-60 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @click.stop
      >
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-2.5">
            <img
              :src="cryptoLogoUrl(hovered.ticker)"
              :alt="hovered.ticker"
              class="w-8 h-8 rounded-full border border-white/10 bg-white/5 object-contain"
              @error="(e) => ((e.target as HTMLImageElement).style.display = 'none')"
            />
            <div>
              <span class="text-sm font-bold text-white block">{{ hovered.ticker }}</span>
              <span class="text-[10px] text-gray-400">{{ cryptoName(hovered.ticker) }}</span>
            </div>
          </div>
          <span class="text-[11px]">{{ icone(hovered.phase) }} <span class="text-gray-400 text-[10px]">{{ labelPhase(hovered.phase) }}</span></span>
        </div>
        <!-- Sparkline multi-TF -->
        <div class="mb-3">
          <div class="flex items-center justify-between mb-1">
            <p class="text-[10px] text-gray-500">Tendance — {{ selectedTF }}</p>
            <div class="flex gap-0.5">
              <button
                v-for="tf in TF_CONFIGS"
                :key="tf.label"
                class="text-[9px] px-1.5 py-0.5 rounded transition-colors"
                :class="selectedTF === tf.label ? 'bg-white/15 text-white' : 'text-gray-500 hover:text-gray-300'"
                @click.stop="choisirTF(tf)"
              >{{ tf.label }}</button>
            </div>
          </div>
          <svg viewBox="0 0 240 48" class="w-full" style="height:44px">
            <template v-if="sparklineActive.length >= 2">
              <polyline
                :points="sparklinePath(sparklineActive)"
                fill="none"
                :stroke="couleurSparkline"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
            </template>
            <text v-else x="120" y="26" text-anchor="middle" fill="#4b5563" font-size="9">Chargement…</text>
          </svg>
        </div>
        <div class="space-y-1.5 text-[11px]">
          <div class="flex justify-between"><span class="text-gray-500">Prix</span><span class="text-white font-mono">{{ formatPrix(hovered.prix) }}$</span></div>
          <div class="flex justify-between"><span class="text-gray-500">Variation {{ selectedTF }}</span><span :class="variationTF >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ variationTF >= 0 ? '+' : '' }}{{ variationTF.toFixed(2) }}%</span></div>
          <div class="flex justify-between"><span class="text-gray-500">Volume spike</span><span :class="hovered.ratioVolume >= 2 ? 'text-orange-400' : 'text-gray-300'">{{ hovered.ratioVolume.toFixed(2) }}×</span></div>
          <div class="flex justify-between"><span class="text-gray-500">ATR ratio</span><span :class="hovered.atrRatio < 0.75 ? 'text-blue-400' : 'text-gray-300'">{{ hovered.atrRatio.toFixed(2) }}</span></div>
          <div class="flex justify-between"><span class="text-gray-500">RSI (14)</span><span :class="labelRsi(hovered.rsi).classe">{{ hovered.rsi.toFixed(1) }} — {{ labelRsi(hovered.rsi).label }}</span></div>
          <div class="border-t border-white/10 pt-1.5 mt-1.5 space-y-1">
            <div class="flex justify-between"><span class="text-gray-500">Support / SL</span><span class="text-red-400 font-mono">{{ formatPrix(hovered.support) }}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">Résistance / TP</span><span class="text-emerald-400 font-mono">{{ formatPrix(hovered.target20) }}</span></div>
          </div>
          <div class="flex justify-between border-t border-white/10 pt-1.5">
            <span class="text-gray-500">Score</span>
            <span class="font-bold" :class="hovered.score >= 70 ? 'text-orange-400' : 'text-emerald-400'">{{ hovered.score }}/100</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
  <RocketsOpportunitesModal :visible="modalOuverte" :signaux="signaux" @close="modalOuverte = false" />
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'
import { cryptoName, cryptoLogoUrl } from '@/composables/useCryptoMeta'
import RocketsOpportunitesModal from '@/components/common/RocketsOpportunitesModal.vue'
import ModalSurveillance from '@/components/common/ModalSurveillance.vue'
import RocketCard from '@/components/common/RocketCard.vue'

const props = defineProps<{
  signaux: SignalRocket[]
  totalCandidats: number
  chargement: boolean
  erreur: boolean
  progression: number
}>()  

const labelCandidats = computed(() =>
  props.chargement
    ? `Scan en cours… ${props.progression}%`
    : props.signaux.length > 0
      ? `${props.signaux.length} signaux · ${props.totalCandidats} paires analysées`
      : `${props.totalCandidats > 0 ? props.totalCandidats : '…'} paires Binance`
)

function icone(phase: PhaseRocket): string {
  if (phase === 'breakout')    return '🚀'
  if (phase === 'prelancement') return '⚡'
  return '🌀'
}

function labelPhase(phase: PhaseRocket): string {
  if (phase === 'breakout')    return 'Breakout'
  if (phase === 'prelancement') return 'Pré-lancement'
  return 'Compression'
}

function labelRsi(rsi: number): { label: string; classe: string } {
  if (rsi < 40) return { label: 'survendu',  classe: 'text-blue-400' }
  if (rsi < 50) return { label: 'neutre↓',   classe: 'text-gray-400' }
  if (rsi < 65) return { label: 'idéal ✓',   classe: 'text-emerald-400' }
  if (rsi < 75) return { label: 'momentum',  classe: 'text-yellow-400' }
  if (rsi < 85) return { label: 'chaud',     classe: 'text-orange-400' }
  return               { label: 'extrême !', classe: 'text-red-400' }
}

function classeCard(phase: PhaseRocket): string {
  if (phase === 'breakout')    return 'border-emerald-500/50 bg-emerald-500/10'
  if (phase === 'prelancement') return 'border-yellow-500/40 bg-yellow-500/[0.08]'
  return 'border-blue-500/30 bg-blue-500/[0.06]'
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

const TF_CONFIGS = [
  { label: '1H', interval: null  as string | null, limit: 0  },
  { label: '4H', interval: '5m'  as string | null, limit: 48 },
  { label: 'D1', interval: '1h'  as string | null, limit: 24 },
  { label: 'W1', interval: '4h'  as string | null, limit: 42 },
]

const hoveredSymbol = ref<string | null>(null)
const hovered = computed(() => hoveredSymbol.value ? props.signaux.find(s => s.symbol === hoveredSymbol.value) ?? null : null)
const modalOuverte = ref(false)
const modalSurveillance = ref(false)
const topSignal = computed(() =>
  props.signaux.length > 0 ? [...props.signaux].reduce((best, s) => s.score > best.score ? s : best) : null
)
const top5 = computed(() =>
  [...props.signaux].sort((a, b) => b.score - a.score).slice(0, 5)
)
const pos = ref({ x: 0, y: 0 })
const selectedTF = ref('1H')
const sparklineTF = ref<number[]>([])

const sparklineActive = computed(() =>
  selectedTF.value === '1H' ? (hovered.value?.closes ?? []) : sparklineTF.value
)
const couleurSparkline = computed(() => {
  const s = sparklineActive.value
  if (s.length < 2) return '#10b981'
  return s.at(-1)! >= s[0] ? '#10b981' : '#ef4444'
})

const variationTF = computed(() => {
  const s = sparklineActive.value
  if (s.length < 2) return hovered.value?.change1h ?? 0
  return ((s.at(-1)! - s[0]) / s[0]) * 100
})

function sparklinePath(closes: number[]): string {
  const W = 240, H = 44
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

async function fetchSparklineTF(ticker: string, interval: string, limit: number) {
  sparklineTF.value = []
  try {
    const res = await fetch(`/api/marche/klines?symbol=${ticker}&interval=${interval}&limit=${limit}`)
    if (!res.ok) return
    const data = await res.json() as unknown[][]
    sparklineTF.value = data.map(k => parseFloat(k[4] as string))
  } catch { /* silencieux */ }
}

function choisirTF(tf: { label: string; interval: string | null; limit: number }) {
  selectedTF.value = tf.label
  if (hovered.value && tf.interval) fetchSparklineTF(hovered.value.ticker, tf.interval, tf.limit)
}

function onCardClick(event: MouseEvent, s: SignalRocket) {
  if (hoveredSymbol.value === s.symbol) { hoveredSymbol.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampX = Math.max(124, Math.min(window.innerWidth - 124, rawX))
  pos.value = { x: clampX, y: rect.top - 8 }
  selectedTF.value = '1H'
  sparklineTF.value = []
  hoveredSymbol.value = s.symbol
}

function fermerTooltip() { hoveredSymbol.value = null }

// Countdown 30s : se réinitialise quand le scan se termine
const SCAN_S = 30
const countdown = ref(SCAN_S)
let tickInterval: ReturnType<typeof setInterval> | null = null
watch(() => props.chargement, (val) => { if (!val) countdown.value = SCAN_S })

onMounted(() => {
  document.addEventListener('click', fermerTooltip)
  tickInterval = setInterval(() => { if (countdown.value > 0) countdown.value-- }, 1000)
})
onUnmounted(() => {
  document.removeEventListener('click', fermerTooltip)
  if (tickInterval) clearInterval(tickInterval)
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-bar  { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.rocket-bar { @apply rounded-xl border-2 border-orange-500/50 bg-white/5 backdrop-blur-sm; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
.tooltip-enter-active, .tooltip-leave-active { transition: opacity 0.12s, transform 0.12s; }
.tooltip-enter-from, .tooltip-leave-to { opacity: 0; transform: translateX(-50%) translateY(calc(-100% + 6px)); }
.tooltip-enter-to, .tooltip-leave-from { opacity: 1; transform: translateX(-50%) translateY(-100%); }
</style>
