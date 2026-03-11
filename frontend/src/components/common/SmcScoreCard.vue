<template>
  <div class="glass-card p-5">
    <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
      Score SMC — {{ asset }} {{ timeframe }}
    </h2>
    <div v-if="scoreSmc" class="space-y-3">
      <!-- Score total + badge confluence -->
      <div class="flex items-center gap-4">
        <span class="text-4xl font-bold" :class="scoreCouleur(scoreSmc.total)">
          {{ scoreSmc.total.toFixed(0) }}
          <span class="text-lg text-gray-400">/100</span>
        </span>
        <span
          class="px-3 py-1 rounded-full text-xs font-semibold"
          :class="scoreSmc.confluence
            ? 'bg-emerald-500/20 text-emerald-300'
            : 'bg-gray-500/20 text-gray-400'"
        >
          {{ scoreSmc.confluence ? '✓ Confluence' : '⚠ Insuffisant' }}
        </span>
        <span class="text-sm font-medium" :class="directionColor(scoreSmc.direction)">
          {{ scoreSmc.direction.toUpperCase() }}
        </span>
      </div>
      <!-- Barre de progression globale -->
      <div class="w-full bg-gray-700 rounded-full h-2">
        <div
          class="h-2 rounded-full transition-all"
          :class="scoreSmc.confluence ? 'bg-emerald-500' : 'bg-yellow-500'"
          :style="{ width: `${scoreSmc.total}%` }"
        />
      </div>
      <!-- Détail composants -->
      <div class="grid grid-cols-5 gap-2 mt-2">
        <div v-for="comp in composants" :key="comp.label" class="text-center">
          <div class="text-xs text-gray-500 mb-1">{{ comp.label }}</div>
          <div class="text-sm font-bold" :class="comp.pts > 0 ? 'text-emerald-400' : 'text-gray-600'">
            {{ comp.pts.toFixed(0) }}
          </div>
          <div class="text-xs text-gray-600">/{{ comp.max }}</div>
        </div>
      </div>
    </div>
    <div v-else class="text-gray-500 text-sm text-center py-4">
      Chargement analyse SMC...
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ScoreSmc } from '@/services/api.service'

const props = defineProps<{
  scoreSmc: ScoreSmc | null
  asset: string
  timeframe: string
}>()

const composants = computed(() => {
  const s = props.scoreSmc
  if (!s) return []
  return [
    { label: 'Tendance',   pts: s.tendance,     max: 25 },
    { label: 'Ord. Block', pts: s.order_block,  max: 25 },
    { label: 'Imbalance',  pts: s.imbalance,    max: 20 },
    { label: 'IFVG',       pts: s.ifvg,         max: 15 },
    { label: 'Fibonacci',  pts: s.fibonacci,    max: 15 },
  ]
})

function scoreCouleur(score: number): string {
  if (score >= 70) return 'text-emerald-400'
  if (score >= 50) return 'text-yellow-400'
  return 'text-red-400'
}

function directionColor(dir: string): string {
  if (dir.toLowerCase().includes('long')) return 'text-emerald-400'
  if (dir.toLowerCase().includes('short')) return 'text-red-400'
  return 'text-yellow-400'
}
</script>
