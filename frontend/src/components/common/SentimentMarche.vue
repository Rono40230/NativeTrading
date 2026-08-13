<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="flex items-center justify-between mb-3">
      <div>
        <p class="text-[11px] font-semibold text-white uppercase tracking-widest">Sentiment de Marché</p>
        <p class="text-[10px] text-slate-400">{{ dateAffichee }}</p>
      </div>
      <div v-if="chargement" class="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
    </div>

    <p v-if="erreur" class="text-xs text-red-400">Données indisponibles</p>

    <template v-else-if="data">
      <div class="space-y-2.5 text-xs">
        <!-- USA -->
        <div>
          <p class="text-slate-500 mb-0.5">🇺🇸 USA</p>
          <div v-for="e in data.usa" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- EUROPE -->
        <div>
          <p class="text-slate-500 mb-0.5">🇪🇺 EUROPE</p>
          <div v-for="e in data.europe" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- MATIÈRES PREMIÈRES -->
        <div>
          <p class="text-slate-500 mb-0.5">⛏️ MATIÈRES PREMIÈRES</p>
          <div v-for="e in data.matieres_premieres" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- CRYPTOS -->
        <div>
          <p class="text-slate-500 mb-0.5">₿ CRYPTOS</p>
          <div v-for="e in data.cryptos" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="text-slate-400 tabular-nums text-[10px]">{{ formatPrix(e.prix) }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- VIX -->
        <div v-if="data.vix != null" class="pt-2 border-t border-white/10 flex items-center gap-2">
          <span class="text-slate-400">VIX {{ data.vix.toFixed(1) }}</span>
          <span
            class="font-semibold"
            :class="data.vix >= 30 ? 'text-red-400' : data.vix >= 20 ? 'text-orange-400' : 'text-emerald-400'"
          >
            {{ data.vix >= 30 ? '⚠ Peur' : data.vix >= 20 ? '⚡ Volatil' : '✓ Stable' }}
          </span>
        </div>
      </div>
    </template>

    <!-- Skeleton si premier chargement -->
    <div v-else class="space-y-2 animate-pulse">
      <div v-for="i in 8" :key="i" class="h-3 rounded bg-white/10" :style="{ width: `${55 + (i % 3) * 15}%` }" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { formatParis } from '@/utils/date'
import { useSentimentStore } from '@/stores/sentiment.store'

const store = useSentimentStore()
const { data, chargement, erreur } = storeToRefs(store)

const dateAffichee = computed(() => {
  if (!data.value) return ''
  return formatParis(new Date(data.value.date), {
    day: '2-digit', month: '2-digit', year: 'numeric',
  })
})

function bille(v: number): string {
  if (v > 0.3) return '🟢'
  if (v < -0.3) return '🔴'
  return '🔵'
}

function couleur(v: number): string {
  if (v > 0) return 'text-emerald-400'
  if (v < 0) return 'text-red-400'
  return 'text-slate-400'
}

function formatPrix(p: number): string {
  return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(p) + ' $'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
