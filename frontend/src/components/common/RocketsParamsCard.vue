<template>
<!-- ROCKETS -->
    <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
      <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-blue-500/50 to-transparent"></div>
      
      <!-- Header -->
      <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
        <h3 class="font-bold text-base flex items-center gap-2">
          <span class="w-8 h-8 rounded-full bg-blue-500/10 flex items-center justify-center text-blue-400">🚀</span>
          STRATÉGIE ROCKETS
        </h3>
      </div>

      <!-- Content -->
      <div class="p-5 flex-1 space-y-4">
        <!-- Général -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Score & Filtres (ML)</h4>
        <div class="space-y-3">
          <div v-for="f in rocketsFields.slice(0, 5)" :key="f.key" class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-[11px] font-medium leading-tight whitespace-nowrap">{{ f.label }}</span>
            <input v-model.number="store.rocketsRaw[f.key]" type="number" :step="f.step" :min="f.min"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <div class="h-px w-full bg-white/5 my-2"></div>

        <!-- Take Profits & SL -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Gestion Risque (R)</h4>
        <div class="space-y-3">
          <div v-for="f in rocketsFields.slice(5)" :key="f.key" class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-[11px] font-medium leading-tight whitespace-nowrap">{{ f.label }}</span>
            <input v-model.number="store.rocketsRaw[f.key]" type="number" :step="f.step" :min="f.min"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <div class="h-px w-full bg-white/5 my-2"></div>

        <!-- Vente Partielle & Extras -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Vente Partielle</h4>
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">Vente partielle active</span>
            <button @click="store.rocketsRaw['vente_partielle'] = !store.rocketsRaw['vente_partielle']"
              :class="store.rocketsRaw['vente_partielle'] ? 'bg-emerald-500' : 'bg-gray-600'"
              class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none">
              <span :class="store.rocketsRaw['vente_partielle'] ? 'translate-x-5' : 'translate-x-1'"
                class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform"></span>
            </button>
          </div>

          <div v-if="store.rocketsRaw['vente_partielle']" class="space-y-3 animate-fade-in pl-2 border-l border-white/10">
            <div class="flex items-center justify-between gap-4">
              <label class="text-[11px] text-gray-400">→ % Vente TP1</label>
              <div class="w-20 relative">
                <input v-model.number="store.rocketsRaw['pct_cloture_tp1']" type="number" step="0.05" min="0" max="1"
                  class="w-full bg-black/20 border border-white/10 rounded-md py-1.5 pl-3 pr-6 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
                <span class="absolute right-2 top-1.5 text-gray-500 text-xs px-0.5">%</span>
              </div>
            </div>
            <div class="flex items-center justify-between gap-4">
              <label class="text-[11px] text-gray-400">→ % Vente TP2</label>
              <div class="w-20 relative">
                <input v-model.number="store.rocketsRaw['pct_cloture_tp2']" type="number" step="0.05" min="0" max="1"
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
          <span v-if="msgRockets" class="text-xs mr-2 transition-opacity" :class="msgRockets.ok ? 'text-emerald-400' : 'text-red-400'">
            {{ msgRockets.text }}
          </span>
          <span v-else class="text-xs mr-2 text-transparent">Sp</span>
          <button @click="sauvegarderRockets" :disabled="savingRockets"
            class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
            {{ savingRockets ? '...' : 'Enregistrer' }}
          </button>
        </div>
      </div>
    </div>

  
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const store = useStrategyParamsStore()

const rocketsFields = [
  { key: 'score_min',            label: 'Score mini.',          step: 1,     min: 20    },
  { key: 'rsi_max',              label: 'RSI max',              step: 1,     min: 50    },
  { key: 'rsi_min',              label: 'RSI min',              step: 1,     min: 0     },
  { key: 'ratio_volume_min',     label: 'Vol ratio min',        step: 0.1,   min: 1     },
  { key: 'vol_marche_min',       label: 'Vol. marché',          step: 10000, min: 10000 },
  { key: 'sl_mult',              label: 'SL × ATR',             step: 0.1,   min: 0.1   },
  { key: 'trailing_coeff_min',   label: 'Trailing min',         step: 0.1,   min: 0.5   },
  { key: 'trailing_coeff_max',   label: 'Trailing max',         step: 0.1,   min: 1.0   },
  { key: 'seuil_score_faible',   label: 'Seuil faible',         step: 1,     min: 30    },
  { key: 'seuil_score_fort',     label: 'Seuil fort',           step: 1,     min: 50    },
]

const savingRockets = ref(false)
const msgRockets = ref<{ ok: boolean; text: string } | null>(null)
async function sauvegarderRockets() {
  savingRockets.value = true; msgRockets.value = null
  try { await store.saveRockets(store.rocketsRaw); msgRockets.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgRockets.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingRockets.value = false; setTimeout(() => msgRockets.value = null, 3000) }
}
</script>
