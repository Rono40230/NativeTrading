<template>
  <div
    class="rounded-xl border px-3 py-2 flex flex-col gap-1 cursor-pointer transition-colors hover:brightness-110 relative"
    :class="[classeCard(s.phase), s.score >= 65 ? 'ring-1 ring-emerald-500/60' : '']"
    @click.stop="emit('click', $event)"
  >
    <!-- Header ligne 1 : Ticker + variation et Badges -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="text-sm font-bold text-white">{{ s.ticker }}</span>
        <span class="text-[10px] font-bold px-1.5 py-0.5 rounded" :class="s.change1h >= 0 ? 'bg-emerald-500/20 text-emerald-400' : 'bg-red-500/20 text-red-400'">
          {{ s.change1h >= 0 ? '+' : '' }}{{ s.change1h.toFixed(2) }}%
        </span>
      </div>
      <div class="flex items-center gap-1.5">
        <div class="text-[9px] font-bold border rounded px-1.5 py-0.5 flex items-center gap-1 transition-all" :class="configPhase(s.phase).classe">
          <span>{{ configPhase(s.phase).icon }}</span>
          <span>{{ configPhase(s.phase).label }}</span>
        </div>
        <div v-if="s.score >= 65" class="text-[9px] font-bold text-emerald-300 bg-emerald-500/20 border border-emerald-500/40 rounded px-1.5 py-0.5 animate-pulse">ÉLIGIBLE</div>
      </div>
    </div>

    <!-- Header ligne 2 : Prix -->
    <div class="flex items-center justify-between text-[11px]">
      <span class="font-mono text-white font-semibold">{{ formatPrix(s.prix) }}$</span>
    </div>

    <!-- Barre score -->
    <div class="flex items-center gap-1.5 py-0.5">
      <div class="relative flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden">
        <div class="absolute inset-y-0 left-0 rounded-full transition-all" :class="s.score >= 65 ? 'bg-emerald-500' : s.score >= 45 ? 'bg-yellow-500' : 'bg-gray-500'" :style="{ width: `${Math.min(s.score, 100)}%` }" />
        <div class="absolute inset-y-0 w-px bg-white/50" style="left:65%" />
      </div>
      <span class="text-[10px] font-mono shrink-0 font-bold" :class="s.score >= 65 ? 'text-emerald-400' : s.score >= 45 ? 'text-yellow-500' : 'text-gray-500'">{{ s.score }}<span class="text-gray-600 font-normal">/100</span></span>
    </div>

    <!-- Métriques compactées sur 1 ligne (3 colonnes) -->
    <div class="grid grid-cols-3 gap-1 text-[9px] mt-0.5">
      <!-- RSI -->
      <div class="flex flex-col bg-black/20 rounded p-1 text-center">
        <span class="text-gray-500 mb-0.5">RSI {{ s.rsi.toFixed(0) }}</span>
        <span class="font-semibold truncate" :class="labelRsi(s.rsi).classe">{{ labelRsi(s.rsi).label }}</span>
      </div>
      <!-- ATR -->
      <div class="flex flex-col bg-black/20 rounded p-1 text-center">
        <span class="text-gray-500 mb-0.5">ATR</span>
        <span class="font-semibold truncate" :class="s.atrRatio < 0.75 ? 'text-blue-400' : s.atrRatio > 1.5 ? 'text-orange-400' : 'text-gray-300'">
          {{ s.atrRatio.toFixed(2) }}×
        </span>
      </div>
      <!-- Volume -->
      <div class="flex flex-col bg-black/20 rounded p-1 text-center">
        <span class="text-gray-500 mb-0.5">Vol</span>
        <span class="font-semibold truncate" :class="s.ratioVolume >= 2 ? 'text-orange-400' : s.ratioVolume >= 1.3 ? 'text-yellow-400' : 'text-gray-400'">
          {{ s.ratioVolume.toFixed(1) }}×
        </span>
      </div>
    </div>

    <!-- Tendance + entrée (condensé à la fin) -->
    <div class="flex items-center justify-between text-[9px] pt-1 mt-1 border-t border-white/5">
      <div class="flex items-center gap-1.5">
        <span class="font-semibold" :class="s.tendanceHaussiere ? 'text-emerald-400' : 'text-gray-500'">{{ s.tendanceHaussiere ? '↗ haussière' : '↘ neutre' }}</span>
        <span v-if="s.phase !== 'breakout'" class="text-gray-500">· VCP: {{ (s.volumeSeche ?? 0).toFixed(1) }}×</span>
      </div>
      <span v-if="s.typeEntreeRec" class="shrink-0 font-medium" :class="s.typeEntreeRec === 'limite' ? 'text-sky-400' : 'text-yellow-400'">
        {{ s.typeEntreeRec === 'limite' ? `Lim ${formatPrix(s.entreeLimite)}$` : `Stop ${formatPrix(s.entreeStop)}$` }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'

defineProps<{ s: SignalRocket }>()
const emit = defineEmits<{ click: [e: MouseEvent] }>()

function configPhase(phase: PhaseRocket): { label: string; icon: string; classe: string } {
  if (phase === 'breakout')     return { label: 'Breakout', icon: '🚀', classe: 'text-emerald-300 bg-emerald-500/20 border-emerald-500/40 animate-pulse' }
  if (phase === 'prelancement') return { label: 'Pré-lancement', icon: '⏳', classe: 'text-yellow-300 bg-yellow-500/20 border-yellow-500/40 animate-pulse' }
  return { label: 'Compression', icon: '🗜️', classe: 'text-blue-300 bg-blue-500/20 border-blue-500/40 animate-[pulse_3s_ease-in-out_infinite]' }
}

function classeCard(phase: PhaseRocket): string {
  if (phase === 'breakout')     return 'border-emerald-500/50 bg-emerald-500/10'
  if (phase === 'prelancement') return 'border-yellow-500/40 bg-yellow-500/[0.08]'
  return 'border-blue-500/30 bg-blue-500/[0.06]'
}

function labelRsi(rsi: number): { label: string; classe: string } {
  if (rsi < 40) return { label: 'survendu',  classe: 'text-blue-400' }
  if (rsi < 50) return { label: 'neutre↓',   classe: 'text-gray-400' }
  if (rsi < 65) return { label: 'idéal ✓',   classe: 'text-emerald-400' }
  if (rsi < 75) return { label: 'momentum',  classe: 'text-yellow-400' }
  if (rsi < 85) return { label: 'chaud',     classe: 'text-orange-400' }
  return               { label: 'extrême !', classe: 'text-red-400' }
}

function formatPrix(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 1 ? v.toFixed(4) : v.toFixed(6)
}
</script>
