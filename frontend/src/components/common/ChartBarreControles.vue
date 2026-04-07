<template>
  <div class="flex items-center gap-3">
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
      {{ analyseEnCours ? '🔍 Analyse...' : '🔍 Analyse SMC par l\'IA' }}
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  chargement: boolean
  analyseEnCours: boolean
}>()

defineEmits<{
  'actualiser': []
  'analyser': []
}>()
</script>
