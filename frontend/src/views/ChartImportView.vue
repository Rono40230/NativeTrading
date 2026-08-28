<template>
  <div class="flex flex-col" style="height: calc(100vh - 5.5rem);">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3 flex-shrink-0">
      <div>
        <h1 class="text-2xl font-bold">🖼️ Chart Import</h1>
        <p class="text-xs text-gray-500 mt-0.5">Glissez un screenshot → analyse vision SMC — {{ anthropicActifChart ? 'Claude Haiku 3.5 (Anthropic)' : 'qwen2.5vl:7b (local)' }}</p>
      </div>
      <div class="flex gap-2 items-center">
        <button
          class="text-xs px-2 py-1 rounded-full font-semibold transition-all cursor-pointer select-none"
          :class="{
            'bg-emerald-900/50 text-emerald-300 hover:bg-emerald-800/60': anthropicStatut === 'ok' && anthropicActifChart,
            'bg-gray-700/50 text-gray-400 hover:bg-gray-600/50 line-through': anthropicStatut === 'ok' && !anthropicActifChart,
            'bg-red-900/50 text-red-300': anthropicStatut === 'credits-insuffisants',
            'bg-gray-800 text-gray-500': anthropicStatut === 'non-configure',
          }"
          :title="anthropicStatut === 'ok' ? (anthropicActifChart ? 'Cliquer pour désactiver Anthropic' : 'Cliquer pour activer Anthropic') : ''"
          :disabled="anthropicStatut !== 'ok'"
          @click="anthropicStatut === 'ok' && toggleAnthropicChart()"
        >
          <span v-if="anthropicStatut === 'ok' && anthropicActifChart">🔑 Anthropic ON</span>
          <span v-else-if="anthropicStatut === 'ok' && !anthropicActifChart">🔑 Anthropic OFF</span>
          <span v-else-if="anthropicStatut === 'credits-insuffisants'">⚠️ Crédits épuisés</span>
          <span v-else>🔑 Pas de clé</span>
        </button>
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
import { anthropicStatutChart as anthropicStatut, anthropicActifChart, toggleAnthropicChart } from '@/composables/useChartImport'

onMounted(async () => {
  const cfg = await apiService.obtenirConfig('anthropic_api_key')
  if (cfg?.valeur && cfg.valeur.length > 0) anthropicStatut.value = 'ok'
})
</script>
