<template>
  <div class="glass-card p-5">
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider flex items-center gap-2">
        🤖 Paramètres Straddle
        <span class="text-gray-600 font-normal text-xs normal-case">(éditables — relancer avec ↺)</span>
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
      </div>
    </div>

    <div v-if="suggestion" class="mb-4 p-3 rounded-lg border border-blue-500/20 bg-blue-900/10 text-sm text-blue-200">
      💡 {{ suggestion }}
    </div>

    <div class="grid grid-cols-5 gap-4">
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
</template>

<script setup lang="ts">
export interface StraddleParams {
  tp_mult_1: number
  tp_mult_2: number
  tp_mult_3: number
  sl_mult: number
  seuil_atr: number
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
}>()

const config = [
  { key: 'seuil_atr' as const, label: 'Seuil ATR',  min: 1.2,  max: 3.0,  step: 0.05 },
  { key: 'tp_mult_1' as const, label: 'TP1 × ATR',  min: 1.0,  max: 4.0,  step: 0.1 },
  { key: 'tp_mult_2' as const, label: 'TP2 × ATR',  min: 2.0,  max: 6.0,  step: 0.1 },
  { key: 'tp_mult_3' as const, label: 'TP3 × ATR',  min: 3.0,  max: 10.0, step: 0.25 },
  { key: 'sl_mult'   as const, label: 'SL × ATR',   min: 0.2,  max: 1.5,  step: 0.05 },
]

function onInput(key: keyof StraddleParams, val: number) {
  emit('update:modelValue', { ...props.modelValue, [key]: val })
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-primary { @apply bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-sm font-semibold px-4 py-2 rounded-lg transition-all; }
</style>
