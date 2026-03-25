<template>
  <aside class="sidebar flex flex-col w-52 min-h-screen bg-gray-900 border-r border-white/10 shrink-0 fixed left-0 top-0 h-full z-40">
    <!-- Navigation -->
    <nav class="flex flex-col gap-0.5 px-3 py-4 flex-1">
      <template v-for="(item, i) in nav" :key="i">
        <!-- Groupe collapsible -->
        <template v-if="estGroupe(item)">
          <button
            class="flex items-center gap-2 px-3 pt-3 pb-1 w-full hover:text-white/80 transition-colors"
            @click="toggleGroupe(item.groupe)"
          >
            <span class="text-sm leading-none">{{ item.icone }}</span>
            <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider flex-1 text-left">{{ item.groupe }}</span>
            <span v-if="item.groupe === 'SMC' && nouvelleAnalyse" class="w-2 h-2 rounded-full bg-orange-400 animate-pulse" title="Nouvelle analyse SMC disponible" />
            <span class="text-gray-600 text-[10px] transition-transform duration-200" :class="groupesOuverts[item.groupe] ? 'rotate-180' : ''">▼</span>
          </button>
          <div v-show="groupesOuverts[item.groupe]" class="flex flex-col gap-0.5">
            <RouterLink
              v-for="(sub, j) in item.liens"
              :key="j"
              :to="sub.to"
              class="flex items-center gap-3 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-colors pl-8 pr-3 py-2"
              active-class="bg-white/10 text-white font-medium"
            >
              <span class="text-base leading-none">{{ sub.icone }}</span>
              <span>{{ sub.label }}</span>
            </RouterLink>
          </div>
        </template>
        <!-- Lien normal -->
        <RouterLink
          v-else
          :to="item.to"
          class="flex items-center gap-3 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-colors px-3 py-2.5"
          active-class="bg-white/10 text-white font-medium"
        >
          <span class="text-base leading-none">{{ item.icone }}</span>
          <span>{{ item.label }}</span>
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
import { computed, reactive } from 'vue'
import { RouterLink } from 'vue-router'
import { useSmcAnalyseNotif } from '@/composables/useSmcAnalyseNotif'

const { nouvelleAnalyse } = useSmcAnalyseNotif()

type LienSimple = { to: string; icone: string; label: string }
type LienGroupe = { groupe: string; icone: string; liens: LienSimple[] }
type NavItem = LienSimple | LienGroupe

function estGroupe(item: NavItem): item is LienGroupe { return 'groupe' in item }

const nav: NavItem[] = [
  { to: '/', icone: '🏠', label: 'Dashboard' },

  // ── Stratégies ────────────────────────────────────────────────────────────
  {
    groupe: 'Rockets', icone: '🚀',
    liens: [
      { to: '/rockets', icone: '📡', label: 'Veille & Historique' },
    ]
  },
  {
    groupe: 'SMC', icone: '📐',
    liens: [
      { to: '/smc',           icone: '⚡', label: 'Signaux actifs' },
      { to: '/smc/analyser',  icone: '📊', label: 'Analyser' },
      { to: '/smc/graphiques',icone: '📈', label: 'Graphiques' },
      { to: '/lexique',       icone: '📖', label: 'Lexique SMC' },
    ]
  },
  {
    groupe: 'Straddle', icone: '⚡',
    liens: [
      { to: '/straddle',          icone: '🔍', label: 'Créneaux volatilité' },
      { to: '/straddle/backtest', icone: '🧪', label: 'Backtest' },
      { to: '/heatmap',           icone: '🔥', label: 'Heatmap' },
      { to: '/data',              icone: '📦', label: 'Données' },
    ]
  },

  // ── Performance ──────────────────────────────────────────────────────────
  { to: '/pnl',     icone: '💰', label: 'P&L' },
  { to: '/history', icone: '📜', label: 'Historique' },

  // ── Outils IA ─────────────────────────────────────────────────────────────
  {
    groupe: 'Outils IA', icone: '🧠',
    liens: [
      { to: '/ia/chart', icone: '🖼️',  label: 'Analyse graphique' },
      { to: '/ia/coach', icone: '💬', label: 'Coach IA' },
    ]
  },

  // ── Système ───────────────────────────────────────────────────────────────
  { to: '/settings', icone: '⚙️',  label: 'Paramètres' },
]

const groupesOuverts = reactive<Record<string, boolean>>({})

function toggleGroupe(nom: string) {
  groupesOuverts[nom] = !groupesOuverts[nom]
}

const dateActuelle = computed(() =>
  new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' }).format(new Date())
)
</script>

<style scoped>
.sidebar {
  transform: translateX(-100%);
  opacity: 0;
  transition: transform 0.2s ease, opacity 0.2s ease;
}
</style>
