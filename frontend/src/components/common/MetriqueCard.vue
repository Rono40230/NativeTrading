<template>
  <div class="rounded-lg border bg-black/20 p-3 flex flex-col gap-0.5"
       :class="borderClass">
    <span class="text-[10px] uppercase text-gray-500 font-semibold">{{ label }}</span>
    <span class="text-lg font-bold" :class="valueClass">{{ value }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  value: string
  color?: 'green' | 'yellow' | 'red' | 'default'
}>()

const valueClass = computed(() => ({
  'text-emerald-400': props.color === 'green',
  'text-yellow-400':  props.color === 'yellow',
  'text-red-400':     props.color === 'red',
  'text-white':       !props.color || props.color === 'default',
}))

const borderClass = computed(() => ({
  'border-emerald-500/30': props.color === 'green',
  'border-yellow-500/30':  props.color === 'yellow',
  'border-red-500/30':     props.color === 'red',
  'border-white/10':       !props.color || props.color === 'default',
}))
</script>
