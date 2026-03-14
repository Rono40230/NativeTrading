<template>
  <div class="glass-card p-5">
    <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Derniers signaux</h2>
    <div v-if="signalStore.chargement" class="text-gray-500 text-sm text-center py-4">
      Chargement...
    </div>
    <div v-else-if="signalStore.signaux.length === 0" class="text-gray-500 text-sm text-center py-6">
      Aucun signal enregistré — lancez une stratégie pour commencer
    </div>
    <div v-else class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-gray-500 text-xs uppercase border-b border-white/10">
            <th class="pb-2 text-left">Asset</th>
            <th class="pb-2 text-left">TF</th>
            <th class="pb-2 text-left">Direction</th>
            <th class="pb-2 text-right">Score</th>
            <th class="pb-2 text-right">Entrée</th>
            <th class="pb-2 text-left">Stratégie</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="signal in signalStore.signaux.slice(0, 8)"
            :key="signal.id"
            class="border-b border-white/5 hover:bg-white/5"
          >
            <td class="py-2 font-medium">{{ signal.asset }}</td>
            <td class="py-2 text-gray-400">{{ signal.timeframe }}</td>
            <td class="py-2">
              <span class="px-2 py-0.5 rounded text-xs" :class="badgeDirection(signal.direction)">
                {{ signal.direction }}
              </span>
            </td>
            <td class="py-2 text-right">{{ signal.score.toFixed(1) }}</td>
            <td class="py-2 text-right font-mono">{{ formatUsd(signal.prix_entree) }}</td>
            <td class="py-2 text-gray-400 text-xs">{{ signal.strategie }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useSignalStore } from '@/stores/signal.store'

const signalStore = useSignalStore()

function formatUsd(v: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 }).format(v)
}

function badgeDirection(dir: string): string {
  if (dir === 'Long') return 'bg-emerald-500/20 text-emerald-300'
  if (dir === 'Short') return 'bg-red-500/20 text-red-300'
  return 'bg-yellow-500/20 text-yellow-300'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
