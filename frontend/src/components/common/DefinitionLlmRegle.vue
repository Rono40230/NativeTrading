<template>
  <div class="flex items-center gap-2 text-sm">
    <span :class="couleur" class="shrink-0 w-4">{{ icon }}</span>
    <span class="text-gray-300">{{ label }}</span>
  </div>
</template>

<script setup lang="ts">
defineProps<{ icon: string; label: string; couleur: string }>()
</script>
