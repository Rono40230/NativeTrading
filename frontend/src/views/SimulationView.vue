<template>
  <div class="p-4 lg:p-6 max-w-7xl mx-auto flex flex-col gap-4">
    <!-- En-tête : identité + étiquette laboratoire -->
    <div class="glass-card p-3 flex flex-wrap items-center gap-2">
      <span class="text-lg">🧪</span>
      <span class="font-bold text-white">Laboratoire de simulation</span>
      <span class="text-[10px] font-bold px-2 py-0.5 rounded-full border border-amber-500/40 bg-amber-500/10 text-amber-300">SIMULATION — jamais les chiffres officiels</span>
      <span class="ml-auto text-[10px] text-white/70">Les réglages modifiés ici sont virtuels tant qu'on n'applique pas</span>
    </div>

    <!-- Onglets par stratégie -->
    <div class="flex gap-1 flex-wrap">
      <button
        v-for="s in STRATEGIES"
        :key="s.id"
        class="text-xs px-3 py-1.5 rounded-lg font-semibold transition-colors"
        :class="onglet === s.id ? 'bg-teal-500/30 text-white' : 'bg-white/5 text-white hover:bg-white/10'"
        @click="choisir(s.id)"
      >{{ s.icone }} {{ s.nom }}</button>
    </div>

    <!-- Panneau de la stratégie -->
    <SimulationSmcPanel v-if="onglet === 'SMC'" />
    <SimulationStraddlePanel v-else-if="onglet === 'straddle'" />
    <div v-else class="glass-card p-8 text-center text-sm text-white flex flex-col gap-2">
      <p>🔬 Simulation {{ nomStrategie(onglet) }} — en attente de matière.</p>
      <p class="text-white/60 text-xs">Effectif clôturé insuffisant pour toute conclusion (règle des 30 trades) et simulateur dédié à reconstruire (le backtesteur historique a été démantelé lors des purges de septembre). La page s'activera avec le chantier backtesteur.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import SimulationSmcPanel from '@/components/simulation/SimulationSmcPanel.vue'
import SimulationStraddlePanel from '@/components/simulation/SimulationStraddlePanel.vue'

const route = useRoute()
const router = useRouter()

const STRATEGIES = [
  { id: 'SMC', nom: 'SMC', icone: '📐' },
  { id: 'straddle', nom: 'Straddle', icone: '⚡' },
  { id: 'rockets', nom: 'Rockets', icone: '🚀' },
  { id: 'kdj_halftrend', nom: 'KDJ/Halftrend', icone: '📈' },
]

const onglet = ref<string>(typeof route.query.strategie === 'string' ? route.query.strategie : 'SMC')

function nomStrategie(id: string): string {
  return STRATEGIES.find(s => s.id === id)?.nom ?? id
}

function choisir(id: string) {
  onglet.value = id
  router.replace({ query: { ...route.query, strategie: id } })
}

watch(() => route.query.strategie, (v) => {
  if (typeof v === 'string' && v !== onglet.value) onglet.value = v
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
