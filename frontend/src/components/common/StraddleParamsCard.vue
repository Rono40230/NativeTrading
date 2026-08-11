<template>
<!-- STRADDLE -->
    <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
      <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-yellow-500/50 to-transparent"></div>
      
      <!-- Header -->
      <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
        <h3 class="font-bold text-base flex items-center gap-2">
          <span class="w-8 h-8 rounded-full bg-yellow-500/10 flex items-center justify-center text-yellow-400">⚡</span>
          STRATÉGIE VOLATILITÉ
        </h3>
      </div>

      <!-- Content -->
      <div class="p-5 flex-1 space-y-4">
        <!-- Général -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Indicateurs & Contexte</h4>
        <div class="space-y-3">
          <div v-for="f in straddleFields.slice(0, 2)" :key="f.key" class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">{{ f.label }}</span>
            <input v-model.number="store.straddleRaw[f.key]" type="number" :step="f.step" :min="f.min"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <div class="h-px w-full bg-white/5 my-2"></div>

        <!-- Take Profits & SL -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Gestion Risque (R)</h4>
        <div class="space-y-3">
          <div v-for="f in straddleFields.slice(2)" :key="f.key" class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">{{ f.label }}</span>
            <input v-model.number="store.straddleRaw[f.key]" type="number" :step="f.step" :min="f.min"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <div class="h-px w-full bg-white/5 my-2"></div>

        <!-- Vente Partielle -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Vente Partielle</h4>
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">Vente partielle active</span>
            <button @click="store.straddleRaw['vente_partielle'] = !store.straddleRaw['vente_partielle']"
              :class="store.straddleRaw['vente_partielle'] ? 'bg-emerald-500' : 'bg-gray-600'"
              class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none">
              <span :class="store.straddleRaw['vente_partielle'] ? 'translate-x-5' : 'translate-x-1'"
                class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform"></span>
            </button>
          </div>

          <div v-if="store.straddleRaw['vente_partielle']" class="space-y-3 animate-fade-in pl-2 border-l border-white/10">
            <div class="flex items-center justify-between gap-4">
              <label class="text-[11px] text-gray-400">→ % Vente TP1</label>
              <div class="w-20 relative">
                <input v-model.number="store.straddleRaw['pct_cloture_tp1']" type="number" step="0.05" min="0" max="1"
                  class="w-full bg-black/20 border border-white/10 rounded-md py-1.5 pl-3 pr-6 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
                <span class="absolute right-2 top-1.5 text-gray-500 text-xs px-0.5">%</span>
              </div>
            </div>
            <div class="flex items-center justify-between gap-4">
              <label class="text-[11px] text-gray-400">→ % Vente TP2</label>
              <div class="w-20 relative">
                <input v-model.number="store.straddleRaw['pct_cloture_tp2']" type="number" step="0.05" min="0" max="1"
                  class="w-full bg-black/20 border border-white/10 rounded-md py-1.5 pl-3 pr-6 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
                <span class="absolute right-2 top-1.5 text-gray-500 text-xs px-0.5">%</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Action -->
      <div class="p-5 mt-auto bg-black/10 border-t border-white/5">
        <div class="flex items-center justify-between">
          <span v-if="msgStraddle" class="text-xs mr-2 transition-opacity" :class="msgStraddle.ok ? 'text-emerald-400' : 'text-red-400'">
            {{ msgStraddle.text }}
          </span>
          <span v-else class="text-xs mr-2 text-transparent">Sp</span>
          <button @click="sauvegarderStraddle" :disabled="savingStraddle"
            class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
            {{ savingStraddle ? '...' : 'Enregistrer' }}
          </button>
        </div>
      </div>
    </div>


    
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const store = useStrategyParamsStore()

const straddleFields = [
  { key: 'atr_periode' as const,     label: 'Période ATR',         step: 1,     min: 5    },
  { key: 'atr_seuil' as const,       label: 'Seuil ATR (×moy)',    step: 0.1,   min: 0.5  },
  { key: 'tp_mult_1' as const,       label: 'TP1 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'tp_mult_2' as const,       label: 'TP2 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'tp_mult_3' as const,       label: 'TP3 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'sl_mult' as const,         label: 'SL × ATR',            step: 0.1,   min: 0.1  },
  { key: 'trailing_atr' as const,    label: 'Trailing Stop × ATR', step: 0.1,   min: 0.0  },
]

const savingStraddle = ref(false)
const msgStraddle = ref<{ ok: boolean; text: string } | null>(null)
async function sauvegarderStraddle() {
  savingStraddle.value = true; msgStraddle.value = null
  try { await store.saveStraddle(store.straddleRaw); msgStraddle.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgStraddle.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingStraddle.value = false; setTimeout(() => msgStraddle.value = null, 3000) }
}
</script>
