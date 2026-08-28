<template>
  <!-- Barre de titre applicative (décision propriétaire 28/08 — option 3) :
       remplace la barre OS (decorations: false) et le panneau latéral.
       Navigation principale en texte (1 clic), sous-pages en déroulant ▾,
       outils regroupés à droite, contrôles fenêtre intégrés. -->
  <header
    class="title-bar flex items-stretch gap-0.5 select-none border-b border-white/10 bg-gray-900"
    data-tauri-drag-region
  >
    <!-- Navigation principale -->
    <nav class="flex items-stretch gap-0.5 pl-2 min-w-0" data-tauri-drag-region>
      <div
        v-for="item in principaux"
        :key="item.to"
        class="relative flex items-stretch"
      >
        <button
          class="flex items-center gap-1.5 px-2.5 text-xs transition-colors"
          :class="actif(item.to) ? 'text-white bg-white/10' : 'text-gray-400 hover:text-white hover:bg-white/5'"
          @click="aller(item.to)"
        >
          <span class="text-sm leading-none">{{ item.icone }}</span>
          <span class="whitespace-nowrap">{{ item.label }}</span>
        </button>
        <!-- Caret déroulant : sous-pages de la stratégie -->
        <button
          v-if="item.sous"
          class="flex items-center pr-1.5 text-[8px] transition-colors"
          :class="ouvert === item.to || sousPageActive(item) ? 'text-white' : 'text-gray-600 hover:text-white'"
          :title="`Sous-pages ${item.label}`"
          @click.stop="basculer(item.to)"
        >▼</button>
        <!-- Déroulant ancré sous le bouton -->
        <div v-if="item.sous && ouvert === item.to" class="menu-deroulant aligne-gauche">
          <div @click="fermer">
            <router-link :to="item.to" class="item-menu" :class="{ 'text-white bg-white/10': actif(item.to) }">
              <span>⚡</span><span>Signaux {{ item.label }}</span>
            </router-link>
            <router-link
              v-for="s in item.sous"
              :key="s.to"
              :to="s.to"
              class="item-menu"
              :class="{ 'text-white bg-white/10': actif(s.to) }"
            >
              <span>{{ s.icone }}</span><span>{{ s.label }}</span>
            </router-link>
          </div>
        </div>
      </div>
    </nav>

    <div class="flex-1" data-tauri-drag-region />

    <!-- Outils IA -->
    <div class="relative flex items-stretch">
      <button
        class="flex items-center gap-1 px-2.5 text-xs transition-colors"
        :class="sectionActive('/ia', '/config/prompts') || ouvert === 'ia' ? 'text-white bg-white/10' : 'text-gray-400 hover:text-white hover:bg-white/5'"
        title="Outils IA"
        @click="basculer('ia')"
      >
        <span class="text-sm leading-none">🧠</span>
        <span class="text-[8px] transition-transform" :class="ouvert === 'ia' ? 'rotate-180' : ''">▼</span>
      </button>
      <div v-if="ouvert === 'ia'" class="menu-deroulant" @click="fermer">
        <router-link v-for="s in outilsIa" :key="s.to" :to="s.to" class="item-menu" :class="{ 'text-white bg-white/10': actif(s.to) }">
          <span>{{ s.icone }}</span><span>{{ s.label }}</span>
        </router-link>
      </div>
    </div>

    <!-- Système : Paramètres + Données -->
    <div class="relative flex items-stretch">
      <button
        class="flex items-center gap-1 px-2.5 text-xs transition-colors"
        :class="actif('/settings') || actif('/data') || ouvert === 'systeme' ? 'text-white bg-white/10' : 'text-gray-400 hover:text-white hover:bg-white/5'"
        title="Paramètres & Données"
        @click="basculer('systeme')"
      >
        <span class="text-sm leading-none">⚙️</span>
        <span class="text-[8px] transition-transform" :class="ouvert === 'systeme' ? 'rotate-180' : ''">▼</span>
      </button>
      <div v-if="ouvert === 'systeme'" class="menu-deroulant" @click="fermer">
        <router-link v-for="s in systeme" :key="s.to" :to="s.to" class="item-menu" :class="{ 'text-white bg-white/10': actif(s.to) }">
          <span>{{ s.icone }}</span><span>{{ s.label }}</span>
        </router-link>
      </div>
    </div>

    <!-- Contrôles fenêtre (masqués hors Tauri — ex. navigateur de dev) -->
    <div v-if="estTauri" class="flex items-stretch ml-1 border-l border-white/10">
      <button class="controle-fenetre" title="Réduire" @click="fenetre?.minimize()">─</button>
      <button class="controle-fenetre" title="Agrandir" @click="fenetre?.toggleMaximize()">□</button>
      <button class="controle-fenetre controle-fermer" title="Fermer" @click="fenetre?.close()">✕</button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { Window as FenetreTauri } from '@tauri-apps/api/window'

