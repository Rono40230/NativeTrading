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
      <div v-else class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <StatCard titre="SMC Directionnel"     :stats="store.analyse.smc?.global"     />
        <StatCard titre="Rockets"              :stats="store.analyse.rockets"          />
        <StatCard titre="Straddle Volatilité"  :stats="store.analyse.straddle"        />
      </div>

      <!-- Détail SMC -->
      <template v-if="store.analyse?.smc">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
          <!-- Par score -->
          <div class="glass-card p-4 space-y-2">
            <h3 class="text-sm font-semibold text-gray-300">SMC — Win Rate par score</h3>
            <div v-for="t in store.analyse.smc.par_score" :key="t.tranche" class="flex items-center gap-2">
              <span class="text-xs text-gray-400 w-16">{{ t.tranche }}</span>
              <div class="flex-1 h-2 rounded-full bg-white/10">
                <div class="h-2 rounded-full" :class="t.win_rate >= 55 ? 'bg-emerald-500' : t.win_rate >= 45 ? 'bg-yellow-500' : 'bg-red-500'" :style="`width:${Math.min(t.win_rate, 100)}%`" />
              </div>
              <span class="text-xs font-medium w-16 text-right">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
            </div>
          </div>

          <!-- Kill Zone -->
          <div class="glass-card p-4 space-y-2">
            <h3 class="text-sm font-semibold text-gray-300">SMC — Kill Zone</h3>
            <div v-for="t in store.analyse.smc.par_kill_zone" :key="t.tranche" class="flex items-center gap-2">
              <span class="text-xs text-gray-400 w-28">{{ t.tranche }}</span>
              <span class="text-sm font-semibold" :class="t.win_rate >= 55 ? 'text-emerald-400' : t.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">{{ t.win_rate.toFixed(0) }}%</span>
              <span class="text-xs text-gray-500">({{ t.nb_trades }} trades)</span>
            </div>
          </div>

          <!-- Corrélation ML -->
          <div class="glass-card p-4 space-y-2">
            <h3 class="text-sm font-semibold text-gray-300">SMC — Confiance ML vs Win Rate</h3>
            <div v-for="t in store.analyse.smc.ml_correlation" :key="t.tranche" class="flex items-center gap-2">
              <span class="text-xs text-gray-400 w-16">{{ t.tranche }}</span>
              <div class="flex-1 h-2 rounded-full bg-white/10">
                <div class="h-2 rounded-full bg-blue-500" :style="`width:${Math.min(t.win_rate, 100)}%`" />
              </div>
              <span class="text-xs font-medium w-16 text-right">{{ t.win_rate.toFixed(0) }}% ({{ t.nb_trades }})</span>
            </div>
          </div>
        </div>
      </template>
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
      <div class="glass-card p-6 space-y-6">
        <!-- Baseline & déclencheur -->
        <div class="flex flex-col md:flex-row md:items-center gap-4">
          <div class="flex-1 space-y-1">
            <p class="text-sm font-semibold text-gray-200">Réentraîner le pipeline ML maintenant</p>
            <p class="text-xs text-gray-400">Lance un entraînement walk-forward sur toutes les combinaisons asset × timeframe disponibles en base. Un rollback automatique est déclenché si l'accuracy chute de plus de 2 pts.</p>
          </div>
          <button
            class="shrink-0 px-5 py-2.5 rounded-lg font-semibold text-sm transition-colors"
            :class="store.retrainState?.en_cours
              ? 'bg-gray-700 text-gray-400 cursor-not-allowed'
              : 'bg-blue-600 hover:bg-blue-500 text-white'"
            :disabled="store.retrainState?.en_cours"
            @click="store.declencherRetrain()"
          >
            {{ store.retrainState?.en_cours ? '⏳ En cours…' : '🔁 Lancer le réentraînement' }}
          </button>
        </div>

        <!-- Statut actuel -->
        <div v-if="store.retrainState?.job_id" class="space-y-3">
          <div class="flex items-center gap-3">
            <span class="text-xs text-gray-500">Job {{ store.retrainState.job_id }}</span>
            <span
              class="text-xs font-bold px-2 py-0.5 rounded"
              :class="store.retrainState.en_cours
                ? 'bg-blue-800 text-blue-200'
                : store.retrainState.rolled_back
                  ? 'bg-yellow-800 text-yellow-200'
                  : 'bg-emerald-800 text-emerald-200'"
            >
              {{ store.retrainState.en_cours ? 'En cours' : store.retrainState.rolled_back ? 'Rollback' : 'Terminé' }}
            </span>
          </div>

          <!-- Message -->
          <p class="text-sm text-gray-300">{{ store.retrainState.message }}</p>

          <!-- Accuracy avant / après -->
          <div v-if="store.retrainState.accuracy_avant > 0" class="flex items-center gap-6 text-sm">
            <div class="text-center">
              <p class="text-xs text-gray-500 mb-1">Accuracy avant</p>
              <p class="text-lg font-bold text-gray-200">{{ (store.retrainState.accuracy_avant * 100).toFixed(1) }}%</p>
            </div>
            <span class="text-gray-600 text-xl">→</span>
            <div v-if="store.retrainState.accuracy_apres !== null" class="text-center">
              <p class="text-xs text-gray-500 mb-1">Accuracy après</p>
              <p
                class="text-lg font-bold"
                :class="store.retrainState.accuracy_apres >= store.retrainState.accuracy_avant
                  ? 'text-emerald-400'
                  : 'text-red-400'"
              >{{ (store.retrainState.accuracy_apres * 100).toFixed(1) }}%</p>
            </div>
            <div v-else class="text-center">
              <p class="text-xs text-gray-500 mb-1">Accuracy après</p>
              <p class="text-gray-500 text-lg">…</p>
            </div>
          </div>
        </div>

        <div v-else class="text-gray-500 text-sm">Aucun réentraînement effectué dans cette session.</div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'
import type { SuggestionParams } from '@/services/api.ml_insights'
import StatCard from '@/components/common/StatCard.vue'

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
