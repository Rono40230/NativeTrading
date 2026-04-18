<template>
  <div class="glass-card p-4 flex flex-col gap-3 h-full">
    <div class="flex items-center justify-between">
      <span class="text-xs uppercase font-bold text-white">&#128208; Signaux SMC</span>
    </div>

    <div v-if="chargement" class="flex-1 flex items-center justify-center">
      <span class="text-gray-500 text-xs animate-pulse">Chargement…</span>
    </div>
    <div v-else-if="!categories.length" class="flex-1 flex items-center justify-center">
      <span class="text-gray-600 text-xs italic">Pas encore de données</span>
    </div>
    <template v-else>
      <div class="flex flex-col gap-1 flex-1 overflow-hidden">
        <div v-for="cat in categories" :key="cat.categorie"
          class="flex items-center gap-2 px-1.5 py-1 rounded-md bg-white/3 hover:bg-white/6 transition-colors">
          <!-- Badge catégorie -->
          <span class="text-[10px] font-semibold px-1.5 py-0.5 rounded shrink-0" :class="badgeCls(cat.categorie)">
            {{ labelCat(cat.categorie) }}
          </span>
          <!-- Barre win rate -->
          <div class="flex-1 h-1.5 rounded-full bg-white/10 overflow-hidden">
            <div class="h-full rounded-full transition-all duration-500"
              :class="cat.win_rate >= 0.55 ? 'bg-emerald-500' : cat.win_rate >= 0.45 ? 'bg-yellow-500' : 'bg-red-500'"
              :style="{ width: `${Math.min(cat.win_rate * 100, 100)}%` }"></div>
          </div>
          <!-- Win rate -->
          <span class="text-[10px] font-mono shrink-0 tabular-nums w-10 text-right"
            :class="cat.win_rate >= 0.55 ? 'text-emerald-400' : cat.win_rate >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
            {{ pct(cat.win_rate) }}
          </span>
          <!-- Nb trades -->
          <span class="text-[10px] text-gray-600 shrink-0 w-7 text-right">{{ cat.nb_trades }}</span>
        </div>
      </div>
      <!-- Résumé invalides -->
      <div v-if="nbInvalides" class="text-[10px] text-gray-600 text-right mt-auto">
        {{ nbInvalides }} signal{{ nbInvalides > 1 ? 's' : '' }} invalide{{ nbInvalides > 1 ? 's' : '' }}
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { SmcMonitoringData } from '@/services/api.types'

const data = ref<SmcMonitoringData | null>(null)
const chargement = ref(true)

const categories = computed(() =>
  (data.value?.par_categorie ?? []).filter(c => c.nb_trades > 0)
)
const nbInvalides = computed(() => data.value?.nb_invalides ?? 0)

async function charger() {
  try {
    data.value = await apiService.getSmcMonitoringML()
  } catch {
    // silence
  } finally {
    chargement.value = false
  }
}

function pct(v: number) { return (v * 100).toFixed(0) + '%' }

function labelCat(cat: string): string {
  const map: Record<string, string> = {
    breakout: 'BRK', reversal: 'REV', continuation: 'CONT', scalping: 'SCP',
  }
  return map[cat] ?? cat.slice(0, 4).toUpperCase()
}

function badgeCls(cat: string): string {
  const map: Record<string, string> = {
    breakout: 'bg-blue-900/30 text-blue-300 border border-blue-500/30',
    reversal: 'bg-purple-900/30 text-purple-300 border border-purple-500/30',
    continuation: 'bg-sky-900/30 text-sky-300 border border-sky-500/30',
    scalping: 'bg-orange-900/30 text-orange-300 border border-orange-500/30',
  }
  return map[cat] ?? 'bg-white/10 text-gray-300 border border-white/20'
}

let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => { charger(); timer = setInterval(charger, 60_000) })
onUnmounted(() => { if (timer) clearInterval(timer) })
</script>


<style scoped>
</style>
