<template>
  <!-- Cadre de modale commun (design app) : téléporté au BODY pour être
       centré à l'ÉCRAN, jamais dans le bloc appelant (owner 26/09 — un
       parent avec overflow/transform piégeait le fixed). Voile sombre,
       panneau ardoise, titre + fermeture. Le contenu vit dans le slot. -->
  <Teleport to="body">
    <div
      class="fixed inset-0 z-[70] bg-black/70 flex items-center justify-center"
      :class="plein ? 'p-1.5' : 'p-6'"
      @click.self="$emit('fermer')"
      @click.stop
    >
      <!-- « plein » = RESPONSIVE : l'unité est le VIEWPORT (vw/vh), la
           modale remplit l'écran quelle que soit sa taille — fini les
           max-w/max-h concurrents. -->
      <div
        class="overflow-y-auto p-5 rounded-xl border border-white/15 bg-slate-900 shadow-2xl break-words"
        :class="plein
          ? 'w-[98.5vw] h-[97vh] max-w-none'
          : 'w-full max-h-[85vh] ' + (tresLarge ? 'max-w-6xl' : large ? 'max-w-3xl' : 'max-w-lg')"
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
  </Teleport>
</template>

<script setup lang="ts">
defineProps<{ titre: string; large?: boolean; tresLarge?: boolean; plein?: boolean }>()
defineEmits<{ (e: 'fermer'): void }>()
</script>
