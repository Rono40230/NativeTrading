<template>
  <!-- Bouton toggle (toujours visible) -->
  <button
    class="absolute top-1/2 -translate-y-1/2 w-7 h-16 z-40
           rounded-l-lg bg-[#0a0e27]/95 border border-r-0 border-white/10
           flex items-center justify-center text-white hover:text-white
           transition-all duration-200"
    :style="{ right: open ? '512px' : '0px' }"
    :title="open ? 'Masquer la sidebar IA' : 'Afficher la sidebar IA'"
    @click="emit('toggle')"
  >
    <span class="text-sm select-none">{{ open ? '›' : '‹' }}</span>
  </button>

  <!-- Drawer overlay — réservé aux analyses IA par stratégie (étape 6 :
       contenu actuel retiré, ménage 2026-08-23 — Prédiction IA / Score SMC v1 /
       Monitoring ML étaient alimentés par les modèles purgés du 15/08). -->
  <div
    v-if="false"
    class="absolute right-0 top-0 bottom-0 z-30 w-[512px]"
  >
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  asset: string
  timeframe: string
  open: boolean
}>()
const _ = props
const emit = defineEmits<{ toggle: [] }>()
</script>
