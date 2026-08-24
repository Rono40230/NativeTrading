<template>
  <div class="flex-1 min-h-0 flex flex-col gap-3">
    <div class="glass-card px-4 py-3 shrink-0">
      <div class="text-xs uppercase text-gray-500 font-semibold tracking-wider mb-2">Setups en formation</div>
      <SetupsFormationPanel strategie="SMC" />
    </div>
    <SignauxTableau strategie="SMC" />
  </div>
</template>

<script setup lang="ts">
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SetupsFormationPanel from '@/components/common/SetupsFormationPanel.vue'
</script>
