<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="$emit('close')"
      >
        <div class="relative w-full max-w-2xl rounded-2xl border border-white/10 p-6 shadow-2xl" style="background: #0b0f28">
          <div class="flex items-center justify-between mb-6">
            <div>
              <p class="text-[10px] uppercase tracking-widest text-gray-500 mb-1">Stratégie Volatilité</p>
              <h2 class="text-lg font-bold text-white">⚡ Opportunités de volatilité</h2>
            </div>
            <button
              class="w-8 h-8 flex items-center justify-center rounded-full text-gray-400 hover:text-white hover:bg-white/10 transition-colors"
              @click="$emit('close')"
            >✕</button>
          </div>

          <!-- Contenu à venir -->
          <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
            <span class="text-3xl">⚡</span>
            <p class="text-sm text-gray-400">Contenu à venir</p>
            <p class="text-xs text-gray-600">Créneaux ATR élevé, événements macro Fed/BCE, alertes de volatilité imminente</p>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
defineProps<{ visible: boolean }>()
defineEmits<{ close: [] }>()
</script>

<style scoped>
.modal-enter-active, .modal-leave-active { transition: opacity 0.2s, transform 0.2s; }
.modal-enter-from, .modal-leave-to { opacity: 0; transform: scale(0.97); }
.modal-enter-to, .modal-leave-from { opacity: 1; transform: scale(1); }
</style>
