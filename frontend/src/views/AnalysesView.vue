<template>
  <!-- « Rapport d'activité » (owner 11/10) : PLUS D'ONGLETS — la page
       s'ouvre sur la vue d'ensemble enrichie (les 4 stratégies + totaux +
       synthèse IA). Le rapport DÉTAILLÉ d'une stratégie s'ouvre par sa
       carte du dashboard (?strategie=) ou le clic sur sa ligne. -->
  <div class="flex flex-col min-h-full gap-3">
    <!-- Bandeau -->
    <div class="flex items-center gap-3 flex-wrap">
      <button
        v-if="onglet !== 'tout'"
        class="text-[11px] px-2.5 py-1 rounded-lg font-semibold bg-white/5 text-white hover:bg-white/10 transition-colors"
        title="Retour à la vue d'ensemble"
        @click="changer('tout')"
      >← Vue d'ensemble</button>
      <h1 class="text-xl font-bold text-white">📊 Rapport d'activité</h1>
      <span v-if="onglet !== 'tout'" class="text-[10px] text-white/60">{{ nomStrategie(onglet) }}</span>
      <span class="ml-auto text-[10px] text-white" title="$ réels composés et R de la convention de chaque moteur — jamais de R de référence ni de pips (décision 04/09)">
        $ réels composés · R de la convention du moteur
      </span>
    </div>

    <!-- Contenu : vue d'ensemble OU rapport d'une stratégie (sans onglets) -->
    <AnalysesVueEnsemble v-if="onglet === 'tout'" @ouvrir="changer" />
    <AnalysesOngletStrategie v-else :id="onglet" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AnalysesVueEnsemble from '@/components/analyses/AnalysesVueEnsemble.vue'
import AnalysesOngletStrategie from '@/components/analyses/AnalysesOngletStrategie.vue'

type Onglet = 'tout' | 'SMC' | 'straddle' | 'rockets' | 'kdj_halftrend'

const VALIDES = new Set<Onglet>(['tout', 'SMC', 'straddle', 'rockets', 'kdj_halftrend'])
const NOMS: Record<Onglet, string> = {
  tout: '', SMC: '📐 SMC', straddle: '⚡ Straddle', rockets: '🚀 Rockets', kdj_halftrend: '📈 KDJ/Halftrend',
}

const route = useRoute()
const router = useRouter()
const onglet = ref<Onglet>('tout')

function nomStrategie(o: Onglet): string {
  return NOMS[o] ?? ''
}

function changer(o: Onglet | string) {
  const cible = VALIDES.has(o as Onglet) ? (o as Onglet) : 'tout'
  onglet.value = cible
  void router.replace({ query: cible === 'tout' ? {} : { strategie: cible } })
}

// Montage + navigation directe (?strategie= depuis les cartes du dashboard).
watch(
  () => route.query.strategie,
  (q) => {
    const id = typeof q === 'string' ? q : 'tout'
    onglet.value = VALIDES.has(id as Onglet) ? (id as Onglet) : 'tout'
  },
  { immediate: true },
)
</script>
