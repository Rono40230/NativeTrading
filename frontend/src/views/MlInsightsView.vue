<template>
  <div class="h-full flex flex-col gap-3 overflow-hidden">
    <!-- En-tête -->
    <div class="shrink-0 flex items-center justify-between">
      <div class="space-y-0.5">
        <h1 class="text-xl font-bold flex items-center gap-2">🤖 Dashboard LLM</h1>
      </div>
      <button
        class="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-xs font-semibold transition-colors disabled:opacity-50"
        :disabled="store.chargement"
        @click="rafraichir"
      >
        {{ store.chargement ? '⏳ Chargement…' : '↺ Rafraîchir' }}
      </button>
    </div>

    <!-- Contenu scrollable ou dense -->
    <div class="flex flex-col gap-3 flex-1 min-h-0">
      
      <!-- LIGNE 1 : Modèles & Performances -->
      <div class="grid grid-cols-1 xl:grid-cols-2 gap-3 shrink-0">
        
        <!-- SECTION 1 : L'État des Cerveaux -->
        <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-blue-500/30">
          <div class="flex items-center justify-between shrink-0">
            <h2 class="font-bold flex items-center gap-2 text-base text-blue-400">
              <span>🧠</span> 1. État des Modèles ML
            </h2>
            <button
              class="shrink-0 px-3 py-1 rounded font-semibold text-[10px] uppercase transition-colors shadow-lg border"
              :class="store.retrainState?.en_cours
                ? 'bg-gray-700/50 text-white border-gray-600/50 cursor-not-allowed'
                : 'bg-blue-600/20 text-blue-300 border-blue-500/30 hover:bg-blue-600/30'"
              :disabled="store.retrainState?.en_cours"
              @click="store.declencherRetrain()"
            >
              {{ store.retrainState?.en_cours ? '⏳ En cours…' : '🔁 Entraînement' }}
            </button>
          </div>
          <div class="flex flex-col gap-2 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <MlRetrainPanel />
          </div>
        </section>

        <!-- SECTION 2 : Performances -->
        <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-emerald-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-emerald-400 shrink-0">
            <span>📈</span> 2. Performances en direct
          </h2>
          <div class="flex flex-col gap-2 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <div v-if="!store.analyse" class="bg-black/20 rounded-lg border border-white/10 p-4 text-center text-white text-xs h-full flex items-center justify-center">
              Aucune donnée disponible.
            </div>
            <div v-else class="space-y-2">
              <!-- SMC -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-blue-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-blue-200 text-xs">SMC</span>
                    <span class="text-[9px] text-white">({{ store.analyse.smc?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.smc?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.smc?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-white">{{ store.analyse.smc?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.smc?.ml_correlation?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-white">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-white'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>

              <!-- Rockets -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-orange-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-orange-200 text-xs">Rockets</span>
                    <span class="text-[9px] text-white">({{ store.analyse.rockets?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.rockets?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.rockets?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-white">{{ store.analyse.rockets?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.rockets?.conviction_llm?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-white">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-white'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>

              <!-- Straddle -->
              <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-purple-500">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="font-bold text-purple-200 text-xs">Straddle</span>
                    <span class="text-[9px] text-white">({{ store.analyse.straddle?.global.nb_trades || 0 }} tr)</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-bold" :class="(store.analyse.straddle?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.straddle?.global.win_rate?.toFixed(0) || 0 }}%</span>
                    <span class="text-xs font-bold text-white">{{ store.analyse.straddle?.global.pnl_r_moyen?.toFixed(2) || 0 }}R</span>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-1 text-[9px] bg-white/5 p-1 rounded">
                  <div v-for="t in store.analyse.straddle?.score_llm?.slice(0, 4)" :key="t.tranche" class="flex justify-between">
                    <span class="text-white">{{ t.tranche }}</span>
                    <span :class="t.win_rate >= 55 ? 'text-emerald-400' : 'text-white'">{{ t.win_rate.toFixed(0) }}%</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

      </div>

      <!-- LIGNE 2 : Prescriptions & Seuils ML -->
      <div class="grid grid-cols-1 xl:grid-cols-2 gap-3 flex-1 min-h-0">
        
        <!-- SECTION 3 : Prescriptions -->
        <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-amber-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-amber-400 shrink-0">
            <span>💊</span> 3. Prescriptions LLM
          </h2>
          <div class="flex flex-col gap-2 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <div v-if="store.suggestions.length === 0" class="bg-black/20 rounded-lg border border-white/10 p-4 text-center text-white text-xs flex items-center justify-center">
              Aucune prescription pour l'instant.
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="s in store.suggestions" :key="`${s.strategie}-${s.param_name}`"
                class="bg-black/20 rounded-lg border border-white/10 p-3 flex flex-col gap-2"
              >
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="text-[9px] font-bold px-1.5 py-0.5 rounded" :class="badgeStrategie(s.strategie)">{{ s.strategie }}</span>
                  </div>
                  <div class="flex gap-1 shrink-0">
                    <template v-if="!appliedSuggestions.has(`${s.strategie}-${s.param_name}`)">
                      <button @click="appliquer(s)" :disabled="store.application" class="px-2 py-1 rounded bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 hover:bg-emerald-600/30 text-[10px] font-bold transition-colors">✓ Appliquer</button>
                      <button class="px-2 py-1 rounded bg-white/5 hover:bg-white/10 border border-white/10 text-[10px] text-white transition-colors">✗ Ignorer</button>
                    </template>
                    <span v-else class="text-emerald-400 font-bold text-[10px] px-2 py-1 bg-emerald-400/10 rounded">✓ Appliquée</span>
                  </div>
                </div>
                <div class="flex items-center gap-2 text-sm">
                  <span class="text-emerald-400 font-bold text-base tabular-nums">{{ s.valeur_suggeree }}</span>
                  <span v-if="!appliedSuggestions.has(`${s.strategie}-${s.param_name}`)" class="text-[10px] text-white">(Objectif recommandé)</span>
                </div>
                <p class="text-[10px] text-white italic">"{{ s.justification }}"</p>
                <div class="flex items-center justify-between text-[9px] text-white border-t border-white/5 pt-1">
                  <span class="text-blue-400">Gains estimé : +{{ s.gain_winrate_estime.toFixed(1) }}% WR</span>
                  <span>Confiance : {{ (s.confiance * 100).toFixed(0) }}%</span>
                  <span>Basé sur {{ s.nb_samples_base }} trades</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- SECTION 4 : Seuils ML -->
        <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-red-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-red-400 shrink-0">
            <span>🤖</span> 4. Seuils ML par stratégie
          </h2>
          <div class="flex flex-col gap-2 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <MlSeuilsPanel ref="panelSeuils" />
          </div>
        </section>

      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'
import type { SuggestionParams } from '@/services/api.ml_insights'
import MlRetrainPanel from '@/components/common/MlRetrainPanel.vue'
import MlSeuilsPanel from '@/components/common/MlSeuilsPanel.vue'

const store = useMlInsightsStore()
const panelSeuils = ref<InstanceType<typeof MlSeuilsPanel> | null>(null)
const appliedSuggestions = ref<Set<string>>(new Set())

function badgeStrategie(s: string) {
  return s === 'SMC'      ? 'bg-blue-800 text-blue-200'    :
         s === 'ROCKETS'  ? 'bg-orange-800 text-orange-200' :
                            'bg-purple-800 text-purple-200'
}

async function rafraichir() {
  await Promise.all([store.chargerStats(), store.chargerSuggestions(), store.chargerDernierRetrain()])
  if (panelSeuils.value) panelSeuils.value.chargerSeuils()
}

async function appliquer(s: SuggestionParams) {
  await store.appliquer(s)
  appliedSuggestions.value.add(`${s.strategie}-${s.param_name}`)
  if (panelSeuils.value) {
    if (s.param_name.includes('score_min') || s.param_name.includes('conviction') || s.param_name.includes('seuil')) {
      panelSeuils.value.forcerSeuil(s.strategie, s.valeur_suggeree)
    } else {
      panelSeuils.value.chargerSeuils()
    }
  }
}

onMounted(() => {
  if (!store.analyse) store.chargerStats()
  if (store.suggestions.length === 0) store.chargerSuggestions()
  store.chargerDernierRetrain()
})
</script>
<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
</style>
