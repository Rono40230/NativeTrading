<template>
  <aside class="sidebar flex flex-col w-64 min-h-screen bg-[#0d1117] border-r border-white/8 shrink-0 fixed left-0 top-0 h-full z-40 overflow-y-auto">

    <!-- Navigation -->
    <nav class="flex flex-col px-3 py-3 flex-1 gap-1">
      <template v-for="(item, i) in nav" :key="i">

        <!-- Groupe permanent -->
        <template v-if="estGroupe(item)">
          <!-- Séparateur + Titre de section -->
          <div class="flex items-center gap-2 px-2 pt-4 pb-1.5">
            <div class="h-px flex-1" :class="accentBorder(item.groupe)" />
            <span class="text-[10px] font-bold uppercase tracking-widest" :class="accentText(item.groupe)">
              {{ item.groupe }}
            </span>
            <span v-if="item.groupe === 'Stratégie SMC' && nouvelleAnalyse"
              class="w-1.5 h-1.5 rounded-full bg-orange-400 animate-pulse"
              title="Nouvelle analyse SMC disponible"
            />
            <div class="h-px flex-1" :class="accentBorder(item.groupe)" />
          </div>
          <!-- Liens du groupe dans une carte -->
          <div class="flex flex-col gap-0.5 rounded-xl bg-white/[0.03] border border-white/5 p-1 mb-1">
            <RouterLink
              v-for="(sub, j) in item.liens"
              :key="j"
              :to="sub.to"
              class="flex items-center gap-2.5 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-all px-3 py-2"
              active-class="text-white font-medium"
              :style="activeStyle(item.groupe)"
            >
              <span class="text-base leading-none w-5 text-center">{{ sub.icone }}</span>
              <span>{{ sub.label }}</span>
            </RouterLink>
          </div>
        </template>

        <!-- Lien direct (Dashboard, Toutes stratégies, Paramètres) -->
        <RouterLink
          v-else
          :to="item.to"
          class="flex items-center gap-2.5 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-white/8 transition-all px-3 py-2.5"
          active-class="bg-white/10 text-white font-medium"
        >
          <span class="text-base leading-none w-5 text-center">{{ item.icone }}</span>
          <span>{{ item.label }}</span>
        </RouterLink>

      </template>
    </nav>

  </aside>
</template>

<script setup lang="ts">

import { RouterLink } from 'vue-router'
import { useSmcAnalyseNotif } from '@/composables/useSmcAnalyseNotif'

const { nouvelleAnalyse } = useSmcAnalyseNotif()

type LienSimple = { to: string; icone: string; label: string }
type LienGroupe = { groupe: string; icone: string; liens: LienSimple[] }
type NavItem = LienSimple | LienGroupe

function estGroupe(item: NavItem): item is LienGroupe { return 'groupe' in item }

const ACCENT: Record<string, { text: string; border: string; active: string }> = {
  'Général':              { text: 'text-cyan-400',   border: 'bg-cyan-500/40',   active: 'background: rgba(6,182,212,0.12)'   },
  'Stratégie Rockets':    { text: 'text-orange-400', border: 'bg-orange-500/40', active: 'background: rgba(249,115,22,0.12)' },
  'Stratégie SMC':        { text: 'text-blue-400',   border: 'bg-blue-500/40',   active: 'background: rgba(59,130,246,0.12)'  },
  'Stratégie Volatilité': { text: 'text-yellow-400', border: 'bg-yellow-500/40', active: 'background: rgba(234,179,8,0.12)'   },
  'Outils IA':            { text: 'text-purple-400', border: 'bg-purple-500/40', active: 'background: rgba(168,85,247,0.12)'  },
  'Configuration':        { text: 'text-gray-400',   border: 'bg-gray-500/30',   active: 'background: rgba(107,114,128,0.12)' },
}

function accentText(groupe: string) { return ACCENT[groupe]?.text ?? 'text-gray-500' }
function accentBorder(groupe: string) { return ACCENT[groupe]?.border ?? 'bg-white/10' }
function activeStyle(groupe: string) { return ACCENT[groupe]?.active ?? '' }

const nav: NavItem[] = [
  { to: '/', icone: '🏠', label: 'Dashboard' },

  // ── Général ───────────────────────────────────────────────────────────────
  {
    groupe: 'Général', icone: '🗂️',
    liens: [
      { to: '/smc/graphiques', icone: '📈', label: 'Graphiques' },
      { to: '/history',        icone: '📜', label: 'Historique des positions' },
      { to: '/lexique',        icone: '📖', label: 'Lexique' },
    ]
  },

  // ── Stratégies ────────────────────────────────────────────────────────────
  {
    groupe: 'Stratégie Rockets', icone: '🚀',
    liens: [
      { to: '/rockets',            icone: '⚡', label: 'Signaux' },
      { to: '/rockets/definition', icone: '🤖', label: 'Définition & Prompt IA' },
    ]
  },
  {
    groupe: 'Stratégie SMC', icone: '📐',
    liens: [
      { to: '/smc',            icone: '⚡', label: 'Signaux' },
      { to: '/smc/analyser',   icone: '📊', label: 'Analyser un setup' },
      { to: '/smc/definition', icone: '🤖', label: 'Définition & Prompt IA' },
    ]
  },
  {
    groupe: 'Stratégie Volatilité', icone: '⚡',
    liens: [
      { to: '/straddle/signaux',    icone: '⚡', label: 'Signaux' },
      { to: '/smc/backtests',       icone: '🧪', label: 'Backtests' },
      { to: '/heatmap',             icone: '🔥', label: 'Heatmap' },
      { to: '/straddle/definition', icone: '🤖', label: 'Définition & Prompt IA' },
    ]
  },

  // ── Outils IA ─────────────────────────────────────────────────────────────
  {
    groupe: 'Outils IA', icone: '🧠',
    liens: [
      { to: '/ia/chart', icone: '🖼️', label: 'Analyse graphique' },
      { to: '/ia/coach', icone: '💬', label: 'Coach IA' },
    ]
  },

  // ── Système ───────────────────────────────────────────────────────────────
  {
    groupe: 'Configuration', icone: '⚙️',
    liens: [
      { to: '/settings',       icone: '⚙️', label: 'Paramètres' },
      { to: '/data',           icone: '📦', label: 'Import des données' },
      { to: '/config/prompts', icone: '🧠', label: 'Configuration de l\'IA' },
    ]
  },
]

</script>

<style scoped>
.sidebar {
  transform: translateX(-100%);
  opacity: 0;
  transition: transform 0.2s ease, opacity 0.2s ease;
}
</style>
