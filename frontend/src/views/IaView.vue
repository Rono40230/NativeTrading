<template>
  <!-- Page « Fonctionnalités IA » — 3 onglets (14/09) : les anciens boutons
       de la tuile dashboard (Prompts / Métriques ML / Dashboard LLM) vivent
       ici ; ?onglet= cible l'onglet (les anciennes routes /ia/ml et /ia/llm
       redirigent). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-violet-500/5 rounded-xl px-3 py-2">
    <div class="flex items-center gap-3 flex-wrap shrink-0 px-1">
      <h1 class="text-xl font-bold text-white">🧠 Fonctionnalités IA</h1>
      <div class="flex gap-1">
        <button
          v-for="o in ONGLETS"
          :key="o.id"
          class="text-[11px] px-2.5 py-1 rounded-lg font-semibold transition-colors"
          :class="onglet === o.id ? 'bg-violet-500/30 text-white' : 'bg-white/5 text-white hover:bg-white/10'"
          @click="changer(o.id)"
        >{{ o.label }}</button>
      </div>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      <PromptsIAView v-if="onglet === 'prompts'" />
      <MetriquesMLView v-else-if="onglet === 'ml'" />
      <MlInsightsView v-else />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import PromptsIAView from '@/views/PromptsIAView.vue'
import MetriquesMLView from '@/views/MetriquesMLView.vue'
import MlInsightsView from '@/views/MlInsightsView.vue'

type Onglet = 'prompts' | 'ml' | 'llm'

const ONGLETS: { id: Onglet; label: string }[] = [
  { id: 'prompts', label: '✏️ Prompts' },
  { id: 'ml', label: '📉 Métriques ML' },
  { id: 'llm', label: '🤖 Dashboard LLM' },
]

const VALIDES = new Set(ONGLETS.map(o => o.id))

const route = useRoute()
const router = useRouter()
const onglet = ref<Onglet>('prompts')

function changer(o: Onglet | string) {
  const cible = VALIDES.has(o as Onglet) ? (o as Onglet) : 'prompts'
  onglet.value = cible
  // URL synchronisée (partageable, bouton précédent du navigateur).
  void router.replace({ query: cible === 'prompts' ? {} : { onglet: cible } })
}

// Montage + navigation directe (?onglet= depuis les redirections /ia/ml
// et /ia/llm).
watch(
  () => route.query.onglet,
  (q) => {
    const id = typeof q === 'string' ? q : 'prompts'
    onglet.value = VALIDES.has(id as Onglet) ? (id as Onglet) : 'prompts'
  },
  { immediate: true },
)
</script>
