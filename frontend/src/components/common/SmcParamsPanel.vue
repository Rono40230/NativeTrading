<template>
  <div class="card-moteur flex flex-col">
    <div class="flex items-center justify-between px-5 py-3">
      <div class="flex items-center gap-2">
        <h2 class="text-xs uppercase font-bold text-white">⚙️ Paramètres Moteur SMC</h2>
        <span class="text-[10px] font-bold px-2 py-0.5 rounded-full bg-blue-600/30 text-blue-300 border border-blue-500/40 uppercase tracking-wider">Moteur Auto</span>
      </div>
      <button
        class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-xs font-medium transition-colors"
        @click="showParams = true"
      >💾 Sauvegarde</button>
    </div>

    <div class="px-5 pb-4">
      <div v-if="loading" class="text-xs text-gray-500 py-2">Chargement…</div>
      <div v-else class="grid grid-cols-6 gap-3">
      <div v-for="p in config" :key="p.key" class="flex flex-col gap-1">
        <label class="text-xs text-gray-400">{{ p.label }}</label>
        <input
          type="number"
          :min="p.min"
          :max="p.max"
          :step="p.step"
          :value="modelValue[p.key]"
          class="bg-[#0a0e27] border border-white/20 text-white text-sm rounded px-2 py-1 w-full"
          @input="onInput(p.key, ($event.target as HTMLInputElement).valueAsNumber)"
        />
        <span class="text-xs text-gray-600">{{ p.min }}–{{ p.max }}</span>
      </div>
    </div>
    </div>

    <SmcParamsModal v-if="showParams" @close="showParams = false" @saved="onParamsSaved" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import SmcParamsModal from '@/components/common/SmcParamsModal.vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

export interface SmcParams {
  atr_periode: number
  score_min: number
  atr_tp1: number
  atr_tp2: number
  atr_tp3: number
  atr_sl: number
}

const props = defineProps<{
  modelValue: SmcParams
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: SmcParams): void
  (e: 'params-saved'): void
}>()

const config = [
  { key: 'atr_periode' as const, label: 'Période ATR',       min: 5,   max: 50,  step: 1    },
  { key: 'score_min'   as const, label: 'Score min (/ 100)', min: 40,  max: 100, step: 1    },
  { key: 'atr_tp1'     as const, label: 'TP1 × ATR',         min: 0.5, max: 4.0, step: 0.1  },
  { key: 'atr_tp2'     as const, label: 'TP2 × ATR',         min: 1.0, max: 6.0, step: 0.1  },
  { key: 'atr_tp3'     as const, label: 'TP3 × ATR',         min: 2.0, max: 10.0, step: 0.25 },
  { key: 'atr_sl'      as const, label: 'SL × ATR',          min: 0.2, max: 2.0, step: 0.05 },
]

const showParams = ref(false)
const loading = ref(true)
const strategyStore = useStrategyParamsStore()

async function chargerParams() {
  await strategyStore.charger()
  emit('update:modelValue', strategyStore.smcParams)
  loading.value = false
}

async function onParamsSaved() {
  showParams.value = false
  emit('update:modelValue', strategyStore.smcParams)
  emit('params-saved')
}

function onInput(key: keyof SmcParams, val: number) {
  emit('update:modelValue', { ...props.modelValue, [key]: val })
}

onMounted(chargerParams)
</script>

<style scoped>
.card-moteur { @apply rounded-xl border border-blue-500/30 bg-blue-900/5 backdrop-blur-sm; }
</style>
