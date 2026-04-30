<template>
  <span class="relative inline-block">
    <span
      class="underline decoration-dotted decoration-gray-500 cursor-help"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
    >
      <slot />
    </span>
    <Teleport v-if="visible" to="body">
      <div
        class="fixed z-[9999] pointer-events-none"
        :style="{ top: posY + 'px', left: posX + 'px', transform: 'translateX(-50%)' }"
      >
        <div class="w-0 h-0 border-l-[5px] border-r-[5px] border-b-[5px] border-l-transparent border-r-transparent border-b-slate-900 mx-auto" />
        <div class="rounded-lg border border-white/10 bg-slate-900/95 backdrop-blur-md shadow-xl px-3 py-2 max-w-56">
          <p class="text-xs text-white leading-relaxed">{{ definition }}</p>
        </div>
      </div>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{ definition: string }>()

const visible = ref(false)
const posX = ref(0)
const posY = ref(0)

function onMouseEnter(e: MouseEvent) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  posX.value = rect.left + rect.width / 2
  posY.value = rect.bottom + 6
  visible.value = true
}
function onMouseLeave() { visible.value = false }
</script>
