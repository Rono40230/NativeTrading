<template>
  <!-- Classement décisionnel d'une dimension (TF, asset, événement) : le
       verdict « les plus intéressants » — tri par R moyen encaissé/trade,
       garde-fous effectif (règle des 30) et WR. -->
  <div class="glass-card p-3">
    <div class="flex items-center gap-2 mb-2 flex-wrap">
      <p class="text-xs font-semibold text-white">🏆 {{ titre }}</p>
      <span class="text-[10px] text-white/50" title="Tri : R moyen encaissé par clôture — la contribution par trade, pas le volume">tri : R moyen encaissé/trade</span>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
      <div v-for="sens in sections" :key="sens.key">
        <p class="text-[10px] font-semibold uppercase tracking-wider mb-1" :class="sens.couleur">{{ sens.label }}</p>
        <div class="flex flex-col gap-1">
          <div v-for="(l, i) in sens.lignes" :key="l.label"
               class="flex items-center gap-2 text-[11px] rounded-lg px-2 py-1"
               :class="l.n < seuil ? 'bg-white/[0.02] text-white/50' : 'bg-white/5'">
            <span class="w-4 text-right text-white/40 font-mono shrink-0">{{ sens.key === 'top' ? i + 1 : '−' + (i + 1) }}</span>
            <span class="font-mono font-bold truncate shrink-0" :class="l.n < seuil ? 'text-white/50' : 'text-white'">{{ l.label }}</span>
            <span v-if="l.n < seuil" class="text-[9px] px-1 py-0.5 rounded-full border border-white/10 text-white/40 shrink-0"
                  :title="`${l.n} clôtures — règle des 30 non atteinte`">{{ l.n }}</span>
            <span class="ml-auto font-mono tabular-nums shrink-0" :class="l.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR(l.rMoyen) }}</span>
            <span class="font-mono text-white/50 tabular-nums w-10 text-right shrink-0" title="Win rate ($ > 0)">{{ Math.round(l.wr * 100) }} %</span>
            <span class="font-mono text-white/60 tabular-nums w-16 text-right shrink-0" title="Contribution $ totale">{{ fmtDollars(l.dollars) }}</span>
          </div>
          <p v-if="!sens.lignes.length" class="text-[11px] text-white/40 px-2 py-1">—</p>
        </div>
      </div>
    </div>
    <p class="text-[9px] text-white/40 mt-2">Colonnes : R moyen encaissé/trade · WR ($&gt;0) · Σ $. Gris = effectif &lt; {{ seuil }} (non significatif).</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { CategorieAnalyse } from '@/composables/useAnalyses'

const props = defineProps<{ titre: string; categories: CategorieAnalyse[]; seuil?: number }>()

const seuil = computed(() => props.seuil ?? 30)

interface Ligne { label: string; n: number; rMoyen: number; wr: number; dollars: number }

const classees = computed<Ligne[]>(() =>
  props.categories
    .filter(c => c.n > 0)
    .map(c => ({ label: c.label, n: c.n, rMoyen: c.r / c.n, wr: c.wr, dollars: c.dollars }))
    .sort((a, b) => b.rMoyen - a.rMoyen),
)

const sections = computed(() => [
  { key: 'top', label: '👍 Les plus intéressants', couleur: 'text-emerald-400', lignes: classees.value.slice(0, 5) },
  { key: 'flop', label: '👎 Les moins intéressants', couleur: 'text-red-400', lignes: classees.value.slice(-5).reverse() },
])

function fmtR(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2)} R`
}
function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
