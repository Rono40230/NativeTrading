<template>
  <Teleport to="body">
    <div class="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      <TransitionGroup name="toast">
        <div
          v-for="alerte in alerteStore.alertes"
          :key="alerte.id"
          class="pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-lg shadow-lg min-w-64 max-w-sm border"
          :class="classeAlerte(alerte.type)"
        >
          <span class="text-lg leading-none mt-0.5">{{ iconeAlerte(alerte.type) }}</span>
          <p class="text-sm leading-snug flex-1">{{ alerte.message }}</p>
          <button
            class="shrink-0 w-5 h-5 flex items-center justify-center rounded-full opacity-70 hover:opacity-100 hover:bg-white/20 text-base leading-none transition-colors"
            @click="alerteStore.supprimer(alerte.id)"
            title="Fermer"
          >×</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { useAlerteStore } from '@/stores/alerte.store'
import type { TypeAlerte } from '@/stores/alerte.store'

const alerteStore = useAlerteStore()

function classeAlerte(type: TypeAlerte): string {
  const map: Record<TypeAlerte, string> = {
    success: 'bg-emerald-900/90 border-emerald-500/50 text-emerald-100',
    error:   'bg-red-900/90 border-red-500/50 text-red-100',
    warning: 'bg-yellow-900/90 border-yellow-500/50 text-yellow-100',
    info:    'bg-blue-900/90 border-blue-500/50 text-blue-100',
  }
  return map[type]
}

function iconeAlerte(type: TypeAlerte): string {
  const map: Record<TypeAlerte, string> = {
    success: '✅', error: '❌', warning: '⚠️', info: 'ℹ️',
  }
  return map[type]
}
</script>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
