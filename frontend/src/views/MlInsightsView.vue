<template>
  <div class="space-y-6">
    <!-- En-tête -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">🤖 ML Insights</h1>
      <button
        class="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-sm font-semibold transition-colors disabled:opacity-50"
        :disabled="store.chargement"
        @click="rafraichir"
      >
        {{ store.chargement ? '⏳ Chargement…' : '↺ Rafraîchir' }}
      </button>
    </div>

    <!-- Onglets -->
    <div class="flex gap-2 border-b border-white/10 pb-1">
      <button
        v-for="o in onglets" :key="o.id"
        class="px-4 py-2 rounded-t text-sm font-medium transition-colors"
        :class="onglet === o.id ? 'bg-white/10 text-white' : 'text-gray-400 hover:text-white'"
        @click="onglet = o.id"
      >{{ o.label }}</button>
    </div>

    <!-- Onglet 1 : Performance par stratégie -->
    <template v-if="onglet === 'perf'">
      <div v-if="!store.analyse" class="glass-card p-6 text-center text-gray-400">
        Aucune donnée disponible — les stats apparaissent dès les premiers trades clôturés.
      </div>
      <div v-else class="space-y-4">

        <!-- === SMC Directionnel === -->
        <div class="glass-card overflow-hidden">
          <div class="px-4 py-3 border-b border-blue-500/30 bg-blue-500/10 flex items-center gap-3">
            <span class="w-2.5 h-2.5 rounded-full bg-blue-400 shrink-0"></span>
            <span class="font-semibold text-blue-200">SMC Directionnel</span>
            <span v-if="store.analyse.smc" class="ml-auto flex items-center gap-1 text-xs text-gray-400">
              {{ store.analyse.smc.global.nb_gagnants }} / {{ store.analyse.smc.global.nb_trades }} trades
              <TooltipIcon>Source : smc_feedback — mémoire apprenante LLM (INVALIDE = -1R)</TooltipIcon>
            </span>
          </div>
          <div v-if="store.analyse.smc" class="p-4 grid grid-cols-1 md:grid-cols-4 gap-6">
            <div class="space-y-1">
              <div class="text-4xl font-bold" :class="store.analyse.smc.global.win_rate >= 55 ? 'text-emerald-400' : store.analyse.smc.global.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.smc.global.win_rate.toFixed(0) }}%</div>
              <div class="text-xs text-gray-400">Win Rate</div>
              <div class="mt-3 text-2xl font-bold" :class="store.analyse.smc.global.pnl_r_moyen >= 1 ? 'text-emerald-400' : store.analyse.smc.global.pnl_r_moyen >= 0.5 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.smc.global.pnl_r_moyen.toFixed(2) }}R</div>
              <div class="text-xs text-gray-400">R:R moyen</div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Par score</p>
              <div v-for="t in store.analyse.smc.par_score" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-12 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full" :class="t.win_rate >= 55 ? 'bg-emerald-500' : t.win_rate >= 45 ? 'bg-yellow-500' : 'bg-red-500'" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Kill Zone</p>
              <div v-for="t in store.analyse.smc.par_kill_zone" :key="t.tranche" class="flex items-center justify-between gap-2">
                <span class="text-xs text-gray-400 shrink-0">{{ t.tranche }}</span>
                <span class="text-sm font-semibold" :class="t.win_rate >= 55 ? 'text-emerald-400' : t.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ t.win_rate.toFixed(0) }}% <span class="text-xs text-gray-500 font-normal">({{ t.nb_trades }})</span></span>
              </div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Confiance ML</p>
              <div v-for="t in store.analyse.smc.ml_correlation" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-12 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full bg-blue-500" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
          </div>
          <div v-else class="p-6 text-center text-gray-500 text-sm">Aucun trade SMC clôturé</div>
        </div>

        <!-- === Rockets === -->
        <div class="glass-card overflow-hidden">
          <div class="px-4 py-3 border-b border-orange-500/30 bg-orange-500/10 flex items-center gap-3">
            <span class="w-2.5 h-2.5 rounded-full bg-orange-400 shrink-0"></span>
            <span class="font-semibold text-orange-200">🚀 Rockets</span>
            <span v-if="store.analyse.rockets" class="ml-auto flex items-center gap-1 text-xs text-gray-400">
              {{ store.analyse.rockets.global.nb_gagnants }} / {{ store.analyse.rockets.global.nb_trades }} trades
              <TooltipIcon>Source : rockets_feedback — trades réels clôturés (invalides/expirés exclus)</TooltipIcon>
            </span>
          </div>
          <div v-if="store.analyse.rockets" class="p-4 grid grid-cols-1 md:grid-cols-3 gap-6">
            <div class="space-y-1">
              <div class="text-4xl font-bold" :class="store.analyse.rockets.global.win_rate >= 55 ? 'text-emerald-400' : store.analyse.rockets.global.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.rockets.global.win_rate.toFixed(0) }}%</div>
              <div class="text-xs text-gray-400">Win Rate</div>
              <div class="mt-3 text-2xl font-bold" :class="store.analyse.rockets.global.pnl_r_moyen >= 1 ? 'text-emerald-400' : store.analyse.rockets.global.pnl_r_moyen >= 0.5 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.rockets.global.pnl_r_moyen.toFixed(2) }}R</div>
              <div class="text-xs text-gray-400">R:R moyen</div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Par phase</p>
              <div v-for="t in store.analyse.rockets.par_phase" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-16 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full" :class="t.win_rate >= 55 ? 'bg-emerald-500' : t.win_rate >= 45 ? 'bg-yellow-500' : 'bg-red-500'" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Conviction LLM</p>
              <div v-for="t in store.analyse.rockets.conviction_llm" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-12 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full bg-orange-500" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
          </div>
          <div v-else class="p-6 text-center text-gray-500 text-sm">Aucun trade Rockets clôturé</div>
        </div>

        <!-- === Straddle Volatilité === -->
        <div class="glass-card overflow-hidden">
          <div class="px-4 py-3 border-b border-purple-500/30 bg-purple-500/10 flex items-center gap-3">
            <span class="w-2.5 h-2.5 rounded-full bg-purple-400 shrink-0"></span>
            <span class="font-semibold text-purple-200">⚡ Straddle Volatilité</span>
            <span v-if="store.analyse.straddle" class="ml-auto flex items-center gap-1 text-xs text-gray-400">
              {{ store.analyse.straddle.global.nb_gagnants }} / {{ store.analyse.straddle.global.nb_trades }} trades
              <TooltipIcon>Source : straddle_feedback — mémoire apprenante LLM (INVALIDE = -1R)</TooltipIcon>
            </span>
          </div>
          <div v-if="store.analyse.straddle" class="p-4 grid grid-cols-1 md:grid-cols-3 gap-6">
            <div class="space-y-1">
              <div class="text-4xl font-bold" :class="store.analyse.straddle.global.win_rate >= 55 ? 'text-emerald-400' : store.analyse.straddle.global.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.straddle.global.win_rate.toFixed(0) }}%</div>
              <div class="text-xs text-gray-400">Win Rate</div>
              <div class="mt-3 text-2xl font-bold" :class="store.analyse.straddle.global.pnl_r_moyen >= 1 ? 'text-emerald-400' : store.analyse.straddle.global.pnl_r_moyen >= 0.5 ? 'text-yellow-400' : 'text-red-400'">{{ store.analyse.straddle.global.pnl_r_moyen.toFixed(2) }}R</div>
              <div class="text-xs text-gray-400">R:R moyen</div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Par catégorie</p>
              <div v-for="t in store.analyse.straddle.par_categorie" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-20 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full" :class="t.win_rate >= 55 ? 'bg-emerald-500' : t.win_rate >= 45 ? 'bg-yellow-500' : 'bg-red-500'" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
            <div class="space-y-2">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-1">Score LLM</p>
              <div v-for="t in store.analyse.straddle.score_llm" :key="t.tranche" class="flex items-center gap-2">
                <span class="text-xs text-gray-400 w-12 shrink-0">{{ t.tranche }}</span>
                <div class="flex-1 h-1.5 rounded-full bg-white/10"><div class="h-1.5 rounded-full bg-purple-500" :style="`width:${Math.min(t.win_rate,100)}%`" /></div>
                <span class="text-xs text-gray-300 w-20 text-right shrink-0">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
              </div>
            </div>
          </div>
          <div v-else class="p-6 text-center text-gray-500 text-sm">Aucun trade Straddle clôturé</div>
        </div>

      </div>
    </template>

    <!-- Onglet 2 : Suggestions -->
    <template v-if="onglet === 'suggestions'">
      <div v-if="store.suggestions.length === 0" class="glass-card p-6 text-center text-gray-400">
        Aucune suggestion pour l'instant — il faut au moins 30 trades clôturés par stratégie.
      </div>
      <div v-else class="space-y-3">
        <div
          v-for="s in store.suggestions" :key="`${s.strategie}-${s.param_name}`"
          class="glass-card p-4 flex items-start justify-between gap-4"
        >
          <div class="flex-1 space-y-1">
            <div class="flex items-center gap-2">
              <span class="text-xs font-bold px-2 py-0.5 rounded" :class="badgeStrategie(s.strategie)">{{ s.strategie }}</span>
              <span class="text-sm font-semibold text-white">{{ s.param_name }}</span>
              <span class="text-xs text-gray-400">{{ s.valeur_actuelle }} → <span class="text-emerald-400 font-semibold">{{ s.valeur_suggeree }}</span></span>
            </div>
            <p class="text-xs text-gray-300">{{ s.justification }}</p>
            <div class="flex items-center gap-4 text-xs text-gray-500 mt-1">
              <span>+{{ s.gain_winrate_estime.toFixed(1) }}% WR estimé</span>
              <span>Confiance : <span :class="s.confiance >= 0.75 ? 'text-emerald-400' : s.confiance >= 0.6 ? 'text-yellow-400' : 'text-red-400'">{{ (s.confiance * 100).toFixed(0) }}%</span></span>
              <span>Base : {{ s.nb_samples_base }} trades</span>
            </div>
          </div>
          <div class="flex gap-2 shrink-0">
            <button
              class="px-3 py-1.5 rounded text-xs font-semibold bg-emerald-700 hover:bg-emerald-600 transition-colors disabled:opacity-50"
              :disabled="store.application"
              @click="appliquer(s)"
            >Appliquer</button>
            <button class="px-3 py-1.5 rounded text-xs font-semibold bg-white/5 hover:bg-white/10 transition-colors text-gray-400">Ignorer</button>
          </div>
        </div>
      </div>

      <!-- Historique -->
      <div v-if="store.historique.length > 0" class="mt-6">
        <h3 class="text-sm font-semibold text-gray-300 mb-2">Historique des suggestions appliquées</h3>
        <div class="glass-card overflow-hidden">
          <table class="w-full text-xs">
            <thead class="border-b border-white/10 text-gray-400">
              <tr>
                <th class="px-3 py-2 text-left">Stratégie</th>
                <th class="px-3 py-2 text-left">Paramètre</th>
                <th class="px-3 py-2 text-right">Avant</th>
                <th class="px-3 py-2 text-right">Après</th>
                <th class="px-3 py-2 text-right">Gain estimé</th>
                <th class="px-3 py-2 text-left">Date</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="h in store.historique" :key="h.id" class="border-b border-white/5 hover:bg-white/5">
                <td class="px-3 py-2">{{ h.strategie }}</td>
                <td class="px-3 py-2 text-gray-300">{{ h.param_name }}</td>
                <td class="px-3 py-2 text-right text-gray-400">{{ h.valeur_avant }}</td>
                <td class="px-3 py-2 text-right text-emerald-400 font-semibold">{{ h.valeur_apres }}</td>
                <td class="px-3 py-2 text-right text-blue-400">+{{ h.gain_winrate_estime.toFixed(1) }}%</td>
                <td class="px-3 py-2 text-gray-500">{{ h.appliquee_le }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Onglet 3 : Réentraînement -->
    <template v-if="onglet === 'retrain'">
      <MlRetrainPanel />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'
import type { SuggestionParams } from '@/services/api.ml_insights'
import MlRetrainPanel from '@/components/common/MlRetrainPanel.vue'
import TooltipIcon from '@/components/common/TooltipIcon.vue'

const store = useMlInsightsStore()
const onglet = ref<'perf' | 'suggestions' | 'retrain'>('perf')

const onglets: { id: 'perf' | 'suggestions' | 'retrain'; label: string }[] = [
  { id: 'perf',        label: '📊 Performance par stratégie' },
  { id: 'suggestions', label: '💡 Suggestions de paramètres' },
  { id: 'retrain',     label: '🔁 Réentraînement' },
]

function badgeStrategie(s: string) {
  return s === 'SMC'      ? 'bg-blue-800 text-blue-200'    :
         s === 'ROCKETS'  ? 'bg-orange-800 text-orange-200' :
                            'bg-purple-800 text-purple-200'
}

async function rafraichir() {
  await Promise.all([store.chargerStats(), store.chargerSuggestions()])
}

async function appliquer(s: SuggestionParams) {
  await store.appliquer(s)
}

onMounted(() => {
  rafraichir()
  store.chargerDernierRetrain()
})
</script>
