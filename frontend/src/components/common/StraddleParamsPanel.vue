<template>
  <div class="glass-card p-5 flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-xs uppercase font-bold text-white flex items-center gap-2">
        🤖 Paramètres Straddle
        <span class="text-white font-normal text-xs normal-case">(éditables — relancer avec ↺)</span>
      </h2>
      <div class="flex gap-2">
        <button
          :disabled="!hasResultats || chargementLlm"
          class="btn-primary text-xs flex items-center gap-1"
          @click="$emit('optimiser')"
        >
          <span v-if="chargementLlm">⏳</span>
          <span v-else>✨</span>
          Optimiser avec l'IA
        </button>
        <button class="btn-primary text-xs" @click="$emit('relancer')">↺ Relancer</button>
        <button
          class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-xs font-medium transition-colors"
          @click="showParams = true"
        >⚙️ Sauvegarde des paramètres</button>
      </div>
    </div>

    <div v-if="suggestion" class="mb-4 p-3 rounded-lg border border-blue-500/20 bg-blue-900/10 text-sm text-blue-200">
      💡 {{ suggestion }}
    </div>

    <div class="grid grid-cols-8 gap-3 flex-1 content-end">
      <div v-for="p in config" :key="p.key" class="flex flex-col gap-1">
        <label class="text-xs text-white">{{ p.label }}</label>
        <input
          type="number"
          :min="p.min"
          :max="p.max"
          :step="p.step"
          :value="modelValue[p.key]"
          class="bg-[#0a0e27] border border-white/20 text-white text-sm rounded px-2 py-1 w-full"
          @input="onInput(p.key, ($event.target as HTMLInputElement).valueAsNumber)"
        />
        <span class="text-xs text-white">{{ p.min }}–{{ p.max }}</span>
      </div>
      <!-- Toggle vente partielle -->
      <div class="flex flex-col gap-1 justify-center">
        <label class="text-xs text-white">Vente partielle</label>
        <button
          :class="modelValue.vente_partielle
            ? 'bg-green-600 hover:bg-green-700'
            : 'bg-gray-600 hover:bg-gray-500'"
          class="rounded px-2 py-1 text-xs font-medium text-white transition-colors"
          @click="onInput('vente_partielle', modelValue.vente_partielle ? 0 : 1)"
        >
          {{ modelValue.vente_partielle ? 'Activée' : 'Désactivée' }}
        </button>
      </div>

      <div class="flex flex-col gap-1" v-if="modelValue.vente_partielle">
        <label class="text-xs text-white">% Vente TP1</label>
        <input
          type="number"
          :min="0" :max="1" :step="0.05"
          :value="modelValue.pct_cloture_tp1 ?? 0.33"
          class="bg-[#0a0e27] border border-white/20 text-white text-sm rounded px-2 py-1 w-full"
          @input="onInput('pct_cloture_tp1', ($event.target as HTMLInputElement).valueAsNumber)"
        />
        <span class="text-xs text-white">ex: 0.33</span>
      </div>

      <div class="flex flex-col gap-1" v-if="modelValue.vente_partielle">
        <label class="text-xs text-white">% Vente TP2</label>
        <input
          type="number"
          :min="0" :max="1" :step="0.05"
          :value="modelValue.pct_cloture_tp2 ?? 0.33"
          class="bg-[#0a0e27] border border-white/20 text-white text-sm rounded px-2 py-1 w-full"
          @input="onInput('pct_cloture_tp2', ($event.target as HTMLInputElement).valueAsNumber)"
        />
        <span class="text-xs text-white">ex: 0.33</span>
      </div>
    </div>
    <StraddleParamsModal v-if="showParams" @close="showParams = false" @saved="onParamsSaved" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import StraddleParamsModal from '@/components/common/StraddleParamsModal.vue'

export interface StraddleParams {
  atr_periode: number
  seuil_atr: number
  tp_mult_1: number
  tp_mult_2: number
  tp_mult_3: number
  sl_mult: number
  trailing_atr: number
  vente_partielle: number
  pct_cloture_tp1: number
  pct_cloture_tp2: number  // 1 = true, 0 = false (number pour simplifier v-model)
}

const props = defineProps<{
  modelValue: StraddleParams
  hasResultats: boolean
  chargementLlm: boolean
  suggestion: string | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: StraddleParams): void
  (e: 'optimiser'): void
  (e: 'relancer'): void
  (e: 'params-saved'): void
}>()

const config = [
  { key: 'atr_periode' as const, label: 'Période ATR',       min: 5,   max: 50,  step: 1    },
  { key: 'seuil_atr'  as const, label: 'Seuil ATR',        min: 0.5, max: 3.0, step: 0.05 },
  { key: 'tp_mult_1'  as const, label: 'TP1 × ATR',         min: 1.0, max: 4.0, step: 0.1  },
  { key: 'tp_mult_2'  as const, label: 'TP2 × ATR',         min: 2.0, max: 6.0, step: 0.1  },
  { key: 'tp_mult_3'  as const, label: 'TP3 × ATR',         min: 3.0, max: 10.0,step: 0.25 },
  { key: 'sl_mult'    as const, label: 'SL × ATR',          min: 0.2, max: 1.5, step: 0.05 },
  { key: 'trailing_atr' as const, label: 'Trailing × ATR',  min: 0.0, max: 3.0, step: 0.1  },
]

const showParams = ref(false)

function onParamsSaved() {
  showParams.value = false
  emit('params-saved')
}

function onInput(key: keyof StraddleParams, val: number) {
  emit('update:modelValue', { ...props.modelValue, [key]: val })
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-primary { @apply bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-semibold px-4 py-2 rounded-lg transition-all; }
</style>
