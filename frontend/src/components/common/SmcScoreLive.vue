<template>
  <div class="glass-bar flex flex-col gap-2 h-full">
    <div class="flex items-center justify-between">
      <span class="text-xs uppercase font-bold text-white">△ Score SMC</span>
      <span v-if="monitoring?.derive_detectee"
        class="text-[10px] font-semibold text-orange-400 bg-orange-900/30 border border-orange-500/30 rounded px-1.5 py-0.5">
        ⚠️ Dérive
      </span>
    </div>

    <div v-if="chargement" class="flex-1 flex items-center justify-center">
      <span class="text-gray-500 text-xs animate-pulse">Chargement…</span>
    </div>
    <div v-else-if="!monitoring" class="flex-1 flex items-center justify-center">
      <span class="text-gray-600 text-xs italic">En attente de trades</span>
    </div>
    <template v-else>
      <!-- Win rate -->
      <div class="flex items-end gap-1">
        <span class="text-2xl font-bold leading-none" :class="wrCls">{{ pct(monitoring.win_rate_global) }}</span>
        <span class="text-[10px] text-gray-500 mb-0.5">win rate</span>
      </div>
      <!-- Barre win rate -->
      <div class="h-1.5 rounded-full bg-white/10 overflow-hidden">
        <div class="h-full rounded-full transition-all duration-500" :class="wrCls"
          :style="{ width: `${Math.min(monitoring.win_rate_global * 100, 100)}%`, backgroundColor: 'currentColor' }" />
      </div>
      <!-- Gagnants / Perdants / PnL -->
      <div class="flex gap-1.5 mt-auto">
        <div class="flex-1 rounded-md bg-emerald-900/20 border border-emerald-500/20 px-2 py-1 text-center">
          <div class="text-[10px] text-emerald-600">✅ Gagnants</div>
          <div class="text-sm font-bold text-emerald-400">{{ monitoring.nb_gagnants }}</div>
        </div>
        <div class="flex-1 rounded-md bg-red-900/20 border border-red-500/20 px-2 py-1 text-center">
          <div class="text-[10px] text-red-600">❌ Perdants</div>
          <div class="text-sm font-bold text-red-400">{{ monitoring.nb_perdants }}</div>
        </div>
        <div class="flex-1 rounded-md bg-white/5 border border-white/10 px-2 py-1 text-center">
          <div class="text-[10px] text-gray-500">P&L moy.</div>
          <div class="text-sm font-bold"
            :class="(monitoring.pnl_moyen_r ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ monitoring.pnl_moyen_r != null ? monitoring.pnl_moyen_r.toFixed(2) + 'R' : '—' }}
          </div>
        </div>
      </div>
      <!-- Total signaux -->
      <div class="text-[10px] text-gray-600 text-right">
        {{ monitoring.nb_feedbacks_clotures }} / {{ monitoring.nb_signals_total }} clôturés
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { SmcMonitoringData } from '@/services/api.types'

const monitoring = ref<SmcMonitoringData | null>(null)
const chargement = ref(true)

async function charger() {
  try {
    monitoring.value = await apiService.getSmcMonitoringML()
  } catch {
    // silence — données optionnelles
  } finally {
    chargement.value = false
  }
}

const wrCls = computed(() => {
  const wr = monitoring.value?.win_rate_global ?? 0
  return wr >= 0.55 ? 'text-emerald-400' : wr >= 0.45 ? 'text-yellow-400' : 'text-red-400'
})

function pct(v: number): string {
  return (v * 100).toFixed(1) + '%'
}

let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => { charger(); timer = setInterval(charger, 60_000) })
onUnmounted(() => { if (timer) clearInterval(timer) })
</script>

<style scoped>
.glass-bar {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 0.75rem;
  padding: 0.75rem;
}
</style>
