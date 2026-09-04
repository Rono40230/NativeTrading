<template>
  <!-- « Rapport d'activité » — centre d'analyse des trois stratégies.
       Onglet par stratégie + vue d'ensemble ; ?strategie= cible l'onglet
       (boutons SMC/Straddle/Rockets du bloc dashboard). -->
  <div class="flex flex-col min-h-full gap-3">
    <!-- Bandeau -->
    <div class="flex items-center gap-3 flex-wrap">
      <h1 class="text-xl font-bold text-white">📊 Rapport d'activité</h1>
      <div class="flex gap-1">
        <button
          v-for="o in ONGLETS"
          :key="o.id"
          class="text-[11px] px-2.5 py-1 rounded-lg font-semibold transition-colors"
          :class="onglet === o.id ? 'bg-teal-500/30 text-white' : 'bg-white/5 text-white hover:bg-white/10'"
          @click="changer(o.id)"
        >{{ o.label }}</button>
      </div>
      <span class="ml-auto text-[10px] text-white" title="$ réels composés et R de la convention de chaque moteur — jamais de R de référence ni de pips (décision 04/09)">
        $ réels · R pondéré/net · données de la veille
      </span>
    </div>

    <!-- Contenu -->
    <AnalysesVueEnsemble v-if="onglet === 'tout'" @ouvrir="changer" />
    <AnalysesOngletStrategie v-else :id="onglet" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AnalysesVueEnsemble from '@/components/analyses/AnalysesVueEnsemble.vue'
import AnalysesOngletStrategie from '@/components/analyses/AnalysesOngletStrategie.vue'

type Onglet = 'tout' | 'SMC' | 'straddle' | 'rockets'

const ONGLETS: { id: Onglet; label: string }[] = [
  { id: 'tout', label: '🧭 Vue d\u2019ensemble' },
  { id: 'SMC', label: '📐 SMC' },
  { id: 'straddle', label: '⚡ Straddle' },
  { id: 'rockets', label: '🚀 Rockets' },
]

const VALIDES = new Set(ONGLETS.map(o => o.id))

const route = useRoute()
const router = useRouter()
const onglet = ref<Onglet>('tout')

function changer(o: Onglet | string) {
  const cible = VALIDES.has(o as Onglet) ? (o as Onglet) : 'tout'
  onglet.value = cible
  // URL synchronisée (partageable, bouton précédent du navigateur).
  void router.replace({ query: cible === 'tout' ? {} : { strategie: cible } })
}

// Montage + navigation directe (?strategie= depuis le bloc dashboard).
watch(
  () => route.query.strategie,
  (q) => {
    const id = typeof q === 'string' ? q : 'tout'
    onglet.value = VALIDES.has(id as Onglet) ? (id as Onglet) : 'tout'
  },
  { immediate: true },
)
</script>
