<template>
  <!-- Page « Fonctionnalités IA » — 3 onglets regroupant les outils IA
       (refonte navigation 01/09 : la barre de titre ne garde que le
       Dashboard, l'accès se fait par la tuile du même nom). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-violet-500/5 rounded-xl px-3 py-2">
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t.id"
        class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="onglet === t.id
          ? 'border-blue-500 text-white bg-white/5'
          : 'border-transparent text-white hover:text-white hover:bg-white/5'"
        @click="changer(t.id)"
      >
        <span class="mr-1.5">{{ t.icone }}</span>{{ t.label }}
      </button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      <!-- KeepAlive : la conversation du coach et les images importées
           survivent aux changements d'onglet. -->
      <KeepAlive>
        <component :is="vueActive" />
      </KeepAlive>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import ChartImportView from '@/views/ChartImportView.vue'
import SMCCoachView from '@/views/SMCCoachView.vue'
import PromptsIAView from '@/views/PromptsIAView.vue'

/// Emballage nommé : KeepAlive a besoin de composants nommés pour
/// conserver l'état de chaque onglet.
function nommee(nom: string, vue: ReturnType<typeof defineComponent>) {
  return defineComponent({ name: nom, setup: () => () => h(vue) })
}

const VUES = {
  chart: nommee('IaAnalyseChart', ChartImportView),
  coach: nommee('IaCoach', SMCCoachView),
  prompts: nommee('IaPrompts', PromptsIAView),
} as const

type OngletId = keyof typeof VUES

const onglets: { id: OngletId; icone: string; label: string }[] = [
  { id: 'chart', icone: '🖼️', label: 'Analyse graphique' },
  { id: 'coach', icone: '💬', label: 'Coach IA' },
  { id: 'prompts', icone: '✏️', label: 'Prompts' },
]

const route = useRoute()
const router = useRouter()

const onglet = computed<OngletId>(() => {
  const t = route.query.tab
  return typeof t === 'string' && t in VUES ? (t as OngletId) : 'chart'
})

const vueActive = computed(() => VUES[onglet.value])

function changer(id: OngletId) {
  router.push({ path: '/ia', query: { tab: id } })
}

/// Astuce de survol de la tuile dashboard : ouvrir directement un onglet.
onMounted(() => {
  const t = route.query.tab
  if (typeof t !== 'string') router.replace({ path: '/ia', query: { tab: 'chart' } })
})
</script>
