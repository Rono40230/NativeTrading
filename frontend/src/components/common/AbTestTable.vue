<template>
  <div v-if="stats.length" class="rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm p-5">
    <h2 class="text-xs uppercase font-bold text-white mb-4">⚖️ Comparaison A/B par stratégie</h2>
    <div class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-gray-500 text-xs border-b border-white/10">
            <th class="text-left pb-2">Stratégie</th>
            <th class="text-right pb-2">Signaux</th>
            <th class="text-right pb-2">Wins</th>
            <th class="text-right pb-2">Win Rate</th>
            <th class="text-right pb-2">Conviction IA</th>
            <th class="text-right pb-2">Score SMC</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in stats" :key="s.strategie" class="border-b border-white/5 hover:bg-white/5">
            <td class="py-2 font-medium text-white">{{ s.strategie }}</td>
            <td class="text-right text-gray-300">{{ s.nb_total }}</td>
            <td class="text-right text-emerald-400">{{ s.nb_wins }}</td>
            <td class="text-right font-semibold" :class="s.win_rate >= 55 ? 'text-emerald-400' : s.win_rate >= 45 ? 'text-yellow-400' : 'text-red-400'">
              {{ s.win_rate.toFixed(1) }}%
            </td>
            <td class="text-right text-blue-400">{{ s.conviction_moy > 0 ? s.conviction_moy.toFixed(0) : '—' }}</td>
            <td class="text-right text-purple-400">{{ s.score_moy.toFixed(1) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'

const stats = ref<{
  strategie: string; nb_total: number; nb_wins: number
  nb_pertes: number; win_rate: number; conviction_moy: number; score_moy: number
}[]>([])

onMounted(() => {
  apiService.getAbTest().then(d => { stats.value = d }).catch(() => {})
})
</script>
