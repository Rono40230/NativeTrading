<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">🖼️ Chart Import</h1>
        <p class="text-xs text-gray-500 mt-0.5">Glissez un screenshot → analyse vision SMC via llama3.2-vision</p>
      </div>
    </div>
    <ChartImportPanel />
  </div>
</template>

<script setup lang="ts">
import ChartImportPanel from '@/components/common/ChartImportPanel.vue'
</script>
