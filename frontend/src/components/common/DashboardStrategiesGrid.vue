<template>
  <!-- LIGNES STRATEGIES (En 3 colonnes) -->
  <div class="flex-1 min-h-0 grid grid-cols-3 gap-3">
    
    <!-- COLONNE 1: SMC -->
    <div class="flex flex-col p-2 gap-2 relative border border-blue-500/20 bg-blue-500/10 rounded-xl backdrop-blur-sm hover:z-[999] min-h-0">
       <div class="text-xs font-bold text-blue-400 uppercase tracking-widest pl-1 border-b border-blue-500/30 pb-1 flex items-center gap-1.5">📐 Stratégie SMC
       <span
         v-if="dansMoins30min"
         class="ml-auto flex items-center gap-1 rounded-full border border-yellow-500/40 bg-yellow-500/10 px-2 py-0.5 text-[8px] font-bold text-yellow-300"
         :title="`SMC Directionnel suspendu automatiquement — ${prochainEvent?.titre}`"
       >
         ⏸ Suspendu
       </span></div>
       
       <!-- Haut: Graphique -->
       <div class="h-[160px] shrink-0 flex flex-col relative z-20 cursor-zoom-in group" @click="isHoveredSmc = true">
          <div class="relative flex-1 min-h-0 flex flex-col transition-all duration-300 ease-out origin-center group-hover:brightness-125 rounded-xl bg-transparent pointer-events-none">
             <SmcEquityChart class="flex-1 min-h-0 bg-[#0a0e27]/80 rounded-xl" />
          </div>

          <!-- Fullscreen Centered Click Modal via Teleport -->
          <Teleport to="body">
             <div 
                class="fixed inset-0 z-[99999] bg-black/60 backdrop-blur-sm transition-all duration-300 ease-out flex items-center justify-center cursor-default"
                :class="isHoveredSmc ? 'opacity-100 visible pointer-events-auto' : 'opacity-0 invisible pointer-events-none'"
                @click="isHoveredSmc = false"
             >
                <div 
                   class="relative w-[1600px] max-w-[95vw] h-[750px] max-h-[90vh] shadow-[0_30px_60px_rgba(0,0,0,0.95)] rounded-2xl bg-[#0c1130]/95 backdrop-blur-xl border border-white/10 p-4 flex flex-col gap-4 transition-transform duration-300 ease-out cursor-default"
                   :class="isHoveredSmc ? 'scale-100' : 'scale-95'"
                   @click.stop
                 >
                   <button @click="isHoveredSmc = false" class="absolute -top-3 -right-3 w-8 h-8 flex items-center justify-center rounded-full bg-red-500/20 text-red-500 hover:bg-red-500 hover:text-white border border-red-500/50 transition-colors z-50">
                      <span class="text-sm font-bold">✕</span>
                   </button>
                   <!-- Top: Expanded Chart -->
                   <div class="flex-1 min-h-0 flex flex-col relative z-10">
                      <SmcEquityChart class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block p-3" />
                   </div>
                   <!-- Bottom: Metrics -->
                   <div class="h-[240px] shrink-0 flex flex-col relative z-10">
                      <SmcPerfBloc class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block overflow-y-auto" />
                   </div>
                </div>
             </div>
          </Teleport>
       </div>

       <!-- Bas: Blocs de la stratégie -->
       <div class="flex-1 min-h-0 flex flex-col gap-2 relative z-10 overflow-hidden">
          <SmcSignauxBloc class="flex-1 min-h-0 overflow-y-auto" />
       </div>
    </div>

    <!-- COLONNE 2: STRADDLE -->
    <div class="flex flex-col p-2 gap-2 relative border border-yellow-500/20 bg-yellow-500/10 rounded-xl backdrop-blur-sm hover:z-[999] min-h-0">
       <div class="text-xs font-bold text-yellow-400 uppercase tracking-widest pl-1 border-b border-yellow-500/30 pb-1 flex items-center gap-1.5">⚡ STRATÉGIE VOLATILITÉ</div>
       
       <!-- Haut: Graphique -->
       <div class="h-[160px] shrink-0 flex flex-col relative z-20 cursor-zoom-in group" @click="isHoveredStraddle = true">
          <div class="relative flex-1 min-h-0 flex flex-col transition-all duration-300 ease-out origin-center group-hover:brightness-125 rounded-xl bg-transparent pointer-events-none">
             <StraddleEquityChart class="flex-1 min-h-0 bg-[#0a0e27]/80 rounded-xl" />
          </div>

          <!-- Fullscreen Centered Click Modal via Teleport -->
          <Teleport to="body">
             <div 
                class="fixed inset-0 z-[99999] bg-black/60 backdrop-blur-sm transition-all duration-300 ease-out flex items-center justify-center cursor-default"
                :class="isHoveredStraddle ? 'opacity-100 visible pointer-events-auto' : 'opacity-0 invisible pointer-events-none'"
                @click="isHoveredStraddle = false"
             >
                <div 
                   class="relative w-[1600px] max-w-[95vw] h-[750px] max-h-[90vh] shadow-[0_30px_60px_rgba(0,0,0,0.95)] rounded-2xl bg-[#0c1130]/95 backdrop-blur-xl border border-white/10 p-4 flex flex-col gap-4 transition-transform duration-300 ease-out cursor-default"
                   :class="isHoveredStraddle ? 'scale-100' : 'scale-95'"
                   @click.stop
                 >
                   <button @click="isHoveredStraddle = false" class="absolute -top-3 -right-3 w-8 h-8 flex items-center justify-center rounded-full bg-red-500/20 text-red-500 hover:bg-red-500 hover:text-white border border-red-500/50 transition-colors z-50">
                      <span class="text-sm font-bold">✕</span>
                   </button>
                   <!-- Top: Expanded Chart -->
                   <div class="flex-1 min-h-0 flex flex-col relative z-10">
                      <StraddleEquityChart class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block p-3" />
                   </div>
                   <!-- Bottom: Metrics -->
                   <div class="h-[240px] shrink-0 flex flex-col relative z-10">
                      <StratPerfBloc class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block overflow-y-auto" />
                   </div>
                </div>
             </div>
          </Teleport>
       </div>

       <!-- Bas: Blocs de la stratégie -->
       <div class="flex-1 min-h-0 flex flex-col gap-2 relative z-10 overflow-hidden">
          <StraddleVolatiliteBloc class="flex-[3] min-h-0 overflow-y-auto" />
          <StraddleCreneauxBloc class="flex-[6] min-h-0 overflow-y-auto" />
       </div>
    </div>

    <!-- COLONNE 3: ROCKETS -->
    <div class="flex flex-col p-2 gap-2 relative border border-orange-500/20 bg-orange-500/10 rounded-xl backdrop-blur-sm hover:z-[999] min-h-0">
       <div class="text-xs font-bold text-orange-400 uppercase tracking-widest pl-1 border-b border-orange-500/30 pb-1 flex items-center gap-1.5">🚀 Stratégie Rockets</div>
       
       <!-- Haut: Graphique -->
       <div class="h-[160px] shrink-0 flex flex-col relative z-20 cursor-zoom-in group" @click="isHoveredRockets = true">
          <div class="relative flex-1 min-h-0 flex flex-col transition-all duration-300 ease-out origin-center group-hover:brightness-125 rounded-xl bg-transparent pointer-events-none">
             <RocketsEquityChart class="flex-1 min-h-0 bg-[#0a0e27]/80 rounded-xl" />
          </div>

          <!-- Fullscreen Centered Click Modal via Teleport -->
          <Teleport to="body">
             <div 
                class="fixed inset-0 z-[99999] bg-black/60 backdrop-blur-sm transition-all duration-300 ease-out flex items-center justify-center cursor-default"
                :class="isHoveredRockets ? 'opacity-100 visible pointer-events-auto' : 'opacity-0 invisible pointer-events-none'"
                @click="isHoveredRockets = false"
             >
                <div 
                   class="relative w-[1600px] max-w-[95vw] h-[750px] max-h-[90vh] shadow-[0_30px_60px_rgba(0,0,0,0.95)] rounded-2xl bg-[#0c1130]/95 backdrop-blur-xl border border-white/10 p-4 flex flex-col gap-4 transition-transform duration-300 ease-out cursor-default"
                   :class="isHoveredRockets ? 'scale-100' : 'scale-95'"
                   @click.stop
                 >
                   <button @click="isHoveredRockets = false" class="absolute -top-3 -right-3 w-8 h-8 flex items-center justify-center rounded-full bg-red-500/20 text-red-500 hover:bg-red-500 hover:text-white border border-red-500/50 transition-colors z-50">
                      <span class="text-sm font-bold">✕</span>
                   </button>
                   <!-- Top: Expanded Chart -->
                   <div class="flex-1 min-h-0 flex flex-col relative z-10">
                      <RocketsEquityChart class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block p-3" />
                   </div>
                   <!-- Bottom: Metrics -->
                   <div class="h-[240px] shrink-0 flex flex-col relative z-10">
                      <RocketsPerfBloc class="flex-1 min-h-0 bg-black/20 border border-white/5 rounded-xl block overflow-y-auto" />
                   </div>
                </div>
             </div>
          </Teleport>
       </div>

       <!-- Bas: Blocs de la stratégie -->
       <div class="flex-1 min-h-0 flex flex-col gap-2 relative z-10 overflow-hidden">
          <VeilleRockets
            class="flex-1 min-h-0"
            :signaux="rockets.signaux.value"
            :total-candidats="rockets.totalCandidats.value"
            :chargement="rockets.chargement.value"
            :erreur="rockets.erreur.value"
            :progression="rockets.progression.value"
            :derniere-m-a-j="rockets.derniereMAJ.value"
          />
       </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

