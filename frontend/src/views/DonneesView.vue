<template>
  <!-- Page « Données » — plomberie de l'app : pilotage du pipeline,
       conventions de risque par actif, connexions. (Ex-onglets Système ;
       les paramètres des stratégies vivent désormais dans chaque page
       stratégie, bouton ⚙️ Paramètres.) -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-rose-500/5 rounded-xl px-3 py-2">
    <div class="flex gap-1 border-b border-white/10 shrink-0 overflow-x-auto">
      <button
        v-for="t in onglets"
        :key="t.id"
        class="px-4 py-2 text-sm font-semibold whitespace-nowrap transition-colors border-b-2"
        :class="onglet === t.id
          ? 'border-blue-500 text-white bg-white/5'
          : 'border-transparent text-white hover:text-white hover:bg-white/5'"
        @click="changer(t.id)"
      >
        <span class="mr-1.5">{{ t.icone }}</span>{{ t.label }}
      </button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      <KeepAlive>
        <component :is="vueActive" />
      </KeepAlive>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import DataManagementView from '@/views/DataManagementView.vue'
import AssetParamsPanel from '@/components/common/AssetParamsPanel.vue'
import ApiKeysPanel from '@/components/common/ApiKeysPanel.vue'

function nommee(nom: string, vue: ReturnType<typeof defineComponent>) {
  return defineComponent({ name: nom, setup: () => () => h(vue) })
}

const VUES = {
  pilotage: nommee('DonneesPilotage', DataManagementView),
  risque: nommee('DonneesRisque', defineComponent({
    setup: () => () => h('div', { class: 'glass-card p-4' }, [h(AssetParamsPanel)]),
  })),
  connexions: nommee('DonneesConnexions', ApiKeysPanel),
} as const

type OngletId = keyof typeof VUES

const onglets: { id: OngletId; icone: string; label: string }[] = [
  { id: 'pilotage', icone: '📦', label: 'Pilotage du pipeline' },
  { id: 'risque', icone: '📊', label: 'Gestion du risque' },
  { id: 'connexions', icone: '🔌', label: 'Connexions' },
]

const route = useRoute()
const router = useRouter()

const onglet = computed<OngletId>(() => {
  const t = route.query.tab
  return typeof t === 'string' && t in VUES ? (t as OngletId) : 'pilotage'
})

const vueActive = computed(() => VUES[onglet.value])

function changer(id: OngletId) {
  router.push({ path: '/donnees', query: { tab: id } })
}

onMounted(() => {
  const t = route.query.tab
  if (typeof t !== 'string') router.replace({ path: '/donnees', query: { tab: 'pilotage' } })
})
</script>
