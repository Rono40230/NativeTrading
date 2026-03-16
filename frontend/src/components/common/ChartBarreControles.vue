<template>
  <div class="flex flex-wrap items-center gap-3">
    <div class="flex rounded-lg overflow-hidden border border-white/10">
      <button
        v-for="a in assets"
        :key="a"
        class="px-4 py-2 text-sm font-medium transition-colors"
        :class="selectedAsset === a ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
        @click="$emit('changer-asset', a)"
      >
        {{ a }}
      </button>
    </div>

    <div class="flex rounded-lg overflow-hidden border border-white/10">
      <button
        v-for="tf in timeframes"
        :key="tf"
        class="px-3 py-2 text-sm font-medium transition-colors"
        :class="selectedTimeframe === tf ? 'bg-blue-600 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
        @click="$emit('changer-timeframe', tf)"
      >
        {{ tf }}
      </button>
    </div>

    <button
      class="ml-auto px-4 py-2 text-sm rounded-lg bg-white/5 border border-white/10 text-gray-300 hover:bg-white/10 transition-colors"
      :disabled="chargement"
      @click="$emit('actualiser')"
    >
      {{ chargement ? '⏳ Chargement...' : '🔄 Actualiser' }}
    </button>

    <button
      class="px-4 py-2 text-sm rounded-lg bg-purple-600/20 border border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40 transition-colors"
      :disabled="analyseEnCours"
      @click="$emit('analyser')"
    >
      {{ analyseEnCours ? '🔍 Analyse...' : '🔍 Analyser (IA)' }}
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  assets: string[]
  timeframes: string[]
  selectedAsset: string
  selectedTimeframe: string
  chargement: boolean
  analyseEnCours: boolean
}>()

defineEmits<{
  'changer-asset': [asset: string]
  'changer-timeframe': [tf: string]
  'actualiser': []
  'analyser': []
}>()
</script>
