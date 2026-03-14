<template>
  <aside class="flex flex-col w-52 min-h-screen bg-gray-900 border-r border-white/10 shrink-0">
    <!-- Navigation -->
    <nav class="flex flex-col gap-1 px-3 py-4 flex-1">
      <RouterLink
        v-for="lien in liens"
        :key="lien.to"
        :to="lien.to"
        class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-colors"
        active-class="bg-white/10 text-white font-medium"
      >
        <span class="text-base leading-none">{{ lien.icone }}</span>
        <span>{{ lien.label }}</span>
      </RouterLink>
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

const liens = [
  { to: '/',           icone: '🏠', label: 'Home' },
  { to: '/charts',     icone: '📈', label: 'Charts' },
  { to: '/pnl',        icone: '💰', label: 'P&L' },
  { to: '/history',    icone: '📜', label: 'History' },
  { to: '/heatmap',    icone: '🔥', label: 'Heatmap' },
  { to: '/ia/analyser',icone: '🧠', label: 'IA Analyse' },
  { to: '/ia/coach',   icone: '💬', label: 'IA Coach' },
  { to: '/settings',   icone: '⚙️', label: 'Settings' },
]

const dateActuelle = computed(() =>
  new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' }).format(new Date())
)
</script>