import SmcEquityChart from '@/components/common/SmcEquityChart.vue'
import SmcSignauxBloc from '@/components/common/SmcSignauxBloc.vue'
import SmcPerfBloc from '@/components/common/SmcPerfBloc.vue'
import StraddleVolatiliteBloc from '@/components/common/StraddleVolatiliteBloc.vue'
import StraddleEquityChart from '@/components/common/StraddleEquityChart.vue'
import StraddleCreneauxBloc from '@/components/common/StraddleCreneauxBloc.vue'
import VeilleRockets from '@/components/common/VeilleRockets.vue'
import RocketsPerfBloc from '@/components/common/RocketsPerfBloc.vue'
import RocketsEquityChart from '@/components/common/RocketsEquityChart.vue'
import StratPerfBloc from '@/components/common/StratPerfBloc.vue'

import { useVeilleRockets } from '@/composables/useVeilleRockets'

const rockets = useVeilleRockets()


import { apiService } from '@/services/api.service'
import type { AnnonceCalendrier } from '@/services/api.types'

const annonces = ref<AnnonceCalendrier[]>([])

onMounted(async () => {
  annonces.value = await apiService.obtenirCalendrier(2)
})

const prochainEvent = computed<AnnonceCalendrier | undefined>(() => {
  const maintenant = Date.now()
  const dans24h = maintenant + 24 * 3_600_000
  return annonces.value.find((a) => {
    const ts = new Date(a.date_heure).getTime()
    return !a.est_passe && ts > maintenant && ts <= dans24h && a.impact === 'High'
  })
})

const dansMoins30min = computed(() => {
  if (!prochainEvent.value) return false
  const diff = new Date(prochainEvent.value.date_heure).getTime() - Date.now()
  return diff > 0 && diff < 30 * 60_000
})

const isHoveredSmc = ref(false)
const isHoveredStraddle = ref(false)
const isHoveredRockets = ref(false)

onMounted(() => {
  rockets.demarrer()
})

onUnmounted(() => {
  rockets.arreter()
})
</script>
