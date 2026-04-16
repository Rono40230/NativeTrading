<template>
  <div class="glass-bar px-4 py-2.5 flex flex-wrap gap-2 content-start h-full">
    <span class="text-[11px] font-semibold uppercase tracking-widest text-white w-full shrink-0">⏳ Signaux en attente</span>
    <span v-if="enAttente.length === 0" class="text-xs text-gray-600 italic">Aucun signal</span>
    <button
      v-for="s in enAttente"
      :key="s.id"
      class="flex items-center gap-1.5 px-2.5 py-1 rounded-full border border-yellow-500/40 bg-yellow-900/20 text-yellow-300 text-xs font-semibold hover:bg-yellow-900/40 hover:border-yellow-400/60 transition-all"
      @click="ouvrirModal(s)"
    >
      <span>🚀 {{ s.ticker }}</span>
      <span class="text-yellow-500/70 font-normal">{{ s.phase }}</span>
      <span v-if="s.llm_conviction !== null" class="text-[10px] opacity-70">{{ s.llm_conviction }}%</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketSignalHistorique } from '@/services/api.types'
import { useSignalAlarmeStore } from '@/stores/signal-alarme.store'
import { rocketToSignal } from '@/composables/useRocketsHistory'

const enAttente = ref<RocketSignalHistorique[]>([])
const alarmeStore = useSignalAlarmeStore()

function ouvrirModal(s: RocketSignalHistorique) {
  alarmeStore.ajouterSignal(rocketToSignal(s))
}

async function charger() {
  try {
    const tous = await apiService.rocketsActifs()
    enAttente.value = tous.filter(r => r.statut === 'attente')
  } catch {
    enAttente.value = []
  }
}

let _poll: ReturnType<typeof setInterval> | null = null
onMounted(() => { charger(); _poll = setInterval(charger, 30_000) })
onUnmounted(() => { if (_poll !== null) { clearInterval(_poll); _poll = null } })
</script>

<style scoped>
.glass-bar { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
