<template>
  <div class="flex gap-4 items-stretch h-56">
    <!-- Prédiction IA — 1/5 -->
    <div class="glass-card p-5 flex-[1]">
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
        <button
          class="mt-3 w-full py-2 px-3 rounded-lg text-xs font-semibold transition-all"
          :class="entraineEnCours ? 'bg-gray-600 text-gray-400 cursor-not-allowed' : 'bg-blue-600 hover:bg-blue-500 text-white'"
          :disabled="entraineEnCours"
          @click="lancerEntrainement"
        >
          {{ entraineEnCours ? '⏳ Entraînement...' : '🧠 Entraîner RF + LSTM' }}
        </button>
      </div>
      <div v-else class="space-y-2">
        <p class="text-yellow-400/80 text-sm">⚠ Modèle non disponible</p>
        <p class="text-xs text-gray-500">Entraînez le modèle pour obtenir une prédiction sur cet actif/TF.</p>
        <button
          class="mt-1 w-full py-2 px-3 rounded-lg text-xs font-semibold bg-blue-600 hover:bg-blue-500 text-white transition-all"
          :disabled="entraineEnCours"
          @click="lancerEntrainement"
        >{{ entraineEnCours ? '⏳ Entraînement...' : '🧠 Entraîner RF + LSTM' }}</button>
      </div>
    </div>

    <!-- Score SMC — 2/5 -->
    <div class="flex-[2]">
      <SmcScoreCard
        :score-smc="signalStore.scoreSmc"
        :asset="asset"
        :timeframe="timeframe"
      />
    </div>

    <!-- Monitoring ML — 2/5 -->
    <div class="flex-[2]">
      <MonitoringML />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useSignalStore } from '@/stores/signal.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { apiService } from '@/services/api.service'
import SmcScoreCard from './SmcScoreCard.vue'
import MonitoringML from './MonitoringML.vue'

const props = defineProps<{
  asset: string
  timeframe: string
}>()

const signalStore = useSignalStore()
const alerteStore = useAlerteStore()
const entraineEnCours = ref(false)

function directionColor(dir: string): string {
  if (dir.toLowerCase().includes('long')) return 'text-emerald-400'
  if (dir.toLowerCase().includes('short')) return 'text-red-400'
  return 'text-yellow-400'
}

async function lancerEntrainement() {
  entraineEnCours.value = true
  alerteStore.afficher('Entraînement ML en cours (RF + LSTM)...', 'info')
  try {
    const res = await apiService.entrainerML(props.asset, props.timeframe, 1000)
    alerteStore.afficherSucces(`✅ ${res.message}`)
    await signalStore.chargerPrediction(props.asset, props.timeframe)
  } catch (err: unknown) {
    const axiosBody = (err as any)?.response?.data?.error
    const msg = axiosBody ?? (err instanceof Error ? err.message : 'Erreur inconnue')
    alerteStore.afficherErreur(`Entraînement échoué: ${msg}`)
  } finally {
    entraineEnCours.value = false
  }
}

function charger() {
  signalStore.chargerPrediction(props.asset, props.timeframe)
  signalStore.chargerScoreSmc(props.asset, props.timeframe)
}

watch(() => `${props.asset}${props.timeframe}`, charger)
onMounted(charger)
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm overflow-hidden;
}
</style>
