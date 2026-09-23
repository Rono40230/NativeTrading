<template>
  <!-- Page ML unique (fusion 23/09 de « Métriques ML » et « ML : état &
       seuils » — décision propriétaire). Les seuils ont été retirés : lus
       par personne jusqu'à la tâche 6.5 (filtre ML en Observation).
       Hauteur portée par le parent. -->
  <div class="flex flex-col gap-3 h-full min-h-0 overflow-y-auto custom-scrollbar pr-1">
    <div class="flex items-center justify-between flex-wrap gap-2 shrink-0 px-1">
      <h1 class="text-xl font-bold text-white">🤖 ML — état & métriques</h1>
      <span v-if="store.retrainState?.en_cours" class="text-xs text-blue-300">⏳ Entraînement en cours…</span>
      <span v-else class="text-[10px] text-white">Entraînement automatique hebdomadaire</span>
    </div>

    <!-- SECTION 1 : état des modèles + progression d'entraînement -->
    <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-blue-500/30 shrink-0">
      <h2 class="font-bold flex items-center gap-2 text-base text-blue-400 shrink-0">
        <span>🧠</span> 1. État des modèles
      </h2>
      <MlRetrainPanel />
    </section>

    <!-- SECTION 2 : performances de trading croisées avec les scores ML/LLM -->
    <section class="glass-card p-3 flex flex-col gap-2 rounded-xl border bg-white/5 border-emerald-500/30 shrink-0">
      <h2 class="font-bold flex items-center gap-2 text-base text-emerald-400 shrink-0">
        <span>📈</span> 2. Performances en direct
      </h2>
      <div v-if="!store.analyse" class="bg-black/20 rounded-lg border border-white/10 p-4 text-center text-white text-xs">
        Aucune donnée disponible.
      </div>
      <div v-else class="grid grid-cols-1 xl:grid-cols-3 gap-2">
        <!-- SMC -->
        <div class="bg-black/20 p-2 flex flex-col gap-2 rounded-lg border border-white/10 border-l-2 !border-l-blue-500">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class="font-bold text-blue-200 text-xs">SMC</span>
              <span class="text-[9px] text-white">({{ store.analyse.smc?.global.nb_trades || 0 }} tr)</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-sm font-bold" :class="(store.analyse.smc?.global.win_rate ?? 0) >= 55 ? 'text-emerald-400' : 'text-red-400'">{{ store.analyse.smc?.global.win_rate?.toFixed(0) || 0 }}%</span>
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
    </section>

    <!-- SECTION 3 : miroir exact de l'historique des trades clôturés
         (décision propriétaire 23/09) — mêmes filtres, même R encaissé
         (SMC = pondéré ventes partielles) que les tables d'historique. -->
    <p class="text-[10px] text-white shrink-0 px-1">
      Miroir de l'historique des trades clôturés — R distance (niveau le plus
      lointain atteint), la même valeur que les tables d'historique.
    </p>
    <div class="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-4 gap-3 shrink-0">
      <!-- Straddle -->
      <div class="glass-card flex flex-col rounded-xl border border-purple-500/30 bg-white/5 overflow-hidden">
        <div class="p-4 border-b border-white/10 shrink-0">
          <h2 class="font-bold flex items-center gap-2 text-base text-purple-400"><span>⚡</span> Straddle</h2>
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
          <StraddleMonitoringML compact />
        </div>
      </div>
      <!-- SMC -->
      <div class="glass-card flex flex-col rounded-xl border border-blue-500/30 bg-white/5 overflow-hidden">
        <div class="p-4 border-b border-white/10 shrink-0">
          <h2 class="font-bold flex items-center gap-2 text-base text-blue-400"><span>📊</span> SMC</h2>
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
          <SmcMonitoringML compact />
        </div>
      </div>
      <!-- Rockets -->
      <div class="glass-card flex flex-col rounded-xl border border-orange-500/30 bg-white/5 overflow-hidden">
        <div class="p-4 border-b border-white/10 shrink-0">
          <h2 class="font-bold flex items-center gap-2 text-base text-orange-400"><span>🚀</span> Rockets</h2>
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
          <RocketsMonitoringML compact />
        </div>
      </div>
      <!-- KDJ (23/09 : la collecte ML attrape les clôtures KDJ ; stats dès
           les premiers trades fermés) -->
      <div class="glass-card flex flex-col rounded-xl border border-teal-500/30 bg-white/5 overflow-hidden">
        <div class="p-4 border-b border-white/10 shrink-0">
          <h2 class="font-bold flex items-center gap-2 text-base text-teal-300"><span>📈</span> KDJ/Halftrend</h2>
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
          <KdjMonitoringML compact />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'
import MlRetrainPanel from '@/components/common/MlRetrainPanel.vue'
import StraddleMonitoringML from '@/components/common/StraddleMonitoringML.vue'
import RocketsMonitoringML from '@/components/common/RocketsMonitoringML.vue'
import KdjMonitoringML from '@/components/common/KdjMonitoringML.vue'
import SmcMonitoringML from '@/components/common/SmcMonitoringML.vue'

const store = useMlInsightsStore()

onMounted(() => {
  if (!store.analyse) void store.chargerStats()
  void store.chargerDernierRetrain()
})
</script>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
</style>
