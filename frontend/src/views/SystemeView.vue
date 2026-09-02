<template>
  <!-- Page « Système » — onglets Paramètres + Données (refonte navigation
       01/09 : accès par la tuile du dashboard, la barre ne garde que
       le Dashboard). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-rose-500/5 rounded-xl px-3 py-2">
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
      <KeepAlive>
        <component :is="vueActive" />
      </KeepAlive>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import SettingsView from '@/views/SettingsView.vue'
import DataManagementView from '@/views/DataManagementView.vue'

function nommee(nom: string, vue: ReturnType<typeof defineComponent>) {
  return defineComponent({ name: nom, setup: () => () => h(vue) })
}

const VUES = {
  settings: nommee('SystemeParametres', SettingsView),
  data: nommee('SystemeDonnees', DataManagementView),
} as const

type OngletId = keyof typeof VUES

const onglets: { id: OngletId; icone: string; label: string }[] = [
  { id: 'settings', icone: '⚙️', label: 'Paramètres' },
  { id: 'data', icone: '📦', label: 'Données' },
]

const route = useRoute()
const router = useRouter()

const onglet = computed<OngletId>(() => {
  const t = route.query.tab
  return typeof t === 'string' && t in VUES ? (t as OngletId) : 'settings'
})

const vueActive = computed(() => VUES[onglet.value])

function changer(id: OngletId) {
  router.push({ path: '/systeme', query: { tab: id } })
}

onMounted(() => {
  const t = route.query.tab
  if (typeof t !== 'string') router.replace({ path: '/systeme', query: { tab: 'settings' } })
})
</script>
