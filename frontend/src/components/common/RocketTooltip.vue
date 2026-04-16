<template>
  <Teleport to="body">
    <Transition name="tooltip">
      <div v-if="signal" class="fixed z-[9999] w-60 rounded-xl border border-white/20 p-4 shadow-2xl"
        :style="{ top: pos.y + 'px', left: pos.x + 'px', transform: 'translateX(-50%) translateY(-100%)', background: '#0b0f28' }"
        @click.stop>
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-2.5">
            <img :src="cryptoLogoUrl(signal.ticker)" :alt="signal.ticker"
              class="w-8 h-8 rounded-full border border-white/10 bg-white/5 object-contain"
              @error="(e) => ((e.target as HTMLImageElement).style.display = 'none')" />
            <div>
              <span class="text-sm font-bold text-white block">{{ signal.ticker }}</span>
              <span class="text-[10px] text-gray-400">{{ cryptoName(signal.ticker) }}</span>
            </div>
          </div>
          <span class="text-[11px]">{{ icone(signal.phase) }} <span class="text-gray-400 text-[10px]">{{ labelPhase(signal.phase) }}</span></span>
        </div>
        <!-- Sparkline multi-TF -->
        <div class="mb-3">
          <div class="flex items-center justify-between mb-1">
            <p class="text-[10px] text-gray-500">Tendance — {{ selectedTf }}</p>
            <div class="flex gap-0.5">
              <button v-for="tf in tfConfigs" :key="tf.label"
                class="text-[9px] px-1.5 py-0.5 rounded transition-colors"
                :class="selectedTf === tf.label ? 'bg-white/15 text-white' : 'text-gray-500 hover:text-gray-300'"
                @click.stop="$emit('chooseTf', tf)">{{ tf.label }}</button>
            </div>
          </div>
          <svg viewBox="0 0 240 48" class="w-full" style="height:44px">
            <template v-if="sparkline.length >= 2">
              <polyline :points="sparklinePath(sparkline)" fill="none" :stroke="couleur"
                stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round" />
            </template>
            <text v-else x="120" y="26" text-anchor="middle" fill="#4b5563" font-size="9">Chargement…</text>
          </svg>
        </div>
        <div class="space-y-1.5 text-[11px]">
          <div class="flex justify-between"><span class="text-gray-500">Prix</span><span class="text-white font-mono">{{ formatPrix(signal.prix) }}$</span></div>
          <div class="flex justify-between"><span class="text-gray-500">Variation {{ selectedTf }}</span><span :class="variation >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ variation >= 0 ? '+' : '' }}{{ variation.toFixed(2) }}%</span></div>
          <div class="flex justify-between"><span class="text-gray-500">Volume spike</span><span :class="signal.ratioVolume >= 2 ? 'text-orange-400' : 'text-gray-300'">{{ signal.ratioVolume.toFixed(2) }}×</span></div>
          <div class="flex justify-between"><span class="text-gray-500">ATR ratio</span><span :class="signal.atrRatio < 0.75 ? 'text-blue-400' : 'text-gray-300'">{{ signal.atrRatio.toFixed(2) }}</span></div>
          <div class="flex justify-between"><span class="text-gray-500">RSI (14)</span><span :class="labelRsi(signal.rsi).classe">{{ signal.rsi.toFixed(1) }} — {{ labelRsi(signal.rsi).label }}</span></div>
          <div class="border-t border-white/10 pt-1.5 mt-1.5 space-y-1">
            <div class="flex justify-between"><span class="text-gray-500">Support / SL</span><span class="text-red-400 font-mono">{{ formatPrix(signal.support) }}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">Résistance / TP</span><span class="text-emerald-400 font-mono">{{ formatPrix(signal.target20) }}</span></div>
          </div>
          <div class="flex justify-between border-t border-white/10 pt-1.5">
            <span class="text-gray-500">Score</span>
            <span class="font-bold" :class="signal.score >= 70 ? 'text-orange-400' : 'text-emerald-400'">{{ signal.score }}/100</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { cryptoName, cryptoLogoUrl } from '@/composables/useCryptoMeta'
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'

export type TFConfig = { label: string; interval: string | null; limit: number }

defineProps<{
  signal: SignalRocket | null
  pos: { x: number; y: number }
  selectedTf: string
  sparkline: number[]
  couleur: string
  variation: number
  tfConfigs: TFConfig[]
}>()

defineEmits<{ chooseTf: [tf: TFConfig] }>()

function icone(phase: PhaseRocket): string {
  if (phase === 'breakout') return '🚀'
  if (phase === 'prelancement') return '⚡'
  return '🌀'
}

function labelPhase(phase: PhaseRocket): string {
  if (phase === 'breakout') return 'Breakout'
  if (phase === 'prelancement') return 'Pré-lancement'
  return 'Compression'
}

function labelRsi(rsi: number): { label: string; classe: string } {
  if (rsi < 40) return { label: 'survendu', classe: 'text-blue-400' }
  if (rsi < 50) return { label: 'neutre↓', classe: 'text-gray-400' }
  if (rsi < 65) return { label: 'idéal ✓', classe: 'text-emerald-400' }
  if (rsi < 75) return { label: 'momentum', classe: 'text-yellow-400' }
  if (rsi < 85) return { label: 'chaud', classe: 'text-orange-400' }
  return { label: 'extrême !', classe: 'text-red-400' }
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}

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
</script>

<style scoped>
.tooltip-enter-active, .tooltip-leave-active { transition: opacity 0.12s, transform 0.12s; }
.tooltip-enter-from, .tooltip-leave-to { opacity: 0; transform: translateX(-50%) translateY(calc(-100% + 6px)); }
.tooltip-enter-to, .tooltip-leave-from { opacity: 1; transform: translateX(-50%) translateY(-100%); }
</style>
