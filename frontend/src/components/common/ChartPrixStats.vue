<template>
  <div class="flex items-center gap-3 flex-wrap">
    <div v-if="dernierPrix !== null" class="flex items-baseline gap-3">
      <span class="text-3xl font-bold">{{ formatPrix(dernierPrix) }}</span>
      <span class="text-sm" :class="variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
        {{ variation >= 0 ? '+' : '' }}{{ variation.toFixed(2) }}%
      </span>
      <span class="text-xs text-gray-500">
        {{ selectedAsset.includes('USD') ? selectedAsset : `${selectedAsset}/USDT` }} · {{ selectedTimeframe }}
      </span>
      <span
        v-if="wsConnecte"
        class="flex items-center gap-1 text-xs ml-2"
        :class="['BTC','ETH'].includes(selectedAsset) ? 'text-emerald-400' : 'text-blue-400'"
      >
        <span
          class="w-1.5 h-1.5 rounded-full animate-pulse inline-block"
          :class="['BTC','ETH'].includes(selectedAsset) ? 'bg-emerald-400' : 'bg-blue-400'"
        />
        {{ ['BTC','ETH'].includes(selectedAsset) ? 'LIVE' : 'LIVE 5s' }}
      </span>
    </div>

    <div v-if="stats" class="flex items-center gap-2 ml-auto">
      <div class="glass-card px-4 py-2 flex flex-col items-center min-w-[72px]">
        <span class="text-xs text-slate-400 leading-none">Bougies</span>
        <span class="text-sm font-semibold text-white mt-1">{{ stats.count }}</span>
      </div>
      <div class="glass-card px-4 py-2 flex flex-col items-center min-w-[80px]">
        <span class="text-xs text-slate-400 leading-none">Vol. moy</span>
        <span class="text-sm font-semibold text-white mt-1">{{ formatVolume(stats.volumeMoy) }}</span>
      </div>
      <div class="glass-card px-4 py-2 flex flex-col items-center min-w-[100px]">
        <span class="text-xs text-slate-400 leading-none">Plus haut</span>
        <span class="text-sm font-semibold text-emerald-400 mt-1">{{ formatPrix(stats.high) }}</span>
      </div>
      <div class="glass-card px-4 py-2 flex flex-col items-center min-w-[100px]">
        <span class="text-xs text-slate-400 leading-none">Plus bas</span>
        <span class="text-sm font-semibold text-red-400 mt-1">{{ formatPrix(stats.low) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  dernierPrix: number | null
  variation: number
  stats: { count: number; high: number; low: number; volumeMoy: number } | null
  selectedAsset: string
  selectedTimeframe: string
  wsConnecte: boolean
}>()

function formatPrix(v: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(v)
}

function formatVolume(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(2)}M`
  if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`
  return v.toFixed(2)
}
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
