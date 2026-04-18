<template>
  <!-- Bloc Surveillance Assets — liste compacte 1 asset/ligne -->
  <div class="glass-card p-4 flex flex-col gap-3 overflow-hidden">
    <!-- Titre -->
    <div class="flex items-center justify-between shrink-0">
      <span class="text-xs uppercase font-bold text-white">&#128208; Surveillance Assets</span>
    </div>
    <!-- Liste -->
    <div v-if="chargement && assets.length === 0" class="flex-1 flex flex-col gap-1 overflow-hidden">
      <div v-for="n in 5" :key="n" class="h-4 rounded bg-white/5 animate-pulse" />
    </div>
    <div v-else class="flex-1 flex flex-col gap-0.5 overflow-y-auto min-h-0">
      <div
        v-for="a in assets"
        :key="a.id"
        class="flex items-center gap-1.5 px-1.5 py-0.5 rounded hover:bg-white/5 transition-colors shrink-0"
      >
        <span class="text-[10px] font-bold text-white w-14 shrink-0 truncate">{{ a.id }}</span>
        <div class="flex-1 min-w-0" />
        <span v-if="a.chargement" class="text-[9px] text-gray-500 animate-pulse shrink-0">…</span>
        <span v-else-if="a.prix !== null" class="text-[10px] font-mono text-gray-300 shrink-0">{{ formatPrix(a.prix, a.id) }}</span>
        <span v-else class="text-[9px] text-gray-600 shrink-0">—</span>
        <span
          v-if="a.variation !== null"
          class="text-[9px] font-bold shrink-0 w-12 text-right tabular-nums"
          :class="a.variation >= 0 ? 'text-emerald-400' : 'text-red-400'"
        >{{ a.variation >= 0 ? '+' : '' }}{{ a.variation.toFixed(2) }}%</span>
        <span v-else class="text-[9px] text-gray-600 shrink-0 w-12 text-right">—</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
type VariationsMultiTF = { h1: number | null; h4: number | null; d1: number | null; w1: number | null; m1: number | null }
type AssetAvecPrix = {
  id: string
  prix: number | null
  variation: number | null
  variationsMultiTF: VariationsMultiTF | null
  clotures: Record<string, number[]>
  chargement: boolean
}

const props = defineProps<{ assets: AssetAvecPrix[]; chargement?: boolean }>()

const DEVISES: Record<string, string> = {
  BTC: 'USD', ETH: 'USD',
  XAUUSD: 'USD', XAGUSD: 'USD', EURUSD: 'USD',
  GBPJPY: 'JPY', CADJPY: 'JPY', NZDJPY: 'JPY',
  USDCAD: 'CAD', USDJPY: 'JPY', DAX: 'EUR', SP500: 'USD',
}

function formatPrix(prix: number, id: string): string {
  const decimales = id === 'BTC' ? 0
    : DEVISES[id] === 'JPY' ? 2
    : prix > 1000 ? 0
    : prix > 1 ? 2
    : 4
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: decimales,
    maximumFractionDigits: decimales,
  }).format(prix)
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-bar  { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
</style>
