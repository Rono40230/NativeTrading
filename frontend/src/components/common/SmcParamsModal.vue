<template>
  <Teleport to="body">
  <!-- Overlay -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div class="glass-card p-5 w-80">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-gray-300 uppercase tracking-wider">🎯 SMC — Paramètres</h2>
        <button class="text-gray-500 hover:text-white text-lg leading-none" @click="$emit('close')">✕</button>
      </div>

      <div v-if="loading" class="text-gray-400 text-xs">Chargement…</div>

      <div v-else class="space-y-1.5">
        <div v-for="field in fields" :key="field.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ field.label }}</label>
          <input
            v-model.number="params[field.key]"
            type="number"
            :step="field.step"
            :min="field.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
      </div>

      <div class="flex items-center gap-2 mt-4">
        <button
          class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
          :disabled="saving"
          @click="sauvegarder"
        >{{ saving ? '…' : 'Enregistrer' }}</button>
      </div>
    </div>
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAlerteStore } from '@/stores/alerte.store'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const emit = defineEmits(['close', 'saved'])
const alerteStore = useAlerteStore()
const strategyStore = useStrategyParamsStore()

const fields = [
  { key: 'atr_periode', label: 'Période ATR',       step: 1,   min: 5   },
  { key: 'score_min',   label: 'Score minimum',     step: 1,   min: 40  },
  { key: 'atr_tp1',     label: 'TP1 × ATR',         step: 0.1, min: 0.5 },
  { key: 'atr_tp2',     label: 'TP2 × ATR',         step: 0.1, min: 0.5 },
  { key: 'atr_tp3',     label: 'TP3 × ATR',         step: 0.1, min: 0.5 },
  { key: 'atr_sl',      label: 'SL × ATR',          step: 0.1, min: 0.1 },
]

const params = ref<Record<string, number>>({})
const loading = ref(true)
const saving = ref(false)

async function sauvegarder() {
  saving.value = true
  try {
    await strategyStore.saveSmc(params.value)
    alerteStore.afficherSucces('Paramètres SMC sauvegardés')
    setTimeout(() => emit('saved'), 800)
  } catch (err: any) {
    alerteStore.afficherErreur(`Erreur: ${err.message}`)
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  try {
    await strategyStore.charger()
    params.value = { ...strategyStore.smcRaw }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.glass-card {
  background: rgba(15, 20, 50, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0.75rem;
  backdrop-filter: blur(16px);
}
</style>
