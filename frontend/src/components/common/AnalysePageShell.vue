<template>
  <!-- Coquille commune des pages d'analyse plein écran (08/09, standard des
       trois stratégies) : retour + titre + bandeau synthèse COLLANT, puis
       sections empilées défilantes. Plus d'onglets, plus de modale. -->
  <div class="h-full flex flex-col min-h-0">
    <div class="glass-card shrink-0 px-4 py-2.5 flex items-center gap-4 flex-wrap z-10">
      <button
        class="text-xs px-2.5 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors shrink-0"
        @click="router.push(retourRoute)"
      >← {{ retourLabel }}</button>
      <h1 class="text-lg font-bold text-white shrink-0">{{ titre }}</h1>
      <!-- Bandeau synthèse : l'essentiel reste visible pendant tout le scroll -->
      <div class="ml-auto flex items-center gap-4 flex-wrap">
        <div v-for="s in synthese" :key="s.label" class="text-center">
          <div class="text-base font-bold leading-tight" :class="s.classe ?? 'text-white'">{{ s.valeur }}</div>
          <div class="text-[9px] uppercase tracking-wide text-white">{{ s.label }}</div>
        </div>
        <slot name="actions" />
      </div>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-4 flex flex-col gap-6">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

defineProps<{
  titre: string
  retourLabel: string
  retourRoute: string
  /// Bandeau collant : [{label, valeur, classe?}] — l'essentiel, toujours visible.
  synthese: { label: string; valeur: string | number; classe?: string }[]
}>()

const router = useRouter()
</script>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.15); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.25); }
</style>
