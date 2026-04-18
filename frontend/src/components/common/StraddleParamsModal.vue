<template>
  <Teleport to="body">
  <!-- Overlay -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div class="glass-card p-5 w-80">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-xs uppercase font-bold text-white">⚡ Straddle — Paramètres</h2>
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
          type="button"
          class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors disabled:opacity-50"
          :disabled="saving"
          @click.stop="sauvegarder"
        >{{ saving ? '…' : 'Enregistrer' }}</button>
        <span v-if="msg" :class="msg.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">{{ msg.text }}</span>
      </div>
    </div>
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAlerteStore } from '@/stores/alerte.store'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const alerteStore = useAlerteStore()
const strategyStore = useStrategyParamsStore()
const emit = defineEmits(['close', 'saved'])

const fields = [
  { key: 'atr_periode',  label: 'Période ATR',         step: 1,   min: 5   },
  { key: 'atr_seuil',    label: 'Seuil ATR (×moy)',    step: 0.1, min: 0.5 },
  { key: 'tp_mult_1',    label: 'TP1 × ATR',           step: 0.1, min: 0.5 },
  { key: 'tp_mult_2',    label: 'TP2 × ATR',           step: 0.1, min: 0.5 },
  { key: 'tp_mult_3',    label: 'TP3 × ATR',           step: 0.1, min: 0.5 },
  { key: 'sl_mult',      label: 'SL × ATR',            step: 0.1, min: 0.1 },
  { key: 'trailing_atr', label: 'Trailing Stop × ATR', step: 0.1, min: 0.0 },
]

const params = ref<Record<string, number>>({})
const loading = ref(true)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

async function sauvegarder() {
  msg.value = { ok: true, text: 'Envoi en cours…' }
  saving.value = true
  try {
    await strategyStore.saveStraddle(params.value)
    msg.value = { ok: true, text: 'Sauvegardé ✓' }
    alerteStore.afficherSucces('Paramètres Straddle sauvegardés')
    setTimeout(() => emit('saved'), 800)
  } catch (err: any) {
    msg.value = { ok: false, text: `Erreur: ${err.message}` }
    alerteStore.afficherErreur(`Erreur sauvegarde Straddle: ${err.message}`)
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  try {
    await strategyStore.charger()
    params.value = { ...strategyStore.straddleRaw }
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
