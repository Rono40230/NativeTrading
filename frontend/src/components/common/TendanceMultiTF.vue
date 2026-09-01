<template>
  <div class="absolute top-3 left-3 z-10 select-none" style="min-width: 132px;">
    <div class="bg-[#0a0e27]/90 border border-white/10 rounded-xl overflow-hidden shadow-2xl backdrop-blur-sm">
      <!-- Entête -->
      <div class="flex items-center justify-between px-3 py-1.5 border-b border-white/10">
        <span class="text-[10px] font-semibold text-white uppercase tracking-wide">Periode</span>
        <div class="flex items-center gap-1.5">
          <span class="text-[9px] text-white">EMA({{ props.periodeRapide }}/{{ props.periodeLente }})</span>
        </div>
      </div>
      <!-- Chargement -->
      <div v-if="chargement" class="px-3 py-3 flex items-center gap-2">
        <span class="w-3 h-3 rounded-full border-2 border-blue-400 border-t-transparent animate-spin" />
        <span class="text-[10px] text-white">Calcul…</span>
      </div>
      <div v-else-if="erreur" class="px-3 py-2 text-[10px] text-red-400">
        ⚠ {{ erreur }}
      </div>
      <div v-else>
        <div
          v-for="ligne in lignes"
          :key="ligne.tf"
          class="flex items-center justify-between px-3 py-1 border-b border-white/5 last:border-0 transition-colors"
          :class="ligne.tf === tfActifTable ? 'bg-blue-500/15 border-l-2 border-l-blue-400' : 'hover:bg-white/5'"
        >
          <!-- Colonne TF -->
          <span
            class="text-[11px] font-mono w-8 shrink-0"
            :class="ligne.tf === tfActifTable ? 'text-blue-300 font-bold' : 'text-white'"
          >{{ ligne.tf }}</span>
          <!-- Colonne direction -->
          <div v-if="ligne.tendance === 'haussier'" class="flex items-center gap-1">
            <span class="text-[10px] font-semibold text-emerald-400">Haussier</span>
            <span class="text-emerald-400 text-[11px]">▲</span>
          </div>
          <div v-else-if="ligne.tendance === 'baissier'" class="flex items-center gap-1">
            <span class="text-[10px] font-semibold text-red-400">Baissier</span>
            <span class="text-red-400 text-[11px]">▼</span>
          </div>
          <span v-else class="text-[10px] text-white">—</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { LigneTendanceKasper } from '@/services/api.types'

const props = defineProps<{
  asset: string
  timeframe: string
  periodeRapide: number
  periodeLente: number
  modeCalcul: 'bougie_cloturee' | 'bougie_en_cours'
}>()

const lignes = ref<LigneTendanceKasper[]>([])
const chargement = ref(false)
const erreur = ref<string | null>(null)

// Format chart (M1, H1…) → format API (1m, 1H…)
const tfActifTable = computed(() => {
  const map: Record<string, string> = {
    M1: '1m', M5: '5m', M15: '15m', M30: '30m',
    H1: '1H', H4: '4H', D1: '1D',
  }
  return map[props.timeframe] ?? ''
})

async function charger() {
  chargement.value = true
  erreur.value = null
  try {
    const data = await apiService.obtenirTendanceMultiTf(
      props.asset,
      props.periodeRapide,
      props.periodeLente,
      props.modeCalcul
    )
    lignes.value = data.lignes
  } catch (err: unknown) {
    erreur.value = err instanceof Error ? err.message : 'Erreur réseau'
  } finally {
    chargement.value = false
  }
}

onMounted(charger)

watch(
  () => [props.asset, props.timeframe, props.periodeRapide, props.periodeLente, props.modeCalcul],
  charger
)
</script>
