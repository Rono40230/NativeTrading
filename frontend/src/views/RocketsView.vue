<template>
  <!-- Page Rockets (refonte layout 06/09) : colonne Setups sur TOUTE la
       hauteur à gauche ; à droite, trois blocs au design identique (carte +
       barre de couleur latérale), même largeur, empilés — À risque,
       Neutralisées, Historique. Le poste d'observation est en lecture seule :
       le moteur décide au cycle de 30 s. -->
  <div class="flex-1 min-h-0 flex flex-col gap-3 rounded-xl bg-purple-500/5 px-4 py-3">
    <!-- En-tête : identité + navigation (même design que le shell) -->
    <div class="glass-card px-4 py-3 flex items-center gap-3 shrink-0">
      <span class="text-xl leading-none">🚀</span>
      <h1 class="text-xl font-bold text-white truncate">Stratégie Rockets</h1>
      <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border shrink-0 bg-amber-500/10 text-amber-300 border-amber-500/40">Observation</span>
      <div class="ml-auto flex gap-2 shrink-0">
        <button class="btn-sm" @click="router.push('/rockets/scanner')">🔭 Scanner</button>
        <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="router.push('/rockets/analyse')">📊 Analyse</button>
      </div>
    </div>

    <!-- Corps : setups pleine hauteur | 3 blocs empilés -->
    <div class="flex-1 min-h-0 grid grid-cols-6 gap-3">
      <!-- Colonne Setups — toute la hauteur -->
      <aside class="col-span-1 min-h-0 glass-card px-4 py-3 flex flex-col">
        <h2 class="text-xs uppercase text-white font-semibold tracking-wider mb-2 shrink-0">⏳ Setups en attente</h2>
        <div class="flex-1 min-h-0 overflow-y-auto pr-2">
          <RocketsSetupsApercu />
        </div>
      </aside>

      <!-- Les trois blocs : même design, même largeur, empilés -->
      <div class="col-span-5 min-h-0 flex flex-col gap-3">

        <!-- 🔴 À risque -->
        <section class="bloc bloc-rouge shrink-0">
          <PositionsARisqueTable :positions="risque" :live="live" @cloturee="rechargerPositions()" />
        </section>

        <!-- 🟡 Neutralisées -->
        <section class="bloc bloc-ambre shrink-0">
          <PositionsNeutraliseesTable :positions="neutralisees" :live="live" />
        </section>

        <!-- 📜 Historique — occupe le reste de la hauteur -->
        <section class="bloc bloc-violet flex-1 min-h-0 flex flex-col overflow-hidden">
          <RocketsHistoriqueTable />
        </section>
      </div>
    </div>
  </div>

</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import RocketsSetupsApercu from '@/components/common/RocketsSetupsApercu.vue'
import PositionsARisqueTable from '@/components/common/PositionsARisqueTable.vue'
import PositionsNeutraliseesTable from '@/components/common/PositionsNeutraliseesTable.vue'
import RocketsHistoriqueTable from '@/components/common/RocketsHistoriqueTable.vue'
import { usePositionsRockets } from '@/composables/usePositionsRockets'

const router = useRouter()

const { risque, neutralisees, live, charger: rechargerPositions } = usePositionsRockets()
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }

/* Le design commun des trois blocs : carte + barre de couleur latérale. */
.bloc {
  @apply rounded-xl border bg-white/[0.02] p-3;
  border-left-width: 4px;
}
.bloc-rouge  { @apply border-red-500/30;    border-left-color: rgb(239 68 68 / 0.6); }
.bloc-ambre  { @apply border-amber-500/30;  border-left-color: rgb(245 158 11 / 0.6); }
.bloc-violet { @apply border-purple-500/30; border-left-color: rgb(168 85 247 / 0.6); }
</style>
