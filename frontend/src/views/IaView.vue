<template>
  <!-- Page « Fonctionnalités IA » — les prompts (seul outil conservé :
       l'analyse graphique et le coach IA ont été retirés le 06/09 sur
       décision propriétaire — « je ne m'en servirai jamais »). -->
  <div class="flex flex-col gap-3 h-[calc(100vh-5.5rem)] overflow-hidden bg-violet-500/5 rounded-xl px-3 py-2">
    <div class="flex-1 min-h-0 overflow-y-auto">
      <PromptsIAView />
    </div>
  </div>
</template>

<script setup lang="ts">
import PromptsIAView from '@/views/PromptsIAView.vue'
</script>
