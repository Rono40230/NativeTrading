<template>
  <!-- Dashboard LLM (extrait des anciens onglets de la page prompts — accès
       par le bouton de la tuile IA, 06/09). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-violet-500/5 rounded-xl px-3 py-2">
    <div class="flex-1 min-h-0 overflow-y-auto pr-1">
      <MlInsightsView />
    </div>
  </div>
</template>

<script setup lang="ts">
import MlInsightsView from '@/views/MlInsightsView.vue'
</script>
