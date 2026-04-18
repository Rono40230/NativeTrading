<template>
  <div class="glass-bar px-4 py-2.5 flex flex-col gap-2 h-full">
    <span class="text-xs font-semibold uppercase tracking-widest text-white shrink-0">⚡ ATR M15 Actuel</span>

    <div v-if="chargementAtr" class="text-[10px] text-gray-600 animate-pulse">Chargement…</div>
    <div v-else-if="topAtr.length" class="flex flex-col gap-1.5">
      <div v-for="a in topAtr" :key="a.asset" class="flex flex-col gap-0.5">
        <div class="flex items-center justify-between">
          <span class="text-[10px] font-semibold text-white/80">{{ a.asset }}</span>
          <span class="font-mono text-[10px]" :style="{ color: couleurRatio(a.ratio) }">{{ a.ratio.toFixed(0) }}%</span>
        </div>
        <div class="relative h-1.5 rounded-full bg-white/10 overflow-hidden">
          <div class="absolute inset-y-0 left-0 rounded-full transition-all duration-500"
            :style="{ width: Math.min(a.ratio, 200) / 2 + '%', background: couleurRatio(a.ratio) }"></div>
          <div class="absolute inset-y-0 w-px bg-white/40" style="left:50%"></div>
        </div>
      </div>
    </div>
    <div v-else class="text-[10px] text-gray-600 italic">Pas de données ATR</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

const assetsStore = useAssetsStore()
const atrRatios = ref<Record<string, number>>({})
const chargementAtr = ref(true)

function calcAtr(candles: Candle[], periode: number): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const f = trs.slice(-Math.min(periode, trs.length))
  return f.reduce((s, v) => s + v, 0) / f.length
}

function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  const court = calcAtr(candles.slice(-7), 6)
  const long = calcAtr(candles, Math.min(candles.length - 1, 60))
  return long > 0 ? (court / long) * 100 : 100
}

function couleurRatio(r: number): string {
  if (r < 80) return '#10b981'
  if (r < 120) return '#f59e0b'
  return '#ef4444'
}

const topAtr = computed(() =>
  Object.entries(atrRatios.value)
    .map(([asset, ratio]) => ({ asset, ratio }))
    .sort((a, b) => b.ratio - a.ratio)
    .slice(0, 2)
)

async function chargerAtr() {
  chargementAtr.value = true
  const resultats = await Promise.allSettled(
    assetsStore.assets.map(a => apiService.getCandles(a.id, 'M15', 80).then(c => ({ asset: a.id, c })))
  )
  const nouveaux: Record<string, number> = {}
  for (const r of resultats) {
    if (r.status === 'fulfilled') nouveaux[r.value.asset] = calcAtrRatio(r.value.c)
  }
  atrRatios.value = nouveaux
  chargementAtr.value = false
}

let _pollAtr: ReturnType<typeof setInterval> | null = null
onMounted(async () => {
  if (!assetsStore.assets.length) await assetsStore.chargerAssets()
  chargerAtr()
  _pollAtr = setInterval(chargerAtr, 60_000)
})
onUnmounted(() => {
  if (_pollAtr !== null) { clearInterval(_pollAtr); _pollAtr = null }
})
</script>

<style scoped>
.glass-bar {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
