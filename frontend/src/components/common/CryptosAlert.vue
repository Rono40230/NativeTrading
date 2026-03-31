<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3 flex-wrap">
        <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
          ⚡ Momentum Cryptos — Forte variation 24h
        </p>
        <div class="flex items-center gap-3 text-[9px] font-medium">
          <span class="text-red-400">🚀 Explosion</span>
          <span class="text-orange-400">⚡ Élan</span>
          <span class="text-yellow-400">🔥 Chaud</span>
          <span class="text-amber-400">🎯 + Rockets</span>
          <span class="text-orange-300">↘ Ralentissement</span>
        </div>        <span class="text-[9px] text-gray-600">{{ labelPaires }}</span>      </div>
      <div class="flex items-center gap-2">
        <span v-if="erreur" class="text-[10px] text-red-400">Binance indisponible</span>
        <div v-if="chargement" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
        <span v-else-if="top20.length > 0" class="text-[9px] text-gray-600">{{ countdown }}s</span>
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
    <div v-else-if="top20.length === 0 && chargement" class="grid grid-cols-8 gap-2">
      <div
        v-for="n in 15"
        :key="n"
        class="rounded-lg border border-white/5 bg-white/5 px-3 py-2 h-[36px] animate-pulse"
      />
    </div>

    <!-- Grille 5 colonnes, scroll après 3 lignes, triée score décroissant -->
    <div v-else class="grid grid-cols-8 gap-2 overflow-y-auto scroll-zone" style="max-height: calc(2 * 44px + 1 * 8px)">
      <div
        v-for="c in top20ParScore"
        :key="c.symbol"
        class="rounded-lg border px-2.5 py-1.5 flex items-center gap-1.5 transition-colors hover:brightness-125 cursor-pointer"
        :class="[classeCard(c.badge), dansScan(c.ticker) ? 'ring-2 ring-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]' : '']"
        @click.stop="onCardClick($event, c)"
      >
        <span class="text-[11px] font-bold text-white truncate flex-1 min-w-0">{{ c.ticker }}</span>
        <span class="text-[10px] shrink-0">{{ dansScan(c.ticker) ? '🎯' : icone(c.badge) }}</span>
        <span class="text-[10px] font-bold text-emerald-400 shrink-0">+{{ c.change24h.toFixed(2) }}%</span>
        <span v-if="c.ralentissement" class="text-[9px] text-orange-400 shrink-0">↘2</span>
        <span v-else class="text-[9px] text-gray-500 shrink-0">{{ formatVolume(c.volume24h) }}</span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <Transition name="tooltip">
      <div
        v-if="hovered"
        class="fixed z-[9999] w-64 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @click.stop
      >
        <div class="flex items-center justify-between mb-3">
          <div>
            <span class="text-base font-bold text-white">{{ hovered.ticker }}</span>
            <span v-if="dansScan(hovered.ticker)" class="ml-2 text-[10px] font-bold text-amber-400">🎯 Confirmé Rockets</span>
          </div>
          <span class="text-[11px]">{{ icone(hovered.badge) }}</span>
        </div>
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
          <svg viewBox="0 0 240 50" class="w-full" style="height:48px">
            <template v-if="sparkline.length >= 2">
              <polyline
                :points="sparklinePath(sparkline)"
                fill="none"
                :stroke="couleurSparkline"
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
            <span class="text-gray-500">Variation {{ selectedTF }}</span>
            <span class="font-bold" :class="variationTF >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ variationTF >= 0 ? '+' : '' }}{{ variationTF.toFixed(2) }}%</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Variation 1h</span>
            <span v-if="hovered.change1h !== null" class="font-bold" :class="hovered.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ hovered.change1h >= 0 ? '+' : '' }}{{ hovered.change1h.toFixed(2) }}%</span>
            <span v-else class="text-gray-600 text-[10px]">chargement…</span>
          </div>
          <div v-if="hovered.ralentissement" class="flex items-center gap-1 text-orange-400 text-[10px] bg-orange-500/10 rounded px-2 py-1">
            <span>⚠️</span><span>Momentum 1h en baisse — pullback potentiel</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Volume 24h</span>
            <span class="text-white">{{ formatVolume(hovered.volume24h) }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Volume spike</span>
            <span :class="hovered.volumeRatio >= 5 ? 'text-orange-400' : hovered.volumeRatio >= 2 ? 'text-yellow-400' : 'text-gray-300'">{{ hovered.volumeRatio.toFixed(1) }}×</span>
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
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import type { CryptoAlert, BadgeNiveau } from '@/composables/useCryptosAlert'
import {
  TF_CONFIGS, icone, classeCard, formatVolume, formatPrix,
  classScore, labelBadge, sparklinePath,
} from '@/composables/useCryptosAlert'
import { usePrixStore } from '@/stores/prix.store'
import CryptosOpportunitesModal from '@/components/common/CryptosOpportunitesModal.vue'

const props = defineProps<{
  top20: CryptoAlert[]
  chargement: boolean
  erreur: boolean
  totalPaires: number
  rocketsTickers: string[]
}>()

function dansScan(ticker: string): boolean {
  return props.rocketsTickers.includes(ticker)
}

const labelPaires = computed(() =>
  props.chargement
    ? 'Chargement…'
    : props.top20.length > 0
      ? `${props.top20.length} signaux · ${props.totalPaires} paires analysées`
      : `${props.totalPaires > 0 ? props.totalPaires : '…'} paires Binance`
)

const modalOuverte = ref(false)

/** Tri réactif côté composant : score décroissant, mise à jour automatique */
const top20ParScore = computed(() =>
  [...props.top20].sort((a, b) => b.change24h - a.change24h)
)

const hoveredTicker = ref<string | null>(null)
const hovered = computed(() => hoveredTicker.value ? props.top20.find(c => c.ticker === hoveredTicker.value) ?? null : null)
const pos = ref({ x: 0, y: 0 })
const sparkline = ref<number[]>([])
const selectedTF = ref('D1')

const couleurSparkline = computed(() => {
  if (sparkline.value.length < 2) return '#10b981'
  return sparkline.value.at(-1)! >= sparkline.value[0] ? '#10b981' : '#ef4444'
})

const variationTF = computed(() => {
  if (sparkline.value.length < 2) return hovered.value?.change24h ?? 0
  const first = sparkline.value[0]
  const last = sparkline.value.at(-1)!
  return ((last - first) / first) * 100
})

async function fetchSparkline(ticker: string) {
  sparkline.value = []
  const tf = TF_CONFIGS.find(t => t.label === selectedTF.value) ?? TF_CONFIGS[2]
  try {
    const res = await fetch(`https://api.binance.com/api/v3/klines?symbol=${ticker}USDT&interval=${tf.interval}&limit=${tf.limit}`)
    if (!res.ok) return
    const data = await res.json() as unknown[][]
    sparkline.value = data.map(k => parseFloat(k[4] as string))
  } catch { /* silencieux */ }
}

function choisirTF(tf: { label: string; interval: string; limit: number }) {
  selectedTF.value = tf.label
  if (hoveredTicker.value) fetchSparkline(hoveredTicker.value)
}

function onCardClick(event: MouseEvent, c: CryptoAlert) {
  if (hoveredTicker.value === c.ticker) { hoveredTicker.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampedX = Math.max(136, Math.min(window.innerWidth - 136, rawX))
  pos.value = { x: clampedX, y: rect.top - 8 }
  selectedTF.value = 'D1'
  fetchSparkline(c.ticker)
  hoveredTicker.value = c.ticker
}

function fermerTooltip() { hoveredTicker.value = null }

// Countdown : se réinitialise à chaque refresh du store (10s)
const prixStore = usePrixStore()
const countdown = ref(10)
let tickInterval: ReturnType<typeof setInterval> | null = null
watch(() => prixStore.tickers, () => { countdown.value = 10 })

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
