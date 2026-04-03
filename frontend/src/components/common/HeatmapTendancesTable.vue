<template>
  <div class="section-card">
    <h3 class="section-title">
      📈 Tendances & créneaux actifs par asset
      <span class="text-gray-600 normal-case font-normal">(↑ volatilité en hausse · ↓ en baisse · → stable)</span>
    </h3>
    <div class="grid grid-cols-2 gap-x-8">
      <div
        v-for="a in assetsAvecTendance"
        :key="a.asset"
        class="flex items-center gap-2 py-1.5 border-b border-white/5 last:border-0"
      >
        <span class="text-sm font-semibold text-white w-16 shrink-0">{{ a.asset }}</span>
        <span class="text-sm font-bold w-5 shrink-0 text-center" :class="a.flecheClass">{{ a.fleche }}</span>
        <span class="text-[10px] text-gray-500 font-mono w-10 shrink-0">{{ a.ratioMoy.toFixed(0) }}%</span>
        <span v-if="a.tfsEleves.length" class="flex flex-wrap gap-1">
          <span
            v-for="tf in a.tfsEleves"
            :key="tf"
            class="bg-red-500/20 text-red-300 text-[10px] font-mono px-1.5 py-0.5 rounded"
          >{{ tf }}</span>
        </span>
        <span v-else class="text-gray-700 text-[10px] italic">aucun créneau élevé</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  classementVol: { cle: string; asset: string; tf: string; atr: number }[]
  tfsActifsParAsset: Record<string, string[]>
  assets: string[]
}>()

const SHORT_TFS = ['M1', 'M5', 'M15']
const LONG_TFS  = ['H4', 'D1', 'W1']

function moy(arr: number[]): number {
  return arr.length ? arr.reduce((s, v) => s + v, 0) / arr.length : 0
}

const assetsAvecTendance = computed(() =>
  props.assets.map(a => {
    const all      = props.classementVol.filter(i => i.asset === a)
    const shortAvg = moy(all.filter(i => SHORT_TFS.includes(i.tf)).map(i => i.atr))
    const longAvg  = moy(all.filter(i => LONG_TFS.includes(i.tf)).map(i => i.atr))
    const ratioMoy = moy(all.map(i => i.atr))
    const diff = shortAvg - longAvg
    let fleche = '→'; let flecheClass = 'text-gray-500'
    if (diff > 8)  { fleche = '↑'; flecheClass = 'text-red-400' }
    if (diff < -8) { fleche = '↓'; flecheClass = 'text-emerald-400' }
    const tfsEleves = props.tfsActifsParAsset[a] ?? []
    return { asset: a, fleche, flecheClass, ratioMoy, tfsEleves }
  })
)
</script>

<style scoped>
.section-card  { @apply rounded-xl border border-white/10 bg-white/5 px-4 py-3; }
.section-title { @apply text-[10px] font-semibold text-gray-400 uppercase tracking-wider mb-3; }
</style>
