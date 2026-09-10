<template>
  <AnalysePageShell
    titre="📊 Analyse SMC"
    retour-label="SMC"
    retour-route="/smc"
  >
    <!-- ═══ RANGÉE 1 : dossier de décision | IA ═══ -->
    <div class="grid grid-cols-2 gap-4">
      <!-- 🎯 Dossier -->
      <section class="rounded-xl border border-blue-500/30 bg-blue-950/20 p-4 flex flex-col gap-4">
        <h2 class="text-sm font-bold text-blue-300 uppercase tracking-wider">🎯 Le dossier de décision</h2>

        <div class="grid grid-cols-4 gap-3">
          <div class="text-center">
            <div class="text-3xl font-bold text-white">{{ stats.total }}</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">clôturés</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold" :class="stats.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ stats.winPct }}%</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">win rate</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold" :class="stats.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR(stats.rMoyen) }}</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">R moyen</div>
          </div>
          <div class="text-center">
            <div class="text-3xl font-bold text-red-400">{{ stats.tauxSL }}%</div>
            <div class="text-[10px] uppercase tracking-wide text-white mt-1">loss rate</div>
          </div>
        </div>

        <!-- LA dimension de la surveillance §2 : par timeframe -->
        <table v-if="smcStats.parTimeframe.value.length" class="w-full text-sm border-collapse">
          <thead>
            <tr class="text-white border-b border-white/15">
              <th class="py-2 text-left text-xs uppercase tracking-wide">Timeframe</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">Trades</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">R moyen</th>
              <th class="py-2 text-right text-xs uppercase tracking-wide">Win rate</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="tf in smcStats.parTimeframe.value" :key="tf.tf" class="border-b border-white/5">
              <td class="py-2 text-white font-semibold">{{ tf.tf }}</td>
              <td class="py-2 text-right text-white">{{ tf.total }}</td>
              <td class="py-2 text-right text-lg font-bold" :class="tf.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR(tf.rMoyen) }}</td>
              <td class="py-2 text-right" :class="tf.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ tf.winPct }}%</td>
            </tr>
          </tbody>
        </table>
      </section>

      <!-- 🤖 IA & LLM -->
      <section class="rounded-xl border border-purple-500/25 bg-purple-950/10 p-4 flex flex-col gap-3">
        <h2 class="text-sm font-bold text-purple-300 uppercase tracking-wider">🤖 IA &amp; LLM</h2>
        <div class="grid grid-cols-3 gap-3">
          <div class="kpi-card text-center">
            <p class="text-xl font-bold text-purple-400">{{ stats.convictionMoyenne }}</p>
            <p class="text-xs text-white mt-1">Conviction moy.</p>
          </div>
          <div class="kpi-card text-center">
            <p class="text-xl font-bold text-blue-400">{{ stats.tauxFiltrage }}%</p>
            <p class="text-xs text-white mt-1">Notés par LLM</p>
          </div>
          <div class="kpi-card">
            <div class="flex gap-3 h-full items-center justify-center">
              <div class="text-center">
                <p class="text-lg font-bold text-emerald-400">{{ stats.longs }}</p>
                <p class="text-xs text-white">📈 LONG</p>
              </div>
              <div class="text-center">
                <p class="text-lg font-bold text-red-400">{{ stats.shorts }}</p>
                <p class="text-xs text-white">📉 SHORT</p>
              </div>
            </div>
          </div>
        </div>
        <div v-if="stats.derniersLlm.length" class="flex flex-col gap-2 overflow-y-auto max-h-64 pr-1">
          <p class="text-xs font-semibold text-white uppercase tracking-wider">Derniers avis LLM</p>
          <div
            v-for="s in stats.derniersLlm" :key="s.id"
            class="flex items-start gap-3 rounded-lg px-3 py-2 text-xs bg-white/5 border border-white/10"
          >
            <span class="shrink-0 font-bold text-white">{{ s.asset }} {{ s.timeframe }}</span>
            <span class="shrink-0 font-bold" :class="classeConviction(s.llm_conviction)">
              🎯 {{ s.llm_conviction }}/100
            </span>
            <span class="text-white truncate" :title="s.llm_raison ?? ''">{{ s.llm_raison ?? '—' }}</span>
          </div>
        </div>
        <p v-else class="text-center text-white text-xs py-2">Aucun avis LLM encore — le rail conviction alimentera cette liste.</p>
      </section>
    </div>

    <!-- ═══ RANGÉE 2 : détails de performance ═══ (les réglages réels vivent
         dans les modales de la carte SMC du dashboard — le panneau fantôme
         smc_params a été purgé le 09/09) -->
    <section class="rounded-xl border border-white/10 bg-white/[0.03] p-4 flex flex-col gap-4">
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-sm font-bold text-white uppercase tracking-wider">📋 Détails de performance</span>
        <span class="ml-auto text-xs text-white">{{ stats.total }} trades · {{ stats.gain }} TP · {{ stats.sl }} SL</span>
      </div>
      <div>
        <h3 class="section-title">Par tranche de score</h3>
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white border-b border-white/10">
              <th class="py-1 text-left">Score</th>
              <th class="py-1 text-right">Nb</th>
              <th class="py-1 text-right text-emerald-400">TP1</th>
              <th class="py-1 text-right text-emerald-300">TP2</th>
              <th class="py-1 text-right text-emerald-200">TP3</th>
              <th class="py-1 text-right text-red-400">SL</th>
              <th class="py-1 text-right">Win%</th>
              <th class="py-1 text-right">R</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t in smcStats.tranches.value" :key="t.label" class="border-b border-white/5">
              <td class="py-1 font-mono text-white">{{ t.label }}</td>
              <td class="py-1 text-right text-white">{{ t.total }}</td>
              <td class="py-1 text-right text-emerald-400">{{ t.tp1 }}</td>
              <td class="py-1 text-right text-emerald-300">{{ t.tp2 }}</td>
              <td class="py-1 text-right text-emerald-200">{{ t.tp3 }}</td>
              <td class="py-1 text-right text-red-400">{{ t.sl }}</td>
              <td class="py-1 text-right font-bold" :class="t.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ t.winPct }}%</td>
              <td class="py-1 text-right font-bold" :class="t.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR(t.rMoyen) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div>
        <AnalysePerfBloc
          :stats="statsPerf"
          :tranches="smcStats.tranches.value"
          :loss-rate-reel="smcStats.lossRateReel.value"
          :sample-size="smcStats.sampleSize.value"
          :k-values="smcStats.kValues"
          :tableau-pertes="smcStats.tableauPertes.value"
          :analyse-proba="smcStats.analyseProba.value"
          sans-kpis
        >
          <template #gauche><span /></template>
        </AnalysePerfBloc>
      </div>
    </section>
  </AnalysePageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { Ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { useSmcStats } from '@/composables/useSmcStats'
import AnalysePageShell from '@/components/common/AnalysePageShell.vue'
import AnalysePerfBloc from '@/components/common/AnalysePerfBloc.vue'

const signaux = ref<Signal[]>([])
onMounted(async () => {
  try {
    const data = await apiService.getSignaux(500)
    signaux.value = data.filter(s => s.strategie?.toLowerCase().startsWith('smc'))
  } catch { signaux.value = [] }
})

const smcStats = useSmcStats(computed(() => signaux.value) as Ref<Signal[]>)
const stats = computed(() => smcStats.stats.value)
const statsPerf = computed(() => ({
  total: stats.value.total, winPct: stats.value.winPct, rMoyen: stats.value.rMoyen,
  gain: stats.value.gain, sl: stats.value.sl, tauxSL: stats.value.tauxSL,
}))

function fmtR(v: number): string {
  return `${v >= 0 ? '+' : ''}${v.toFixed(2)}R`
}

// Bandes de conviction alignées sur seuil_confiance_smc (0,4 → 40/100).
function classeConviction(c: number | null): string {
  if (c == null) return 'text-white'
  if (c >= 70) return 'text-emerald-400'
  if (c >= 40) return 'text-amber-400'
  return 'text-red-400'
}
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-white mb-2 uppercase tracking-wide; }
</style>
