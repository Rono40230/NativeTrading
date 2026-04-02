<template>
  <div class="flex flex-col" style="height: calc(100vh - 3rem);">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3 flex-shrink-0">
      <div>
        <h1 class="text-2xl font-bold">🖼️ Chart Import</h1>
        <p class="text-xs text-gray-500 mt-0.5">Glissez un screenshot → analyse vision SMC via Claude Haiku 3.5 (Anthropic)</p>
      </div>
      <div class="flex gap-2 items-center">
        <span
          class="text-xs px-2 py-1 rounded-full font-semibold"
          :class="{
            'bg-emerald-900/50 text-emerald-300': anthropicStatut === 'ok',
            'bg-red-900/50 text-red-300': anthropicStatut === 'credits-insuffisants',
            'bg-gray-800 text-gray-500': anthropicStatut === 'non-configure',
          }"
          :title="anthropicStatut === 'credits-insuffisants' ? 'Rechargez vos crédits sur console.anthropic.com' : ''"
        >
          <span v-if="anthropicStatut === 'ok'">🔑 Anthropic OK</span>
          <span v-else-if="anthropicStatut === 'credits-insuffisants'">⚠️ Crédits épuisés</span>
          <span v-else>🔑 Pas de clé</span>
        </span>
      </div>
    </div>

    <!-- Panel prend tout l'espace restant -->
    <div class="flex-1 min-h-0">
      <ChartImportPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import ChartImportPanel from '@/components/common/ChartImportPanel.vue'
import { anthropicStatutChart as anthropicStatut } from '@/composables/useChartImport'

onMounted(async () => {
  const cfg = await apiService.obtenirConfig('anthropic_api_key')
  if (cfg?.valeur && cfg.valeur.length > 0) anthropicStatut.value = 'ok'
})
</script>
