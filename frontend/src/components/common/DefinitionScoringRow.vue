<template>
  <div class="flex flex-col xl:flex-row xl:items-center justify-between gap-1.5 xl:gap-3 rounded-lg bg-black/20 border border-white/5 px-3 py-2">
    <div class="flex items-center justify-between xl:justify-start gap-2 xl:w-40 shrink-0">
      <span class="text-xs xl:text-sm font-semibold text-white">{{ label }}</span>
      <span
        v-if="maxPts !== undefined"
        class="text-[10px] xl:text-xs font-bold xl:hidden"
        :class="isDynamic ? 'text-green-400' : 'text-blue-300'"
      >
        +{{ maxPts }}
      </span>
    </div>
    <div class="flex flex-wrap gap-1.5 flex-1">
      <span v-for="(badge, index) in detailBadges" :key="index" class="text-[10px] xl:text-xs bg-white/5 text-gray-300 border border-white/10 px-2 py-0.5 rounded-md">
        {{ badge.trim() }}
      </span>
    </div>
    <div
      v-if="maxPts !== undefined"
      class="text-xs font-bold shrink-0 hidden xl:block"
      :class="isDynamic ? 'text-green-400' : 'text-blue-300'"
    >
      +{{ maxPts }}pts
      <span v-if="isDynamic" class="text-green-400/60 text-[9px] ml-0.5">live</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  detail: string
  maxPts?: number
  isDynamic?: boolean
}>()

const detailBadges = computed(() => {
  return props.detail.split(' | ').filter(b => b.trim().length > 0)
})
</script>