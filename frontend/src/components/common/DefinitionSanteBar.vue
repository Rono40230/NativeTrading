<template>
  <div class="flex flex-wrap items-center gap-2 px-4 py-2 rounded-lg bg-black/20 border border-white/5">
    <!-- Badge warm start -->
    <span
      v-if="warmStart !== undefined"
      class="flex items-center gap-1 text-xs px-2 py-1 rounded-full border font-medium"
      :class="warmStart
        ? 'text-amber-400 bg-amber-500/10 border-amber-500/20'
        : 'text-green-400 bg-green-500/10 border-green-500/20'"
    >
      <span class="w-1.5 h-1.5 rounded-full inline-block" :class="warmStart ? 'bg-amber-400' : 'bg-green-400'" />
      {{ warmStart ? 'Warm start' : 'Calibration active' }}
    </span>

    <!-- Seuil LLM affiché -->
    <span v-if="seuilLlm !== undefined" class="text-xs text-gray-400">
      Seuil LLM :
      <span class="text-white font-semibold">{{ seuilLlm }}</span>
    </span>

    <!-- Conviction min -->
    <span v-if="convictionMin !== undefined" class="text-xs text-gray-400">
      Conviction min :
      <span class="text-white font-semibold">{{ convictionMin }}/100</span>
    </span>

    <!-- Invalide -->
    <span
      v-if="invalide"
      class="text-xs text-red-400 bg-red-500/10 border border-red-500/20 px-2 py-1 rounded-full"
    >
      ⚠️ Seuils invalides — fallback défaut
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  warmStart?: boolean
  seuilLlm?: string
  convictionMin?: number
  invalide?: boolean
}>()
</script>
