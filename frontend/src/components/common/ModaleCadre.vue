<template>
  <!-- Cadre de modale commun (design app) : voile sombre, panneau ardoise,
       titre + fermeture. Le contenu vit dans le slot. -->
  <div
    class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6"
    @click.self="$emit('fermer')"
    @click.stop
  >
    <div
      class="w-full max-h-[85vh] overflow-y-auto p-5 rounded-xl border border-white/15 bg-slate-900 shadow-2xl"
      :class="large ? 'max-w-3xl' : 'max-w-lg'"
    >
      <div class="flex items-center justify-between mb-3 sticky top-0">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">{{ titre }}</h2>
        <button
          class="text-xs px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors"
          @click="$emit('fermer')"
        >✕ Fermer</button>
      </div>
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ titre: string; large?: boolean }>()
defineEmits<{ (e: 'fermer'): void }>()
</script>
