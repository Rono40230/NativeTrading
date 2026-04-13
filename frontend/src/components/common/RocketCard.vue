<template>
  <div
    class="rounded-xl border px-3 py-2.5 flex flex-col gap-1.5 cursor-pointer transition-colors hover:brightness-110 relative"
    :class="[classeCard(s.phase), s.score >= 65 ? 'ring-1 ring-emerald-500/60' : '']"
    @click.stop="emit('click', $event)"
  >
    <!-- Badge ÉLIGIBLE -->
    <div v-if="s.score >= 65" class="absolute top-2 right-2 text-[9px] font-bold text-emerald-300 bg-emerald-500/20 border border-emerald-500/40 rounded px-1.5 py-0.5 animate-pulse">ÉLIGIBLE</div>

    <!-- Ticker + phase + variation -->
    <div class="flex items-center gap-1 pr-16">
      <span class="text-[13px] font-bold text-white truncate flex-1">{{ s.ticker }}</span>
      <span class="text-[10px] shrink-0 text-gray-400 font-medium">{{ labelPhase(s.phase) }}</span>
      <span class="text-[11px] font-bold shrink-0" :class="s.change1h >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ s.change1h >= 0 ? '+' : '' }}{{ s.change1h.toFixed(2) }}%</span>
    </div>

    <!-- Prix -->
    <div class="flex items-center justify-between text-[10px]">
      <span class="text-gray-500">Prix</span>
      <span class="font-mono text-white font-semibold">{{ formatPrix(s.prix) }}$</span>
    </div>

    <!-- Barre score -->
    <div class="flex items-center gap-1.5">
      <div class="relative flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden">
        <div class="absolute inset-y-0 left-0 rounded-full transition-all" :class="s.score >= 65 ? 'bg-emerald-500' : s.score >= 45 ? 'bg-yellow-500' : 'bg-gray-500'" :style="{ width: `${Math.min(s.score, 100)}%` }" />
        <div class="absolute inset-y-0 w-px bg-white/50" style="left:65%" />
      </div>
      <span class="text-[10px] font-mono shrink-0 font-bold" :class="s.score >= 65 ? 'text-emerald-400' : s.score >= 45 ? 'text-yellow-500' : 'text-gray-500'">{{ s.score }}<span class="text-gray-600 font-normal">/100</span></span>
    </div>

    <!-- RSI -->
    <div class="flex items-center justify-between text-[10px]">
      <span class="text-gray-500">RSI {{ s.rsi.toFixed(0) }}</span>
      <span class="font-semibold" :class="labelRsi(s.rsi).classe">{{ labelRsi(s.rsi).label }}</span>
    </div>

    <!-- ATR ratio -->
    <div class="flex items-center justify-between text-[10px]">
      <span class="text-gray-500">ATR ratio</span>
      <span :class="s.atrRatio < 0.75 ? 'text-blue-400 font-semibold' : s.atrRatio > 1.5 ? 'text-orange-400 font-semibold' : 'text-gray-300'">
        {{ s.atrRatio.toFixed(2) }}{{ s.atrRatio < 0.75 ? ' — ressort chargé' : s.atrRatio > 1.5 ? ' — expansion' : '' }}
      </span>
    </div>

    <!-- Volume spike -->
    <div class="flex items-center justify-between text-[10px]">
      <span class="text-gray-500">Volume</span>
      <span :class="s.ratioVolume >= 2 ? 'text-orange-400 font-semibold' : s.ratioVolume >= 1.3 ? 'text-yellow-400' : 'text-gray-400'">
        {{ s.ratioVolume.toFixed(1) }}× {{ s.ratioVolume >= 2 ? '— spike !' : s.ratioVolume >= 1.3 ? '— élevé' : '— normal' }}
      </span>
    </div>

    <!-- Compression (uniquement si phase ≠ breakout) -->
    <div v-if="s.phase !== 'breakout'" class="text-[10px] text-gray-400">
      ⏳ <span class="text-white font-medium">{{ s.nbBougiesCompression }}</span> bougies en compression
      <span v-if="s.volumeSeche < 0.75" class="text-blue-400 ml-1">· VCP actif ({{ s.volumeSeche.toFixed(2) }}×)</span>
      <span v-else class="text-gray-600 ml-1">· vol. normal</span>
    </div>

    <!-- Tendance + entrée -->
    <div class="flex items-center justify-between text-[10px] pt-0.5 border-t border-white/5 mt-0.5">
      <span class="font-semibold" :class="s.tendanceHaussiere ? 'text-emerald-400' : 'text-gray-500'">{{ s.tendanceHaussiere ? '↗ haussière' : '↘ neutre' }}</span>
      <span v-if="s.typeEntreeRec" class="shrink-0" :class="s.typeEntreeRec === 'limite' ? 'text-sky-400' : 'text-yellow-400'">
        {{ s.typeEntreeRec === 'limite' ? `Limite ${formatPrix(s.entreeLimite)}$` : `Stop ${formatPrix(s.entreeStop)}$` }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'

defineProps<{ s: SignalRocket }>()
const emit = defineEmits<{ click: [e: MouseEvent] }>()

function labelPhase(phase: PhaseRocket): string {
  if (phase === 'breakout')     return 'Breakout'
  if (phase === 'prelancement') return 'Pré-lancement'
  return 'Compression'
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
