<template>
  <!-- Page « Paramètres » — les réglages des stratégies, une carte chacune.
       Ouverte par le bouton ⚙️ des pages stratégies (?strategie=SMC n'en
       montre qu'une) ou directement (les trois). Bouton ← Stratégie quand
       une stratégie est ciblée (retour direct à sa page, 05/09). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-white/5 rounded-xl px-3 py-2">
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">⚙️ Paramètres</h1>
      <span v-if="filtre" class="text-sm text-white">· {{ nomStrategie }}</span>
      <RouterLink
        v-if="filtre"
        :to="routeStrategie"
        class="text-[11px] px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors whitespace-nowrap"
        :title="`Retour à la ${nomStrategie}`"
      >← Stratégie</RouterLink>
      <RouterLink
        v-if="filtre"
        :to="{ path: '/parametres' }"
        class="ml-auto text-[11px] px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors"
      >← Toutes les stratégies</RouterLink>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      <StrategiesParamsPanel :filtre="filtre" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import StrategiesParamsPanel from '@/components/StrategiesParamsPanel.vue'

const route = useRoute()
const filtre = computed(() => {
  const s = route.query.strategie
  return typeof s === 'string' ? s : null
})

const NOMS: Record<string, string> = {
  SMC: 'Stratégie SMC',
  straddle: 'Stratégie Straddle',
  rockets: 'Stratégie Rockets',
}
const nomStrategie = computed(() => (filtre.value ? NOMS[filtre.value] ?? filtre.value : ''))

/// Route de la page stratégie ciblée (bouton ← Stratégie).
const ROUTES: Record<string, string> = {
  SMC: '/smc',
  straddle: '/straddle',
  rockets: '/rockets',
}
const routeStrategie = computed(() => (filtre.value ? ROUTES[filtre.value] ?? '/smc' : '/smc'))
</script>
