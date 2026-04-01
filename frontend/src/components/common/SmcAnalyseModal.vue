<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div class="rounded-xl border border-white/10 p-5 w-[96vw] h-[92vh] flex flex-col gap-3" style="background: #0d1117;">

      <!-- Header -->
      <div class="flex items-center justify-between flex-shrink-0">
        <h2 class="text-lg font-bold">🧠 Analyse SMC Directionnel</h2>
        <button class="text-gray-400 hover:text-white text-xl leading-none" @click="$emit('close')">×</button>
      </div>

      <!-- Onglets -->
      <div class="flex gap-1 flex-shrink-0">
        <button
          v-for="tab in TABS" :key="tab.id"
          class="px-4 py-1.5 rounded-lg text-xs font-semibold transition-all border"
          :class="onglet === tab.id
            ? 'bg-white/10 border-white/20 text-white'
            : 'border-transparent text-gray-500 hover:text-gray-300'"
          @click="onglet = tab.id as OngletSmc"
        >{{ tab.label }}</button>
      </div>

      <!-- Onglet Performance -->
      <div v-if="onglet === 'perf'" class="flex-1 min-h-0 overflow-auto flex flex-col gap-3">
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
            <!-- Tranches de score -->
            <div>
              <h3 class="section-title">Par tranche de score</h3>
              <table class="w-full text-xs">
                <thead>
                  <tr class="text-gray-500 border-b border-white/10">
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
                    <td class="py-1 text-right text-gray-400">{{ t.total }}</td>
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

            <!-- Par timeframe -->
            <div>
              <h3 class="section-title">Par timeframe</h3>
              <div class="grid grid-cols-2 gap-2">
                <div v-for="tf in smcStats.parTimeframe.value" :key="tf.tf" class="kpi-card">
                  <div class="flex justify-between mb-1">
                    <span class="text-xs font-bold px-1.5 py-0.5 rounded-full bg-blue-900/60 text-blue-300">{{ tf.tf }}</span>
                    <span class="text-gray-500 text-xs">{{ tf.total }}</span>
                  </div>
                  <div class="text-xs">Win : <span class="font-bold" :class="tf.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ tf.winPct }}%</span></div>
                  <div class="text-xs">R : <span class="font-bold" :class="tf.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ tf.rMoyen }}</span></div>
                </div>
              </div>
            </div>
          </template>
        </AnalysePerfBloc>
      </div>

      <!-- Onglet IA -->
      <div v-if="onglet === 'ia'" class="flex-1 overflow-auto">
        <div class="space-y-3">
          <!-- Conviction LLM -->
          <div class="grid grid-cols-3 gap-3">
            <div class="kpi-card text-center">
              <p class="text-xl font-bold text-purple-400">{{ smcStats.stats.value.convictionMoyenne }}</p>
              <p class="text-xs text-gray-400 mt-1">Conviction LLM moy.</p>
            </div>
            <div class="kpi-card text-center">
              <p class="text-xl font-bold text-blue-400">{{ smcStats.stats.value.tauxFiltrage }}%</p>
              <p class="text-xs text-gray-400 mt-1">Filtrés par LLM</p>
            </div>
            <div class="kpi-card">
              <div class="flex gap-3 h-full items-center justify-center">
                <div class="text-center">
                  <p class="text-lg font-bold text-emerald-400">{{ smcStats.stats.value.longs }}</p>
                  <p class="text-xs text-gray-400">📈 LONG</p>
                </div>
                <div class="text-center">
                  <p class="text-lg font-bold text-red-400">{{ smcStats.stats.value.shorts }}</p>
                  <p class="text-xs text-gray-400">📉 SHORT</p>
                </div>
              </div>
            </div>
          </div>
          <!-- Derniers filtrages -->
          <div v-if="smcStats.stats.value.derniersLlm.length > 0" class="space-y-2">
            <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Derniers filtrages LLM</p>
            <div
              v-for="s in smcStats.stats.value.derniersLlm"
              :key="s.id"
              class="flex items-start gap-3 rounded-lg px-3 py-2 text-xs"
              :class="s.llm_valide === 1 ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'"
            >
              <span class="shrink-0 font-bold text-white">{{ s.asset }} {{ s.timeframe }}</span>
              <span class="shrink-0" :class="s.llm_valide === 1 ? 'text-emerald-400' : 'text-red-400'">
                {{ s.llm_valide === 1 ? '✅' : '🚫' }} {{ s.llm_conviction ?? '—' }}/100
              </span>
              <span class="text-gray-400 truncate">{{ s.llm_raison ?? '—' }}</span>
            </div>
          </div>
          <p v-else class="text-center text-gray-500 text-sm py-4">Aucun signal SMC avec données LLM</p>
        </div>
      </div>

      <!-- Onglet Réglages -->
      <div v-if="onglet === 'reglages'" class="flex-1 overflow-auto">
        <SmcParamsPanel v-model="smcParams" @params-saved="() => {}" />
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Ref } from 'vue'
import type { Signal } from '@/services/api.service'
import { useSmcStats } from '@/composables/useSmcStats'
import AnalysePerfBloc from '@/components/common/AnalysePerfBloc.vue'
import SmcParamsPanel from '@/components/common/SmcParamsPanel.vue'
import type { SmcParams } from '@/components/common/SmcParamsPanel.vue'

type OngletSmc = 'perf' | 'ia' | 'reglages'

const TABS = [
  { id: 'perf',     label: '📊 Performance' },
  { id: 'ia',       label: '🤖 IA & LLM' },
  { id: 'reglages', label: '⚙️ Paramètres' },
]

const props = defineProps<{
  open: boolean
  signaux: Signal[]
}>()
defineEmits<{ close: [] }>()

const onglet = ref<OngletSmc>('perf')

const smcStats = useSmcStats(computed(() => props.signaux) as Ref<Signal[]>)

const statsPerf = computed(() => {
  const s = smcStats.stats.value
  return {
    total: s.total, winPct: s.winPct, rMoyen: s.rMoyen,
    gain: s.gain, sl: s.sl, tauxSL: s.tauxSL,
  }
})

const smcParams = ref<SmcParams>({
  atr_periode: 14, score_min: 70,
  atr_tp1: 1.5, atr_tp2: 2.5, atr_tp3: 4.0, atr_sl: 0.8,
})
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-gray-400 mb-2 uppercase tracking-wide; }
</style>
