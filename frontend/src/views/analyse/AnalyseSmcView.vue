<template>
  <AnalysePageShell
    titre="📊 Analyse SMC"
    retour-label="SMC"
    retour-route="/smc"
    :synthese="bandeau"
  >
    <!-- ═══ 1. Performance ═══ -->
    <section class="flex flex-col gap-3">
      <h2 class="section-h">📊 Performance</h2>
      <AnalysePerfBloc
        :stats="statsPerf"
        :tranches="smcStats.tranches.value"
        :loss-rate-reel="smcStats.lossRateReel.value"
        :sample-size="smcStats.sampleSize.value"
        :k-values="smcStats.kValues"
        :tableau-pertes="smcStats.tableauPertes.value"
        :analyse-proba="smcStats.analyseProba.value"
      >
        <template #gauche>
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
                  <td class="py-1 text-right font-bold" :class="t.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ t.rMoyen }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div>
            <h3 class="section-title">Par timeframe</h3>
            <div class="grid grid-cols-3 gap-2">
              <div v-for="tf in smcStats.parTimeframe.value" :key="tf.tf" class="kpi-card">
                <div class="flex justify-between mb-1">
                  <span class="text-xs font-bold px-1.5 py-0.5 rounded-full bg-blue-900/60 text-blue-300">{{ tf.tf }}</span>
                  <span class="text-white text-xs">{{ tf.total }}</span>
                </div>
                <div class="text-xs">Win : <span class="font-bold" :class="tf.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ tf.winPct }}%</span></div>
                <div class="text-xs">R : <span class="font-bold" :class="tf.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ tf.rMoyen }}</span></div>
              </div>
            </div>
          </div>
        </template>
      </AnalysePerfBloc>
    </section>

    <!-- ═══ 2. IA & LLM ═══ -->
    <section class="flex flex-col gap-3">
      <h2 class="section-h">🤖 IA &amp; LLM</h2>
      <div class="grid grid-cols-3 gap-3 max-w-2xl">
        <div class="kpi-card text-center">
          <p class="text-xl font-bold text-purple-400">{{ smcStats.stats.value.convictionMoyenne }}</p>
          <p class="text-xs text-white mt-1">Conviction LLM moy.</p>
        </div>
        <div class="kpi-card text-center">
          <p class="text-xl font-bold text-blue-400">{{ smcStats.stats.value.tauxFiltrage }}%</p>
          <p class="text-xs text-white mt-1">Filtrés par LLM</p>
        </div>
        <div class="kpi-card">
          <div class="flex gap-3 h-full items-center justify-center">
            <div class="text-center">
              <p class="text-lg font-bold text-emerald-400">{{ smcStats.stats.value.longs }}</p>
              <p class="text-xs text-white">📈 LONG</p>
            </div>
            <div class="text-center">
              <p class="text-lg font-bold text-red-400">{{ smcStats.stats.value.shorts }}</p>
              <p class="text-xs text-white">📉 SHORT</p>
            </div>
          </div>
        </div>
      </div>
      <div v-if="smcStats.stats.value.derniersLlm.length > 0" class="flex flex-col gap-2">
        <p class="text-xs font-semibold text-white uppercase tracking-wider">Derniers avis LLM</p>
        <div
          v-for="s in smcStats.stats.value.derniersLlm" :key="s.id"
          class="flex items-start gap-3 rounded-lg px-3 py-2 text-xs"
          :class="s.llm_valide === 1 ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'"
        >
          <span class="shrink-0 font-bold text-white">{{ s.asset }} {{ s.timeframe }}</span>
          <span class="shrink-0" :class="s.llm_valide === 1 ? 'text-emerald-400' : 'text-red-400'">
            {{ s.llm_valide === 1 ? '✅' : '🚫' }} {{ s.llm_conviction ?? '—' }}/100
          </span>
          <span class="text-white truncate">{{ s.llm_raison ?? '—' }}</span>
        </div>
      </div>
      <p v-else class="text-center text-white text-xs py-2">Aucun signal SMC avec données LLM</p>
    </section>

    <!-- ═══ 3. Paramètres ═══ -->
    <section class="flex flex-col gap-2">
      <h2 class="section-h">⚙️ Paramètres</h2>
      <SmcParamsPanel v-model="smcParams" @params-saved="() => {}" />
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
import SmcParamsPanel from '@/components/common/SmcParamsPanel.vue'
import type { SmcParams } from '@/components/common/SmcParamsPanel.vue'

const signaux = ref<Signal[]>([])
onMounted(async () => {
  try {
    const data = await apiService.getSignaux(500)
    signaux.value = data.filter(s => s.strategie.toLowerCase() === 'smc' || s.strategie.toLowerCase() === 'smcdirectionnel')
  } catch { signaux.value = [] }
})

const smcStats = useSmcStats(computed(() => signaux.value) as Ref<Signal[]>)
const statsPerf = computed(() => {
  const s = smcStats.stats.value
  return { total: s.total, winPct: s.winPct, rMoyen: s.rMoyen, gain: s.gain, sl: s.sl, tauxSL: s.tauxSL }
})

const bandeau = computed(() => {
  const s = smcStats.stats.value
  return [
    { label: 'clôturés', valeur: s.total },
    { label: 'win rate', valeur: `${s.winPct}%`, classe: s.winPct >= 50 ? 'text-emerald-400' : 'text-red-400' },
    { label: 'R moyen', valeur: `${s.rMoyen}R`, classe: s.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400' },
    { label: 'conviction moy.', valeur: s.convictionMoyenne ?? '—', classe: 'text-purple-400' },
  ]
})

const smcParams = ref<SmcParams>({
  atr_periode: 14, score_min: 70,
  atr_tp1: 1.5, atr_tp2: 2.5, atr_tp3: 4.0, atr_sl: 0.8,
})
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-white mb-2 uppercase tracking-wide; }
.section-h    { @apply text-sm font-bold text-white uppercase tracking-wider border-b border-white/10 pb-1.5; }
</style>
