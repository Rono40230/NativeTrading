<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="mb-2 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3 flex-wrap">
        <p class="text-[11px] font-semibold uppercase tracking-widest text-white">🚀 Veille Rockets</p>
        <div class="flex items-center gap-3 text-[9px] font-medium">
          <span class="text-blue-400">🌀 Compression</span>
          <span class="text-yellow-400">⚡ Pré-lancement</span>
          <span class="text-emerald-400">🚀 Breakout</span>
        </div>
        <span class="text-[9px] text-gray-600">{{ labelCandidats }}</span>
      </div>
      <div class="flex items-center gap-2">
        <span v-if="erreur" class="text-[10px] text-red-400">Erreur Binance</span>
        <span v-if="chargement" class="text-[9px] text-orange-400">{{ progression }}%</span>
        <div v-if="chargement" class="h-2 w-2 animate-pulse rounded-full bg-orange-500" />
        <span v-else-if="signaux.length > 0" class="text-[9px] text-gray-600">{{ countdown }}s</span>
        <button
          v-if="signaux.length > 0"
          class="text-[10px] font-semibold text-orange-300 hover:text-orange-100 border border-orange-500/40 hover:border-orange-400/70 rounded-lg px-2.5 py-1 transition-all hover:bg-orange-500/10"
          @click="modalOuverte = true"
        >Opportunités ▸</button>
      </div>
    </div>

    <!-- Vide -->
    <div v-if="signaux.length === 0 && !chargement" class="flex items-center justify-center py-5 text-xs text-gray-500">
      Aucun signal Rocket détecté pour l'instant
    </div>

    <!-- Squelette -->
    <div v-else-if="signaux.length === 0 && chargement" class="grid grid-cols-6 gap-2">
      <div v-for="n in 10" :key="n" class="rounded-lg border border-white/5 bg-white/5 h-[36px] animate-pulse" />
    </div>

    <!-- Grille 5 colonnes, scroll 3 lignes -->
    <div v-else class="grid grid-cols-6 gap-2 overflow-y-auto scroll-zone" style="max-height: calc(3 * 44px + 2 * 8px)">
      <div
        v-for="s in signaux"
        :key="s.symbol"
        class="rounded-lg border px-2.5 py-1.5 flex items-center gap-1.5 cursor-pointer transition-colors hover:brightness-125"
        :class="classeCard(s.phase)"
        @click.stop="onCardClick($event, s)"
      >
        <span class="text-[11px] font-bold text-white truncate flex-1 min-w-0">{{ s.ticker }}</span>
        <span class="text-[10px] shrink-0">{{ icone(s.phase) }}</span>
        <span class="text-[10px] font-bold shrink-0" :class="s.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ s.change1h >= 0 ? '+' : '' }}{{ s.change1h.toFixed(2) }}%</span>
        <span class="text-[9px] shrink-0" :class="s.ratioVolume >= 2 ? 'text-orange-400 font-bold' : 'text-gray-400'">{{ s.ratioVolume.toFixed(1) }}×</span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hovered"
        class="fixed z-[9999] w-60 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @click.stop
      >
        <div class="flex items-center justify-between mb-2">
          <span class="text-sm font-bold text-white">{{ hovered.ticker }}</span>
          <span class="text-[11px]">{{ icone(hovered.phase) }} <span class="text-gray-400 text-[10px]">{{ labelPhase(hovered.phase) }}</span></span>
        </div>
        <!-- Sparkline 1h -->
        <div class="mb-3">
          <p class="text-[10px] text-gray-500 mb-1">Tendance 1h (24 bougies)</p>
          <svg viewBox="0 0 240 48" class="w-full" style="height:44px">
            <template v-if="hovered.closes.length >= 2">
              <polyline
                :points="sparklinePath(hovered.closes)"
                fill="none"
                :stroke="hovered.change1h >= 0 ? '#10b981' : '#ef4444'"
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
          <div class="flex justify-between"><span class="text-gray-500">Variation 1h</span><span :class="hovered.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ hovered.change1h >= 0 ? '+' : '' }}{{ hovered.change1h.toFixed(2) }}%</span></div>
          <div class="flex justify-between"><span class="text-gray-500">Volume spike</span><span :class="hovered.ratioVolume >= 2 ? 'text-orange-400' : 'text-gray-300'">{{ hovered.ratioVolume.toFixed(2) }}×</span></div>
          <div class="flex justify-between"><span class="text-gray-500">ATR ratio</span><span :class="hovered.atrRatio < 0.75 ? 'text-blue-400' : 'text-gray-300'">{{ hovered.atrRatio.toFixed(2) }}</span></div>
          <div class="flex justify-between"><span class="text-gray-500">RSI (14)</span><span :class="hovered.rsi > 70 ? 'text-orange-400' : hovered.rsi > 60 ? 'text-emerald-400' : 'text-gray-300'">{{ hovered.rsi.toFixed(1) }}</span></div>
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
import RocketsOpportunitesModal from '@/components/common/RocketsOpportunitesModal.vue'

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

function classeCard(phase: PhaseRocket): string {
  if (phase === 'breakout')    return 'border-emerald-500/50 bg-emerald-500/10'
  if (phase === 'prelancement') return 'border-yellow-500/40 bg-yellow-500/[0.08]'
  return 'border-blue-500/30 bg-blue-500/[0.06]'
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

const hovered = ref<SignalRocket | null>(null)
const modalOuverte = ref(false)
const pos = ref({ x: 0, y: 0 })

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

function onCardClick(event: MouseEvent, s: SignalRocket) {
  if (hovered.value?.symbol === s.symbol) { hovered.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampX = Math.max(124, Math.min(window.innerWidth - 124, rawX))
  pos.value = { x: clampX, y: rect.top - 8 }
  hovered.value = s
}

function fermerTooltip() { hovered.value = null }

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
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
.tooltip-enter-active, .tooltip-leave-active { transition: opacity 0.12s, transform 0.12s; }
.tooltip-enter-from, .tooltip-leave-to { opacity: 0; transform: translateX(-50%) translateY(calc(-100% + 6px)); }
.tooltip-enter-to, .tooltip-leave-from { opacity: 1; transform: translateX(-50%) translateY(-100%); }
</style>
