<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
        ⚡ Surveillance des événements de volatilité — Stratégie Straddle
      </p>
      <button
        class="text-[10px] font-semibold text-yellow-300 hover:text-yellow-100 border border-yellow-500/40 hover:border-yellow-400/70 rounded-lg px-2.5 py-1 transition-all hover:bg-yellow-500/10"
        @click="modalOuverte = true"
      >Opportunités ▸</button>
    </div>

    <!-- Placeholder -->
    <div class="flex items-center justify-center py-6 text-xs text-gray-600 italic">
      Implémentation à venir — affichage des créneaux ATR élevé, nouvelles macro, événements Fed/BCE
    </div>
  </div>

  <StraddleVolatiliteModal :visible="modalOuverte" @close="modalOuverte = false" />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import StraddleVolatiliteModal from './StraddleVolatiliteModal.vue'

const modalOuverte = ref(false)
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>

