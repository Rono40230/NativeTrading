<template>
  <div class="glass-card p-4 space-y-3">
    <h3 class="text-sm font-semibold text-gray-300">{{ titre }}</h3>
    <div v-if="stats" class="grid grid-cols-2 gap-3">
      <div class="text-center">
        <div class="text-2xl font-bold" :class="stats.win_rate >= 55 ? 'text-emerald-400' : stats.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">
          {{ stats.win_rate.toFixed(0) }}%
        </div>
        <div class="text-xs text-gray-400">Win Rate</div>
      </div>
      <div class="text-center">
        <div class="text-2xl font-bold" :class="stats.pnl_r_moyen >= 1 ? 'text-emerald-400' : stats.pnl_r_moyen >= 0.5 ? 'text-yellow-400' : 'text-red-400'">
          {{ stats.pnl_r_moyen.toFixed(2) }}R
        </div>
        <div class="text-xs text-gray-400">R:R moyen</div>
      </div>
      <div class="text-center col-span-2 border-t border-white/10 pt-2">
        <span class="text-sm text-gray-300">{{ stats.nb_gagnants }} / {{ stats.nb_trades }} trades</span>
      </div>
    </div>
    <div v-else class="text-center text-gray-500 text-sm py-2">
      Aucun trade
    </div>
  </div>
</template>

<script setup lang="ts">
import type { StatsGlobales } from '@/services/api.ml_insights'
defineProps<{ titre: string; stats?: StatsGlobales | null }>()
</script>
