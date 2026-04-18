<template>
  <!-- Bouton toggle (toujours visible) -->
  <button
    class="absolute top-1/2 -translate-y-1/2 w-7 h-16 z-40
           rounded-l-lg bg-[#0a0e27]/95 border border-r-0 border-white/10
           flex items-center justify-center text-gray-400 hover:text-white
           transition-all duration-200"
      :style="{ right: open ? '1120px' : '0px' }"
    :title="open ? 'Masquer la sidebar IA' : 'Afficher la sidebar IA'"
    @click="emit('toggle')"
  >
    <span class="text-sm select-none">{{ open ? '›' : '‹' }}</span>
  </button>

  <!-- Drawer overlay -->
  <div
    class="absolute right-0 top-0 bottom-0 z-30 w-[1120px] flex flex-col gap-4
           overflow-y-auto overflow-x-hidden
           bg-[#0a0e27]/95 backdrop-blur-md border-l border-white/10 p-4
           transition-transform duration-200"
    :class="open ? 'translate-x-0' : 'translate-x-full'"
  >
    <!-- Prédiction IA + Score SMC empilés -->
    <div class="flex flex-col gap-4">

      <!-- Prédiction IA -->
      <div class="glass-card p-5">
        <h2 class="text-xs uppercase font-bold text-white mb-4">
          Prédiction IA — {{ asset }} {{ timeframe }}
        </h2>
        <div v-if="signalStore.prediction" class="space-y-3">
          <div class="flex items-center gap-3">
            <span class="text-2xl font-bold" :class="directionColor(signalStore.prediction.direction)">
              {{ signalStore.prediction.direction.toUpperCase() }}
            </span>
            <span
              class="px-2 py-1 rounded text-xs font-medium"
              :class="signalStore.prediction.est_confiant ? 'bg-emerald-500/20 text-emerald-300' : 'bg-yellow-500/20 text-yellow-300'"
            >
              {{ signalStore.prediction.est_confiant ? '✓ Confiant' : '⚠ Indécis' }}
            </span>
          </div>
          <div class="w-full bg-gray-700 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="signalStore.prediction.est_confiant ? 'bg-emerald-500' : 'bg-yellow-500'"
              :style="{ width: `${(signalStore.prediction.confiance * 100).toFixed(0)}%` }"
            />
          </div>
          <p class="text-xs text-gray-400">
            Confiance: {{ (signalStore.prediction.confiance * 100).toFixed(1) }}%
            — Modèle: {{ signalStore.prediction.modele_pret ? '✓ Entraîné' : '⏳ Non entraîné' }}
          </p>
        </div>
        <div v-else class="space-y-2">
          <p class="text-yellow-400/80 text-sm">⚠ Modèle non disponible</p>
          <p class="text-xs text-gray-500">Entraînement automatique quotidien à 00h00 UTC.</p>
        </div>
      </div>

      <!-- Score SMC -->
      <div>
        <SmcScoreCard
          :score-smc="signalStore.scoreSmc"
          :asset="asset"
          :timeframe="timeframe"
        />
      </div>

    </div><!-- fin blocs empilés -->

    <!-- Monitoring ML pleine largeur -->
    <MonitoringML />
  </div>
</template>

<script setup lang="ts">
import { watch, onMounted, onUnmounted } from 'vue'
import { useSignalStore } from '@/stores/signal.store'
import SmcScoreCard from '@/components/common/SmcScoreCard.vue'
import MonitoringML from '@/components/common/MonitoringML.vue'

const props = defineProps<{
  asset: string
  timeframe: string
  open: boolean
}>()

const emit = defineEmits<{ toggle: [] }>()

const signalStore = useSignalStore()

function directionColor(dir: string): string {
  if (dir.toLowerCase().includes('long')) return 'text-emerald-400'
  if (dir.toLowerCase().includes('short')) return 'text-red-400'
  return 'text-yellow-400'
}

function chargerSidebar() {
  signalStore.chargerPrediction(props.asset, props.timeframe)
  signalStore.chargerScoreSmc(props.asset, props.timeframe)
}

watch(() => `${props.asset}${props.timeframe}`, chargerSidebar)
onMounted(chargerSidebar)

const intervalSidebar = setInterval(chargerSidebar, 30_000)
onUnmounted(() => clearInterval(intervalSidebar))
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
