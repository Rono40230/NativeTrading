<template>
  <aside class="flex flex-col w-52 min-h-screen bg-gray-900 border-r border-white/10 shrink-0">
    <!-- Navigation -->
    <nav class="flex flex-col gap-0.5 px-3 py-4 flex-1">
      <template v-for="(lien, i) in liens" :key="i">
        <!-- En-tête de groupe -->
        <div v-if="estGroupe(lien)" class="flex items-center gap-2 px-3 pt-3 pb-1">
          <span class="text-sm leading-none">{{ lien.icone }}</span>
          <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">{{ lien.groupe }}</span>
        </div>
        <!-- Lien normal ou sous-lien -->
        <RouterLink
          v-else
          :to="lien.to"
          class="flex items-center gap-3 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-colors"
          :class="lien.sub ? 'pl-8 pr-3 py-2' : 'px-3 py-2.5'"
          active-class="bg-white/10 text-white font-medium"
        >
          <span class="text-base leading-none">{{ lien.icone }}</span>
          <span>{{ lien.label }}</span>
        </RouterLink>
      </template>
    </nav>

    <!-- Pied de sidebar : date -->
    <div class="px-5 py-4 border-t border-white/10">
      <p class="text-[10px] text-gray-600 text-center">{{ dateActuelle }}</p>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

type LienSimple = { to: string; icone: string; label: string; sub?: boolean }
type LienGroupe = { groupe: string; icone: string }
type Lien = LienSimple | LienGroupe

function estGroupe(l: Lien): l is LienGroupe { return 'groupe' in l }

const liens: Lien[] = [
  { to: '/',             icone: '🏠', label: 'Home' },
  { to: '/charts',       icone: '📈', label: 'Charts' },
  { to: '/pnl',          icone: '💰', label: 'P&L' },
  { to: '/history',      icone: '📜', label: 'History' },
  { to: '/heatmap',      icone: '🔥', label: 'Heatmap' },
  { groupe: 'IAnalyse',  icone: '🧠' },
  { to: '/ia/analyser',  icone: '📊', label: 'Signal',       sub: true },
  { to: '/ia/chart',     icone: '🖼️',  label: 'Chart Import', sub: true },
  { to: '/ia/coach',     icone: '💬', label: 'IA Coach' },
  { to: '/settings',     icone: '⚙️', label: 'Settings' },
]

const dateActuelle = computed(() =>
  new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' }).format(new Date())
)
</script>
