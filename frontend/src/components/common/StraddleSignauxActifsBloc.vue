<template>
  <div class="glass-bar px-4 py-2.5 flex flex-col gap-1.5 h-full">
    <span class="text-[11px] font-semibold uppercase tracking-widest text-cyan-400 shrink-0">
      ⚡ Straddle actifs
    </span>

    <span v-if="signaux.length === 0" class="text-[10px] text-gray-500 italic">Aucun signal ouvert</span>

    <div v-for="s in signaux" :key="s.id"
      class="flex flex-col gap-0.5 rounded-lg border border-cyan-500/20 bg-cyan-900/10 px-2 py-1.5">
      <!-- Ligne 1 : asset + score + rebours -->
      <div class="flex items-center justify-between gap-1">
        <span class="text-[11px] font-bold text-white">{{ s.asset }}</span>
        <span class="text-[9px] font-semibold text-gray-400">{{ s.timeframe }}</span>
        <span class="ml-auto text-[10px] font-bold" :class="badgeRebours(s).cls">
          {{ badgeRebours(s).label }}
        </span>
      </div>
      <!-- Ligne 2 : jambes + score LLM -->
      <div class="flex items-center gap-1.5 text-[9px]">
        <span class="text-emerald-400 font-semibold">L</span>
        <span class="text-gray-500">{{ formatPrix(s.prix_entree) }}</span>
        <span class="text-gray-600">·</span>
        <span class="text-red-400 font-semibold">S</span>
        <span class="text-gray-500">{{ formatPrix(s.prix_entree) }}</span>
        <span v-if="s.llm_conviction !== null" class="ml-auto text-gray-500">
          LLM {{ s.llm_conviction }}%
        </span>
      </div>
      <!-- Ligne 3 : TP atteints si présents -->
      <div v-if="tpsAtteints(s).length" class="flex gap-1 flex-wrap">
        <span v-for="tp in tpsAtteints(s)" :key="tp"
          class="text-[9px] font-bold text-emerald-300 border border-emerald-500/30 rounded px-1">
          ✓ {{ tp }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.types'

const signaux = ref<Signal[]>([])

function badgeRebours(s: Signal): { label: string; cls: string } {
  if (!s.heure_entree) return { label: 'Entrée active', cls: 'text-emerald-400' }
  const resteSec = s.heure_entree - Math.floor(Date.now() / 1000)
  if (resteSec <= 0) return { label: 'Entrée active', cls: 'text-emerald-400 animate-pulse' }
  const min = Math.ceil(resteSec / 60)
  return { label: `⏱ dans ${min}min`, cls: 'text-yellow-400' }
}

function tpsAtteints(s: Signal): string[] {
  const long = (s.tps_long_atteints ?? []).map(t => `L-${t.toUpperCase()}`)
  const short = (s.tps_short_atteints ?? []).map(t => `S-${t.toUpperCase()}`)
  return [...long, ...short]
}

function formatPrix(p: number): string {
  return p >= 100 ? p.toFixed(2) : p.toFixed(5)
}

async function charger() {
  try {
    const tous = await apiService.getSignaux(50)
    signaux.value = tous.filter(s => s.strategie === 'straddle' && s.statut === 'Actif')
  } catch {
    signaux.value = []
  }
}

let _poll: ReturnType<typeof setInterval> | null = null
onMounted(() => { charger(); _poll = setInterval(charger, 15_000) })
onUnmounted(() => { if (_poll !== null) { clearInterval(_poll); _poll = null } })
</script>

<style scoped>
.glass-bar { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
