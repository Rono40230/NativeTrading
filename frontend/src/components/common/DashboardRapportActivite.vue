<template>
  <!-- Bloc « Rapport d'activité » (colonne gauche du dashboard, 04/09) :
       3 boutons = analyse directe d'une stratégie ; le clic sur le BLOC
       ouvre la vue d'ensemble (toutes les analyses). -->
  <div
    class="rounded-xl border backdrop-blur-sm p-2.5 flex flex-col gap-1.5 shrink-0 cursor-pointer transition-colors bg-teal-500/10 border-teal-500/25 hover:border-teal-400/50"
    title="Ouvrir le rapport d'activité complet (toutes les stratégies)"
    @click="router.push('/analyses')"
  >
    <div class="flex items-center gap-1.5">
      <span class="text-sm leading-none">📊</span>
      <span class="text-[11px] font-bold uppercase tracking-wider text-white truncate">Rapport d'activité</span>
    </div>
    <div class="grid grid-cols-3 gap-1">
      <button
        v-for="s in STRATEGIES"
        :key="s.id"
        class="text-[9px] font-semibold px-1 py-1 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors"
        :title="`Analyse ${s.label}`"
        @click.stop="router.push(`/analyses?strategie=${s.id}`)"
      >{{ s.label }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const STRATEGIES = [
  { id: 'SMC', label: 'SMC' },
  { id: 'straddle', label: 'Straddle' },
  { id: 'rockets', label: 'Rockets' },
] as const
</script>
