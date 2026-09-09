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

      <!-- LIGNE 2 : Seuils ML -->
      <div class="grid grid-cols-1 gap-3 flex-1 min-h-0">

        <!-- SECTION 3 : Seuils ML -->
        <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-red-500/30">
          <h2 class="font-bold flex items-center gap-2 text-base text-red-400 shrink-0">
            <span>🤖</span> 3. Seuils ML par stratégie
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
import MlRetrainPanel from '@/components/common/MlRetrainPanel.vue'
import MlSeuilsPanel from '@/components/common/MlSeuilsPanel.vue'

const store = useMlInsightsStore()
const panelSeuils = ref<InstanceType<typeof MlSeuilsPanel> | null>(null)
async function rafraichir() {
  await Promise.all([store.chargerStats(), store.chargerDernierRetrain()])
  if (panelSeuils.value) panelSeuils.value.chargerSeuils()
}

onMounted(() => {
  if (!store.analyse) store.chargerStats()
  store.chargerDernierRetrain()
})
</script>
<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
</style>
