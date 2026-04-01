<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div class="rounded-xl border border-white/10 p-5 w-[96vw] h-[92vh] flex flex-col gap-3" style="background: #0d1117;">

      <!-- Header -->
      <div class="flex items-center justify-between flex-shrink-0">
        <h2 class="text-lg font-bold">⚡ Analyse Straddle</h2>
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
          @click="onglet = tab.id as OngletStr"
        >{{ tab.label }}</button>
      </div>

      <!-- Onglet Performance -->
      <div v-if="onglet === 'perf'" class="flex-1 min-h-0 overflow-auto flex flex-col gap-3">
        <p class="text-xs text-gray-500 flex-shrink-0">
          Le Straddle prend des positions opposées (LONG + SHORT) simultanées. Un seul verdict = jambe gagnante.
        </p>
        <AnalysePerfBloc
          :stats="statsPerf"
          :tranches="straddleStats.tranches.value"
          :loss-rate-reel="straddleStats.lossRateReel.value"
          :sample-size="straddleStats.sampleSize.value"
          :k-values="straddleStats.kValues"
          :tableau-pertes="straddleStats.tableauPertes.value"
          :analyse-proba="straddleStats.analyseProba.value"
        >
          <template #gauche>
            <!-- Tranches -->
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
                  <tr v-for="t in straddleStats.tranches.value" :key="t.label" class="border-b border-white/5">
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

            <!-- Par asset -->
            <div>
              <h3 class="section-title">Par asset</h3>
              <div class="grid grid-cols-2 gap-2">
                <div v-for="a in straddleStats.parAsset.value" :key="a.asset" class="kpi-card">
                  <div class="flex justify-between mb-1">
                    <span class="text-xs font-bold px-1.5 py-0.5 rounded-full bg-yellow-900/60 text-yellow-300">{{ a.asset }}</span>
                    <span class="text-gray-500 text-xs">{{ a.total }}</span>
                  </div>
                  <div class="text-xs">Win : <span class="font-bold" :class="a.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ a.winPct }}%</span></div>
                  <div class="text-xs">R : <span class="font-bold" :class="a.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ a.rMoyen }}</span></div>
                </div>
              </div>
            </div>
          </template>
        </AnalysePerfBloc>
      </div>

      <!-- Onglet Créneaux -->
      <div v-if="onglet === 'creneaux'" class="flex-1 overflow-auto">
        <RouterLink
          to="/straddle"
          class="block mb-4 text-center py-2.5 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition text-sm"
          @click="$emit('close')"
        >
          → Voir les créneaux &amp; backtest horaire complet
        </RouterLink>
        <StraddleCreneauxTable />
      </div>

      <!-- Onglet Réglages -->
      <div v-if="onglet === 'reglages'" class="flex-1 overflow-auto">
        <StraddleParamsPanel
          v-model="strParams"
          :has-resultats="false"
          :chargement-llm="false"
          :suggestion="null"
          @relancer="() => {}"
          @optimiser="() => {}"
          @params-saved="() => {}"
        />
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Ref } from 'vue'
import type { Signal } from '@/services/api.service'
import { RouterLink } from 'vue-router'
import { useStraddleStats } from '@/composables/useStraddleStats'
import AnalysePerfBloc from '@/components/common/AnalysePerfBloc.vue'
import StraddleParamsPanel from '@/components/common/StraddleParamsPanel.vue'
import StraddleCreneauxTable from '@/components/common/StraddleCreneauxTable.vue'
import type { StraddleParams } from '@/components/common/StraddleParamsPanel.vue'

type OngletStr = 'perf' | 'creneaux' | 'reglages'

const TABS = [
  { id: 'perf',     label: '📊 Performance' },
  { id: 'creneaux', label: '🕐 Créneaux horaires' },
  { id: 'reglages', label: '⚙️ Paramètres' },
]

const props = defineProps<{
  open: boolean
  signaux: Signal[]
}>()
defineEmits<{ close: [] }>()

const onglet = ref<OngletStr>('perf')

const straddleStats = useStraddleStats(computed(() => props.signaux) as Ref<Signal[]>)

const statsPerf = computed(() => {
  const s = straddleStats.stats.value
  return {
    total: s.total, winPct: s.winPct, rMoyen: s.rMoyen,
    gain: s.gain, sl: s.sl, tauxSL: s.tauxSL,
  }
})

const strParams = ref<StraddleParams>({
  atr_periode: 14, seuil_atr: 1.5,
  tp_mult_1: 2.0, tp_mult_2: 3.0, tp_mult_3: 5.0,
  sl_mult: 0.5, trailing_atr: 1.5, vente_partielle: 1,
})
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-gray-400 mb-2 uppercase tracking-wide; }
</style>
