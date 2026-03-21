<template>
  <span
    class="inline-flex items-center align-middle ml-1 cursor-help"
    @mouseenter="afficher"
    @mouseleave="masquer"
  >
    <span class="text-gray-500 hover:text-blue-400 text-[10px] leading-none select-none transition-colors duration-150">ⓘ</span>
    <Teleport v-if="visible" to="body">
      <div
        class="fixed z-[9999] w-56 px-3 py-2 text-xs text-gray-200 bg-gray-950 border border-white/10 rounded-lg shadow-2xl pointer-events-none whitespace-normal leading-relaxed -translate-x-1/2 -translate-y-full"
        :style="{ top: `${pos.top}px`, left: `${pos.left}px` }"
      >{{ texte }}</div>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{ texte: string }>()

const visible = ref(false)
const pos = ref({ top: 0, left: 0 })

function afficher(e: MouseEvent) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  pos.value = { top: rect.top - 10, left: rect.left + rect.width / 2 }
  visible.value = true
}
function masquer() {
  visible.value = false
}
</script>
