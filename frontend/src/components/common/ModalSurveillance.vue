<template>
  <Teleport to="body">
    <Transition name="modal-surv">
      <div v-if="visible" class="fixed inset-0 z-[9970] flex items-center justify-center">
        <div class="absolute inset-0 bg-black/70 backdrop-blur-sm" @click="emit('close')" />
        <div class="relative w-[96vw] max-h-[93vh] rounded-2xl border border-white/10 bg-[#080c1f] shadow-2xl flex flex-col overflow-hidden">
          <!-- Header -->
          <div class="flex items-center justify-between px-5 py-3 border-b border-white/10 shrink-0">
            <p class="text-[12px] font-semibold uppercase tracking-widest text-white">{{ titre }}</p>
            <button
              class="text-gray-400 hover:text-white text-xl transition-colors leading-none px-1"
              @click="emit('close')"
            >✕</button>
          </div>
          <!-- Contenu -->
          <div class="flex-1 overflow-y-auto p-5">
            <slot />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
defineProps<{ visible: boolean; titre: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<style scoped>
.modal-surv-enter-active, .modal-surv-leave-active { transition: opacity 0.15s, transform 0.15s; }
.modal-surv-enter-from, .modal-surv-leave-to { opacity: 0; transform: scale(0.97); }
.modal-surv-enter-to, .modal-surv-leave-from { opacity: 1; transform: scale(1); }
</style>
