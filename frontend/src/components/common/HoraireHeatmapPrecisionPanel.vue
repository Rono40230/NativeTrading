<template>
  <Transition name="slide-down">
    <div v-if="cellule" class="glass-card p-5 mt-1 space-y-4">
      <!-- En-tête -->
      <div class="flex items-center justify-between">
        <p class="text-sm font-semibold text-white">
          Précision à la minute —
          <span class="text-blue-400 font-mono">{{ jourLabel }} {{ heureParis }}h Paris</span>
        </p>
        <button class="text-gray-500 hover:text-white text-lg leading-none" @click="emit('fermer')">✕</button>
      </div>

      <!-- Chargement -->
      <div v-if="chargement" class="flex items-center gap-2 text-gray-400 text-sm">
        <span class="animate-spin">⏳</span> Analyse en cours…
      </div>

      <!-- Données insuffisantes -->
      <div v-else-if="donnees && !donnees.ok" class="text-sm text-gray-500 italic">
        Données insuffisantes pour ce créneau.
      </div>

      <!-- Résultats -->
      <div v-else-if="donnees?.ok" class="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <!-- Timing optimal -->
        <div class="flex flex-col gap-1">
          <p class="text-[10px] text-gray-400 uppercase tracking-wider">Pic de volatilité</p>
          <p class="text-2xl font-bold font-mono text-emerald-400">{{ utcToParis(donnees.timing_optimal!) }}</p>
          <p class="text-[10px] text-gray-500">Heure Paris médiane</p>
        </div>

        <!-- Fenêtre d'entrée -->
        <div class="flex flex-col gap-1">
          <p class="text-[10px] text-gray-400 uppercase tracking-wider">Fenêtre d'entrée</p>
          <p class="text-sm font-mono text-white">{{ convertirFenetre(donnees.fenetre_entree!) }}</p>
          <p class="text-[10px] text-gray-500">±5 min autour du pic</p>
        </div>

        <!-- ATR au pic -->
        <div class="flex flex-col gap-1">
          <p class="text-[10px] text-gray-400 uppercase tracking-wider">ATR au pic</p>
          <p class="text-lg font-bold font-mono text-amber-400">{{ donnees.atr_pic?.toFixed(2) }}</p>
          <p class="text-[10px] text-gray-500">Moyenne au moment du pic</p>
        </div>

        <!-- Occurrences + whipsaw -->
        <div class="flex flex-col gap-1">
          <p class="text-[10px] text-gray-400 uppercase tracking-wider">Statistiques</p>
          <p class="text-sm text-white">
            <span class="font-semibold text-blue-400">{{ donnees.nb_occurrences }}</span> occurrences
          </p>
          <p v-if="donnees.whipsaw_minutes" class="text-[10px] text-orange-400">
            ⚠ Éviter {{ donnees.whipsaw_minutes }} min avant le pic (whipsaw)
          </p>
        </div>
      </div>

      <!-- Session + raison -->
      <div class="flex flex-wrap items-start gap-3 pt-1">
        <span v-if="donnees?.session" class="px-2 py-0.5 rounded-full bg-blue-500/20 text-blue-300 text-xs font-medium">{{ donnees?.session }}</span>
        <span v-if="donnees?.raison" class="text-xs text-gray-400 italic">{{ donnees?.raison }}</span>
      </div>

      <!-- Légende -->
      <p class="text-[10px] text-gray-600">
        Basé sur l'historique M1 de l'asset sélectionné. Pas une garantie de performance future.
      </p>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import type { PrecisionHoraire } from '@/services/api.types'

const props = defineProps<{
  asset: string
  cellule: { heure: number; jour: number } | null
  jourLabel: string
  heureParis: number | null
}>()

const emit = defineEmits<{ fermer: [] }>()

const alerteStore = useAlerteStore()
const chargement = ref(false)
const donnees = ref<PrecisionHoraire | null>(null)

/** Décalage UTC → Paris depuis les props (heureParis - heure UTC de la cellule). */
const decalage = computed(() => {
  if (props.heureParis === null || !props.cellule) return 1
  return ((props.heureParis! - props.cellule.heure) + 24) % 24
})

function utcToParis(hhmm: string): string {
  const [h, m] = hhmm.split(':').map(Number)
  const hP = (h + decalage.value) % 24
  return `${String(hP).padStart(2, '0')}:${String(m).padStart(2, '0')}`
}

function convertirFenetre(s: string): string {
  return s.split('–').map(utcToParis).join('–')
}

watch(() => props.cellule, async (c) => {
  if (!c) { donnees.value = null; return }
  chargement.value = true
  donnees.value = null
  try {
    donnees.value = await apiService.analyserPrecisionHoraire(props.asset, c.heure, c.jour)
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Précision horaire: ${(e as Error).message}`)
    emit('fermer')
  } finally {
    chargement.value = false
  }
}, { immediate: false })
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.25s ease;
}
.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
