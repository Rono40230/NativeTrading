<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div class="rounded-xl border border-white/10 p-6 w-full max-w-xl" style="background: #0d1117;">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold text-white">⚡ Analyse Straddle</h3>
        <button class="text-gray-400 hover:text-white text-xl" @click="$emit('close')">×</button>
      </div>
      <p class="text-sm text-gray-400 mb-5">
        Le Straddle est un <strong class="text-white">outil de recherche de créneaux</strong> de forte
        volatilité récurrente. Il ne génère pas de signaux live — il identifie des plages horaires à
        valider en backtest.
      </p>
      <div class="grid grid-cols-3 gap-3 mb-5">
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p class="text-2xl font-bold text-white">{{ stats.total }}</p>
          <p class="text-xs text-gray-400 mt-1">Signaux</p>
        </div>
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p
            class="text-2xl font-bold"
            :class="stats.winrate >= 55 ? 'text-emerald-400' : stats.winrate >= 40 ? 'text-yellow-400' : 'text-red-400'"
          >{{ stats.winrate }}%</p>
          <p class="text-xs text-gray-400 mt-1">Win Rate</p>
        </div>
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p class="text-2xl font-bold text-blue-400">{{ stats.actifs }}</p>
          <p class="text-xs text-gray-400 mt-1">Actifs</p>
        </div>
      </div>
      <RouterLink
        to="/straddle"
        class="block w-full text-center py-2.5 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition text-sm"
        @click="$emit('close')"
      >
        → Aller sur la vue Straddle (créneaux &amp; backtest)
      </RouterLink>
    </div>
  </div>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'

defineProps<{
  open: boolean
  stats: { total: number; winrate: number; actifs: number }
}>()

defineEmits<{ close: [] }>()
</script>
