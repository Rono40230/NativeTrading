<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
        ⚡ Alertes Cryptos — Top 20 hausse 24h
      </p>
      <div class="flex items-center gap-2">
        <span v-if="erreur" class="text-[10px] text-red-400">Binance indisponible</span>
        <div v-if="chargement" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
        <span v-else-if="top20.length > 0" class="text-[9px] text-gray-600">60s</span>
        <button
          v-if="top20.length > 0"
          class="text-[10px] font-semibold text-blue-300 hover:text-blue-100 border border-blue-500/40 hover:border-blue-400/70 rounded-lg px-2.5 py-1 transition-all hover:bg-blue-500/10"
          @click="modalOuverte = true"
        >Opportunités ▸</button>
      </div>
    </div>

    <!-- État vide -->
    <div v-if="top20.length === 0 && !chargement" class="flex items-center justify-center py-6 text-xs text-gray-500">
      Aucun signal ≥ 10% pour l'instant
    </div>

    <!-- Squelette chargement -->
    <div v-else-if="top20.length === 0 && chargement" class="grid grid-cols-5 gap-2">
      <div
        v-for="n in 15"
        :key="n"
        class="rounded-lg border border-white/5 bg-white/5 px-3 py-2 h-[60px] animate-pulse"
      />
    </div>

    <!-- Grille 5 colonnes, scroll après 3 lignes, triée score décroissant -->
    <div v-else class="grid grid-cols-5 gap-2 overflow-y-auto scroll-zone" style="max-height: calc(3 * 68px + 2 * 8px)">
      <div
        v-for="c in top20ParScore"
        :key="c.symbol"
        class="rounded-lg border px-3 py-2 flex flex-col gap-0.5 transition-colors hover:brightness-125 cursor-default"
        :class="classeCard(c.badge)"
        @mouseenter="onCardEnter($event, c)"
        @mouseleave="onCardLeave"
      >
        <div class="flex items-center justify-between">
          <span class="text-xs font-bold text-white truncate mr-1">{{ c.ticker }}</span>
          <span class="text-[11px] shrink-0">{{ icone(c.badge) }}</span>
        </div>
        <span class="text-[11px] font-bold text-emerald-400">+{{ c.change24h.toFixed(2) }}%</span>
        <span class="text-[9px] text-gray-500">{{ formatVolume(c.volume24h) }}</span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hovered"
        class="fixed z-[9999] w-64 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%)', background: '#0b0f28' }"
        @mouseenter="onTipEnter"
        @mouseleave="onTipLeave"
      >
        <div class="flex items-center justify-between mb-3">
          <span class="text-base font-bold text-white">{{ hovered.ticker }}</span>
          <span class="text-[11px]">{{ icone(hovered.badge) }}</span>
        </div>
        <div class="mb-3">
          <p class="text-[10px] text-gray-500 mb-1">Tendance 24h (1h)</p>
          <svg viewBox="0 0 240 50" class="w-full" style="height:48px">
            <template v-if="sparkline.length >= 2">
              <polyline
                :points="sparklinePath(sparkline)"
                fill="none"
                :stroke="hovered!.change24h >= 0 ? '#10b981' : '#ef4444'"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
            </template>
            <text v-else x="120" y="27" text-anchor="middle" fill="#4b5563" font-size="9">Chargement…</text>
          </svg>
        </div>
        <div class="space-y-1.5 text-[11px]">
          <div class="flex justify-between">
            <span class="text-gray-500">Prix</span>
            <span class="text-white font-mono">{{ formatPrix(hovered.prix) }}$</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Variation 24h</span>
            <span class="font-bold text-emerald-400">+{{ hovered.change24h.toFixed(2) }}%</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Volume 24h</span>
            <span class="text-white">{{ formatVolume(hovered.volume24h) }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Trades 24h</span>
            <span class="text-white">{{ hovered.nbTrades.toLocaleString('fr-FR') }}</span>
          </div>
          <div class="flex justify-between border-t border-white/10 pt-1.5 mt-1.5">
            <span class="text-gray-500">Score</span>
            <span class="font-bold" :class="classScore(hovered.score)">{{ hovered.score.toFixed(1) }}/100</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Signal</span>
            <span class="font-semibold text-white">{{ labelBadge(hovered.badge) }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
  <CryptosOpportunitesModal :visible="modalOuverte" :top20="top20" @close="modalOuverte = false" />
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { CryptoAlert, BadgeNiveau } from '@/composables/useCryptosAlert'
import CryptosOpportunitesModal from '@/components/common/CryptosOpportunitesModal.vue'

const props = defineProps<{
  top20: CryptoAlert[]
  chargement: boolean
  erreur: boolean
}>()

function icone(badge: BadgeNiveau): string {
  if (badge === 'explosion') return '🚀'
  if (badge === 'breakout') return '⚡'
  if (badge === 'chaud') return '🔥'
  return '📈'
}

function classeCard(badge: BadgeNiveau): string {
  if (badge === 'explosion') return 'border-red-500/50 bg-red-500/10'
  if (badge === 'breakout') return 'border-orange-500/40 bg-orange-500/10'
  if (badge === 'chaud') return 'border-yellow-500/30 bg-yellow-500/[0.08]'
  return 'border-emerald-500/20 bg-emerald-500/[0.05]'
}

function formatVolume(v: number): string {
  if (v >= 1_000_000_000) return `${(v / 1_000_000_000).toFixed(1)}B$`
  if (v >= 1_000_000)     return `${(v / 1_000_000).toFixed(1)}M$`
  if (v >= 1_000)         return `${(v / 1_000).toFixed(0)}K$`
  return `${v.toFixed(0)}$`
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

function classScore(s: number): string {
  if (s >= 70) return 'text-red-400'
  if (s >= 50) return 'text-orange-400'
  return 'text-emerald-400'
}

function labelBadge(badge: BadgeNiveau): string {
  if (badge === 'explosion') return '🚀 Explosion'
  if (badge === 'breakout') return '⚡ Breakout'
  if (badge === 'chaud') return '🔥 Chaud'
  return '📈 Haussier'
}

const modalOuverte = ref(false)

/** Tri réactif côté composant : score décroissant, mise à jour automatique */
const top20ParScore = computed(() =>
  [...props.top20].sort((a, b) => b.change24h - a.change24h)
)

const hovered = ref<CryptoAlert | null>(null)
const pos = ref({ x: 0, y: 0 })
const sparkline = ref<number[]>([])
let leaveTimer: ReturnType<typeof setTimeout> | null = null

function sparklinePath(closes: number[]): string {
  const W = 240, H = 48
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

async function fetchSparkline(ticker: string) {
  sparkline.value = []
  try {
    const res = await fetch(`https://api.binance.com/api/v3/klines?symbol=${ticker}USDT&interval=1h&limit=24`)
    if (!res.ok) return
    const data = await res.json() as unknown[][]
    sparkline.value = data.map(k => parseFloat(k[4] as string))
  } catch { /* silencieux */ }
}

function onCardEnter(event: MouseEvent, c: CryptoAlert) {
  if (leaveTimer !== null) { clearTimeout(leaveTimer); leaveTimer = null }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampedX = Math.max(136, Math.min(window.innerWidth - 136, rawX))
  pos.value = { x: clampedX, y: rect.bottom + 8 }
  if (hovered.value?.ticker !== c.ticker) fetchSparkline(c.ticker)
  hovered.value = c
}
function onCardLeave() { leaveTimer = setTimeout(() => { hovered.value = null }, 120) }
function onTipEnter() { if (leaveTimer !== null) { clearTimeout(leaveTimer); leaveTimer = null } }
function onTipLeave() { leaveTimer = setTimeout(() => { hovered.value = null }, 120) }
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
.tooltip-enter-active, .tooltip-leave-active { transition: opacity 0.12s, transform 0.12s; }
.tooltip-enter-from, .tooltip-leave-to { opacity: 0; transform: translateX(-50%) translateY(-4px); }
.tooltip-enter-to, .tooltip-leave-from { opacity: 1; transform: translateX(-50%) translateY(0); }
</style>
