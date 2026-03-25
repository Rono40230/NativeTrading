<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div class="rounded-xl border border-white/10 p-6 w-full max-w-2xl" style="background: #0d1117;">
      <div class="flex items-center justify-between mb-5">
        <h3 class="text-lg font-semibold text-white">🧠 Analyse SMC Directionnel</h3>
        <button class="text-gray-400 hover:text-white text-xl" @click="$emit('close')">×</button>
      </div>

      <!-- KPIs globaux -->
      <div class="grid grid-cols-4 gap-3 mb-5">
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p class="text-xl font-bold text-white">{{ stats.total }}</p>
          <p class="text-xs text-gray-400 mt-1">Signaux</p>
        </div>
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p
            class="text-xl font-bold"
            :class="stats.winrate >= 55 ? 'text-emerald-400' : stats.winrate >= 40 ? 'text-yellow-400' : 'text-red-400'"
          >{{ stats.winrate }}%</p>
          <p class="text-xs text-gray-400 mt-1">Win Rate</p>
        </div>
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p class="text-xl font-bold text-purple-400">{{ stats.convictionMoyenne }}</p>
          <p class="text-xs text-gray-400 mt-1">Conviction LLM moy.</p>
        </div>
        <div class="rounded-lg bg-white/5 p-3 text-center">
          <p class="text-xl font-bold text-blue-400">{{ stats.tauxFiltrage }}%</p>
          <p class="text-xs text-gray-400 mt-1">Filtrés par LLM</p>
        </div>
      </div>

      <!-- Répartition direction -->
      <div class="flex gap-3 mb-5">
        <div class="flex-1 rounded-lg bg-emerald-500/10 border border-emerald-500/20 p-3 text-center">
          <p class="text-lg font-bold text-emerald-400">{{ stats.longs }}</p>
          <p class="text-xs text-gray-400">📈 LONG</p>
        </div>
        <div class="flex-1 rounded-lg bg-red-500/10 border border-red-500/20 p-3 text-center">
          <p class="text-lg font-bold text-red-400">{{ stats.shorts }}</p>
          <p class="text-xs text-gray-400">📉 SHORT</p>
        </div>
      </div>

      <!-- Derniers signaux LLM avec raison -->
      <div v-if="stats.derniersLlm.length > 0" class="space-y-2">
        <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Derniers filtrages LLM</p>
        <div
          v-for="s in stats.derniersLlm"
          :key="s.id"
          class="flex items-start gap-3 rounded-lg px-3 py-2 text-xs"
          :class="s.llm_valide === 1 ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'"
        >
          <span class="shrink-0 font-bold text-white">{{ s.asset }} {{ s.timeframe }}</span>
          <span
            class="shrink-0"
            :class="s.llm_valide === 1 ? 'text-emerald-400' : 'text-red-400'"
          >{{ s.llm_valide === 1 ? '✅' : '🚫' }} {{ s.llm_conviction ?? '—' }}/100</span>
          <span class="text-gray-400 truncate">{{ s.llm_raison ?? '—' }}</span>
        </div>
      </div>
      <p v-else class="text-center text-gray-500 text-sm py-4">Aucun signal SMC avec données LLM</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Signal } from '@/services/api.service'
import type { ComputedRef } from 'vue'

interface SmcStats {
  total: number
  winrate: number
  convictionMoyenne: number
  tauxFiltrage: number
  longs: number
  shorts: number
  derniersLlm: Signal[]
}

defineProps<{
  open: boolean
  stats: SmcStats | ComputedRef<SmcStats>
}>()

defineEmits<{ close: [] }>()
</script>
