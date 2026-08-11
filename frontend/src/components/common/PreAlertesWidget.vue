<template>
  <div class="glass-card p-3 flex flex-col gap-2">
    <div class="flex items-center justify-between">
      <h3 class="text-[10px] font-bold uppercase tracking-widest text-yellow-400">
        ⚠️ Setups en formation
      </h3>
      <span v-if="preAlertes.length" class="text-[9px] text-gray-500">
        {{ preAlertes.length }} alerte{{ preAlertes.length > 1 ? 's' : '' }}
      </span>
    </div>

    <div v-if="chargement" class="text-gray-600 text-xs text-center py-2">…</div>

    <div v-else-if="preAlertes.length === 0" class="text-gray-600 text-[10px] text-center py-2">
      Aucun setup détecté
    </div>

    <div v-else class="flex flex-col gap-1.5 overflow-y-auto max-h-[180px]">
      <div
        v-for="alerte in preAlertes"
        :key="alerte.id"
        class="rounded-lg border px-2 py-1.5"
        :class="badgeClasse(alerte.strategie)"
      >
        <div class="flex items-center justify-between gap-1">
          <span class="font-bold text-[11px]">{{ alerte.asset }}</span>
          <span class="text-[9px] uppercase tracking-wide opacity-70">{{ alerte.strategie }}</span>
        </div>
        <div class="text-[10px] text-gray-300 leading-tight mt-0.5 line-clamp-2">
          {{ alerte.raison }}
        </div>
        <div v-if="alerte.score_actuel" class="text-[9px] text-blue-300 mt-0.5">
          Score {{ alerte.score_actuel.toFixed(0) }}/100
        </div>
        <div v-if="alerte.evenement" class="text-[9px] text-yellow-300 mt-0.5">
          📅 {{ alerte.evenement }} dans {{ alerte.minutes_avant }} min
        </div>
        <div class="text-[8px] text-gray-500 mt-0.5">{{ formatDate(alerte.cree_le) }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'

interface PreAlerte {
  id: string
  asset: string
  strategie: string
  raison: string
  score_actuel: number | null
  evenement: string | null
  minutes_avant: number | null
  cree_le: string
}

const preAlertes = ref<PreAlerte[]>([])
const chargement = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

async function charger() {
  chargement.value = true
  try {
    preAlertes.value = (await apiService.getPreAlertes(10)) as PreAlerte[]
  } catch {
    // silencieux — widget non critique
  } finally {
    chargement.value = false
  }
}

function badgeClasse(strategie: string): string {
  if (strategie === 'straddle') return 'border-yellow-500/30 bg-yellow-500/10'
  if (strategie === 'smc') return 'border-blue-500/30 bg-blue-500/10'
  return 'border-white/10 bg-white/5'
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return iso
  }
}

onMounted(() => {
  charger()
  timer = setInterval(charger, 60_000) // refresh toutes les 60s
})
onUnmounted(() => { if (timer) clearInterval(timer) })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
