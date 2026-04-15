<template>
  <span
    ref="iconRef"
    class="relative cursor-help inline-flex items-center"
    @mouseenter="montrer"
    @mouseleave="cacher"
  >
    <span class="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-gray-600 text-gray-500 text-[9px] font-bold leading-none select-none">?</span>
    <Teleport to="body">
      <span
        v-show="visible"
        :style="style"
        class="fixed w-52 text-xs bg-gray-900 text-gray-200 border border-white/10 rounded-lg p-2 pointer-events-none z-[9999] shadow-lg whitespace-normal"
      >
        <slot />
      </span>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const iconRef = ref<HTMLElement | null>(null)
const visible = ref(false)
const rect    = ref<DOMRect | null>(null)

function montrer() {
  rect.value    = iconRef.value?.getBoundingClientRect() ?? null
  visible.value = true
}

function cacher() { visible.value = false }

const style = computed(() => {
  if (!rect.value) return {}
  return {
    top:   `${rect.value.bottom + 6}px`,
    right: `${window.innerWidth - rect.value.right}px`,
  }
})
</script>
