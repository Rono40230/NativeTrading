<template>
  <div class="glass-card p-6 space-y-6">
    <!-- Déclencheur -->
    <div class="flex flex-col md:flex-row md:items-center gap-4">
      <div class="flex-1 space-y-1">
        <p class="text-sm font-semibold text-gray-200">Réentraîner le pipeline ML maintenant</p>
        <p class="text-xs text-gray-400">Lance un entraînement walk-forward sur toutes les combinaisons asset × timeframe disponibles en base. Un rollback automatique est déclenché si l'accuracy chute de plus de 2 pts.</p>
      </div>
      <button
        class="shrink-0 px-5 py-2.5 rounded-lg font-semibold text-sm transition-colors"
        :class="store.retrainState?.en_cours
          ? 'bg-gray-700 text-gray-400 cursor-not-allowed'
          : 'bg-blue-600 hover:bg-blue-500 text-white'"
        :disabled="store.retrainState?.en_cours"
        @click="store.declencherRetrain()"
      >
        {{ store.retrainState?.en_cours ? '⏳ En cours…' : '🔁 Lancer le réentraînement' }}
      </button>
    </div>

    <!-- Barre de progression (uniquement pendant l'entraînement) -->
    <div v-if="store.retrainState?.en_cours" class="space-y-1">
      <div class="flex justify-between text-xs text-gray-500 mb-1">
        <span>
          Entraînement walk-forward…
          <template v-if="store.retrainState.nb_combinaisons_total > 0">
            — {{ store.retrainState.nb_combinaisons_done }} / {{ store.retrainState.nb_combinaisons_total }} combinaisons
          </template>
        </span>
        <span>{{ elapsed }}s écoulées</span>
      </div>
      <div class="w-full bg-gray-700 rounded-full h-2 overflow-hidden">
        <div
          class="h-2 rounded-full bg-blue-500 transition-all duration-1000"
          :style="{ width: progres + '%' }"
        />
      </div>
    </div>

    <!-- Résultats du job -->
    <div v-if="store.retrainState?.job_id && !store.retrainState?.en_cours" class="space-y-3">
      <div class="flex items-center gap-3">
        <span class="text-xs text-gray-500">Job {{ store.retrainState.job_id }}</span>
        <span
          class="text-xs font-bold px-2 py-0.5 rounded"
          :class="store.retrainState.rolled_back
            ? 'bg-yellow-800 text-yellow-200'
            : 'bg-emerald-800 text-emerald-200'"
        >
          {{ store.retrainState.rolled_back ? 'Rollback' : 'Terminé' }}
        </span>
      </div>

      <p class="text-sm text-gray-300">{{ store.retrainState.message }}</p>

      <!-- Métriques -->
      <div v-if="store.retrainState.accuracy_avant > 0" class="flex items-center gap-6 text-sm">
        <div class="text-center">
          <p class="text-xs text-gray-500 mb-1">Accuracy avant</p>
          <p class="text-lg font-bold text-gray-200">{{ (store.retrainState.accuracy_avant * 100).toFixed(1) }}%</p>
        </div>
        <span class="text-gray-600 text-xl">→</span>
        <div class="text-center">
          <p class="text-xs text-gray-500 mb-1">Accuracy après</p>
          <p
            class="text-lg font-bold"
            :class="(store.retrainState.accuracy_apres ?? 0) >= store.retrainState.accuracy_avant
              ? 'text-emerald-400' : 'text-red-400'"
          >{{ ((store.retrainState.accuracy_apres ?? 0) * 100).toFixed(1) }}%</p>
        </div>
        <div v-if="store.retrainState.gap_train_wf !== null" class="text-center border-l border-white/10 pl-6">
          <p class="text-xs text-gray-500 mb-1">Santé modèle</p>
          <p
            class="text-lg font-bold"
            :class="store.retrainState.overfitting ? 'text-red-400' : 'text-emerald-400'"
          >{{ store.retrainState.overfitting ? '⚠ Overfit' : '✓ OK' }}</p>
          <p class="text-xs text-gray-500">gap {{ (store.retrainState.gap_train_wf * 100).toFixed(1) }}%</p>
        </div>
      </div>
    </div>

    <div v-else-if="!store.retrainState?.job_id" class="text-gray-500 text-sm">
      Aucun réentraînement effectué dans cette session.
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'

const store = useMlInsightsStore()

const elapsed = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

// Progression basée sur le compteur réel renvoyé par le backend (0→100%)
// Fallback à 0% si le backend n'a pas encore communiqué le total (démarrage)
const progres = computed(() => {
  const s = store.retrainState
  if (!s?.en_cours) return 100
  const total = s.nb_combinaisons_total
  if (!total) return 0
  return Math.round((s.nb_combinaisons_done / total) * 100)
})

// Démarre/arrête le timer selon l'état du job
watch(
  () => store.retrainState?.en_cours,
  (enCours) => {
    if (enCours) {
      elapsed.value = store.retrainState?.demarre_le
        ? Math.max(0, Math.floor(Date.now() / 1000 - store.retrainState.demarre_le))
        : 0
      timer = setInterval(() => { elapsed.value++ }, 1000)
    } else {
      if (timer) { clearInterval(timer); timer = null }
    }
  },
  { immediate: true }
)

onUnmounted(() => { if (timer) clearInterval(timer) })
</script>
