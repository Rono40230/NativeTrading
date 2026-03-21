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

    <!-- Grille 5 colonnes, scroll après 3 lignes -->
    <div v-else class="grid grid-cols-5 gap-2 overflow-y-auto scroll-zone" style="max-height: calc(3 * 68px + 2 * 8px)">
      <div
        v-for="c in top20Tries"
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
        class="fixed z-[9999] w-72 rounded-xl border border-white/20 p-4 shadow-2xl text-white"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%)', background: '#0b0f28' }"
        @mouseenter="onTipEnter"
        @mouseleave="onTipLeave"
      >
        <div class="flex items-center justify-between mb-2">
          <span class="text-base font-bold">{{ hovered.ticker }}</span>
          <span class="text-[11px]">{{ icone(hovered.badge) }}</span>
        </div>
        <!-- Sparkline 1H (24 bougies) -->
        <div class="mb-3">
          <p class="text-[10px] text-gray-500 mb-1">Tendance 1H (24h)</p>
          <svg viewBox="0 0 272 56" class="w-full" style="height:48px">
            <template v-if="klines[hovered.symbol] && klines[hovered.symbol].length >= 2">
              <polyline
                :points="sparkline(klines[hovered.symbol])"
                fill="none"
                :stroke="hovered.change24h >= 0 ? '#10b981' : '#ef4444'"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
            </template>
            <template v-else-if="klinesChargement">
              <text x="136" y="30" text-anchor="middle" fill="#4b5563" font-size="9">Chargement…</text>
            </template>
            <template v-else>
              <text x="136" y="30" text-anchor="middle" fill="#4b5563" font-size="9">Données indisponibles</text>
            </template>
          </svg>
        </div>
        <div class="space-y-1.5 text-[11px]">
          <div class="flex justify-between">
            <span class="text-gray-500">Prix</span>
            <span class="font-mono">{{ formatPrix(hovered.prix) }}$</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Variation 24h</span>
            <span class="font-bold text-emerald-400">+{{ hovered.change24h.toFixed(2) }}%</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Volume 24h</span>
            <span>{{ formatVolume(hovered.volume24h) }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Trades 24h</span>
            <span>{{ hovered.nbTrades.toLocaleString('fr-FR') }}</span>
          </div>
          <div class="flex justify-between border-t border-white/10 pt-1.5 mt-1.5">
            <span class="text-gray-500">Score</span>
            <span class="font-bold" :class="classScore(hovered.score)">{{ hovered.score.toFixed(1) }}/100</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Signal</span>
            <span class="font-semibold">{{ labelBadge(hovered.badge) }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { CryptoAlert, BadgeNiveau } from '@/composables/useCryptosAlert'

const props = defineProps<{
  top20: CryptoAlert[]
  chargement: boolean
  erreur: boolean
}>()

const top20Tries = computed(() =>
  [...props.top20].sort((a, b) => b.change24h - a.change24h)
)

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

const hovered = ref<CryptoAlert | null>(null)
const pos = ref({ x: 0, y: 0 })
const klines = ref<Record<string, number[]>>({})
const klinesChargement = ref(false)
let leaveTimer: ReturnType<typeof setTimeout> | null = null

function sparkline(closes: number[]): string {
  if (closes.length < 2) return ''
  const W = 272, H = 56
  const min = Math.min(...closes), max = Math.max(...closes)
  const range = max - min || 1
  return closes.map((v, i) => {
    const x = (i / (closes.length - 1)) * W
    const y = H - ((v - min) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  }).join(' ')
}

async function fetchKlines(symbol: string) {
  if (klines.value[symbol]) return
  klinesChargement.value = true
  try {
    const res = await fetch(`https://api.binance.com/api/v3/klines?symbol=${symbol}&interval=1h&limit=24`)
    if (!res.ok) return
    const data = await res.json() as Array<unknown[]>
    klines.value = { ...klines.value, [symbol]: data.map(k => parseFloat(k[4] as string)) }
  } catch { /* silencieux */ } finally {
    klinesChargement.value = false
  }
}

function onCardEnter(event: MouseEvent, c: CryptoAlert) {
  if (leaveTimer !== null) { clearTimeout(leaveTimer); leaveTimer = null }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampedX = Math.max(148, Math.min(window.innerWidth - 148, rawX))
  pos.value = { x: clampedX, y: rect.bottom + 8 }
  hovered.value = c
  fetchKlines(c.symbol)
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
