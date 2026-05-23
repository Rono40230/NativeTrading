<template>
  <div class="glass-card rounded-xl border border-white/10 bg-white/5 p-3">
    <h3 class="text-xs font-semibold text-gray-300 mb-2.5">Distribution des résultats</h3>
    <div class="flex flex-col gap-2">
      <div v-for="item in distribution" :key="item.label" class="flex items-center gap-2">
        <span class="text-[10px] font-mono w-8 shrink-0 font-semibold" :class="item.textClass">
          {{ item.label }}
        </span>
        <div class="flex-1 bg-white/5 rounded-full h-1.5 overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-500"
            :class="item.barClass"
            :style="{ width: item.pct + '%' }"
          />
        </div>
        <span class="text-[10px] text-gray-500 w-[68px] text-right shrink-0">
          {{ item.count }} · {{ item.pct.toFixed(0) }}%
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { TradeBacktest } from '@/services/api.backtest'

const CONFIGS: Record<string, { label: string; textClass: string; barClass: string; ordre: number }> = {
  Tp1:        { label: 'TP1',   textClass: 'text-emerald-400', barClass: 'bg-emerald-500', ordre: 1 },
  Tp2:        { label: 'TP2',   textClass: 'text-emerald-300', barClass: 'bg-emerald-400', ordre: 2 },
  Tp3:        { label: 'TP3',   textClass: 'text-teal-300',    barClass: 'bg-teal-400',    ordre: 3 },
  double_sl:  { label: '2× SL', textClass: 'text-red-400',     barClass: 'bg-red-600',     ordre: 4 },
  StopLoss:   { label: 'SL',    textClass: 'text-red-400',     barClass: 'bg-red-500',     ordre: 5 },
  NonFerme:   { label: '—',     textClass: 'text-gray-500',    barClass: 'bg-gray-600',    ordre: 6 },
}

const props = defineProps<{ trades: TradeBacktest[] }>()

const distribution = computed(() => {
  const total = props.trades.length
  if (!total) return []
  const counts: Record<string, number> = {}
  for (const t of props.trades) {
    // Straddle : catégorie = TP du gagnant (Tp1/Tp2/Tp3) ou double_sl
    // SMC/Rockets : résultat standard
    const cle = t.direction === 'Both' ? (t.categorie || t.resultat) : t.resultat
    counts[cle] = (counts[cle] ?? 0) + 1
  }
  return Object.entries(counts)
    .map(([key, count]) => ({
      ...(CONFIGS[key] ?? { label: key, textClass: 'text-gray-400', barClass: 'bg-gray-500', ordre: 9 }),
      count,
      pct: (count / total) * 100,
    }))
    .sort((a, b) => a.ordre - b.ordre)
})
</script>
