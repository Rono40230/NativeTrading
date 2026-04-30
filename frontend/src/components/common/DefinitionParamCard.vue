<template>
  <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5 relative">
    <span class="text-xs text-gray-500">{{ label }}</span>
    <span class="text-white font-bold text-xl">{{ value }}</span>
    <span
      v-if="badge === 'dynamic'"
      class="absolute top-1.5 right-2 text-[9px] font-semibold text-green-400 bg-green-500/10 border border-green-500/20 px-1.5 py-0.5 rounded-full"
    >
      live
    </span>
    <span
      v-else-if="badge === 'formula'"
      class="absolute top-1.5 right-2 text-[9px] font-semibold text-blue-400 bg-blue-500/10 border border-blue-500/20 px-1.5 py-0.5 rounded-full"
    >
      formule
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label: string
  value: string | number
  badge?: 'dynamic' | 'formula'
}>()
</script>
