<template>
  <!-- Bloc « Rapport d'activité » : bouton simple (14/09) — les anciens
       raccourcis SMC/Straddle/Rockets vivent en onglets dans la page. -->
  <div
    class="rounded-xl border backdrop-blur-sm p-2.5 flex items-center gap-1.5 shrink-0 cursor-pointer transition-colors bg-teal-500/10 border-teal-500/25 hover:border-teal-400/50"
    title="Ouvrir le rapport d'activité (toutes les stratégies)"
    @click="router.push('/analyses')"
  >
    <span class="text-sm leading-none">📊</span>
    <span class="text-[11px] font-bold uppercase tracking-wider text-white truncate">Rapport d'activité</span>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>