type SousPage = { to: string; icone: string; label: string }
type Principal = { to: string; icone: string; label: string; sous?: SousPage[] }

/// Définition des stratégies (bouton = page signaux, ▾ = sous-pages).
const strategies: Principal[] = [
  { to: '/smc', icone: '📐', label: 'SMC', sous: [{ to: '/smc/definition', icone: '📖', label: 'Définition' }] },
  { to: '/straddle', icone: '⚡', label: 'Straddle', sous: [{ to: '/straddle/definition', icone: '📖', label: 'Définition' }] },
  { to: '/rockets', icone: '🚀', label: 'Rockets', sous: [
    { to: '/rockets/definition', icone: '📖', label: 'Définition' },
    { to: '/rockets/scanner', icone: '🔭', label: 'Scanner' },
  ] },
]

const principaux: Principal[] = [
  { to: '/', icone: '🏠', label: 'Dashboard' },
  { to: '/smc/graphiques', icone: '📈', label: 'Graphiques' },
  ...strategies,
  { to: '/history', icone: '📜', label: 'Historiques des trades' },
  { to: '/presse', icone: '📰', label: 'Presse' },
]

const outilsIa: SousPage[] = [
  { to: '/ia/chart', icone: '🖼️', label: 'Analyse graphique' },
  { to: '/ia/coach', icone: '💬', label: 'Coach IA' },
  { to: '/config/prompts', icone: '✏️', label: 'Prompts IA' },
]

const systeme: SousPage[] = [
  { to: '/settings', icone: '⚙️', label: 'Paramètres' },
  { to: '/data', icone: '📦', label: 'Données' },
]

const route = useRoute()
const router = useRouter()
const ouvert = ref<string | null>(null)
const fenetre = ref<FenetreTauri | null>(null)
const estTauri = computed(() => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window)

onMounted(async () => {
  if (estTauri.value) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      fenetre.value = getCurrentWindow()
    } catch {
      fenetre.value = null // dégradation silencieuse (contrôles masqués)
    }
  }
  document.addEventListener('mousedown', surClicExterieur)
  document.addEventListener('keydown', surEchap)
})
onUnmounted(() => {
  document.removeEventListener('mousedown', surClicExterieur)
  document.removeEventListener('keydown', surEchap)
})

function actif(to: string): boolean {
  return to === '/' ? route.path === '/' : route.path.startsWith(to)
}
function sectionActive(...prefixes: string[]): boolean {
  return prefixes.some((p) => route.path.startsWith(p))
}
function sousPageActive(item: Principal): boolean {
  return item.sous?.some((s) => actif(s.to)) ?? false
}
function basculer(cle: string) {
  ouvert.value = ouvert.value === cle ? null : cle
}
function fermer() {
  ouvert.value = null
}
function aller(to: string) {
  ouvert.value = null
  router.push(to)
}

/// Ferme le déroulant si le clic tombe hors de la barre.
function surClicExterieur(e: MouseEvent) {
  if (ouvert.value && e.target instanceof Element && !e.target.closest('.title-bar')) fermer()
}
function surEchap(e: KeyboardEvent) {
  if (e.key === 'Escape') fermer()
}
</script>

<style scoped>
.title-bar {
  height: 40px;
  flex-shrink: 0;
}
.menu-deroulant {
  @apply absolute right-0 top-full mt-1 z-50 min-w-[190px] rounded-lg border border-white/10 bg-gray-800/95 backdrop-blur py-1 shadow-xl shadow-black/40;
}
.menu-deroulant.aligne-gauche {
  left: 0;
  right: auto;
}
.item-menu {
  @apply flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-300 hover:text-white hover:bg-white/10 transition-colors;
}
.controle-fenetre {
  @apply w-11 flex items-center justify-center text-sm text-gray-400 hover:text-white hover:bg-white/10 transition-colors;
}
.controle-fermer {
  @apply hover:bg-red-600 hover:text-white;
}
</style>
