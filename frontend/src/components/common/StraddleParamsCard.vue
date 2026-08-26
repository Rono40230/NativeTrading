<template>
<!-- STRADDLE (définition étape 4 — T-10 s, R×ATR, trailing ×R) -->
    <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
      <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-yellow-500/50 to-transparent"></div>

      <!-- Header -->
      <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
        <h3 class="font-bold text-base flex items-center gap-2">
          <span class="w-8 h-8 rounded-full bg-yellow-500/10 flex items-center justify-center text-yellow-400">⚡</span>
          STRATÉGIE STRADDLE
        </h3>
      </div>

      <!-- Content -->
      <div class="p-5 flex-1 space-y-4">
        <!-- Minutage -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Minutage</h4>
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">Placement des 2 jambes (secondes avant l'annonce)</span>
            <input v-model.number="store.straddleRaw['placement_sec']" type="number" :step="1" :min="1"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <div class="h-px w-full bg-white/5 my-2"></div>

        <!-- Risque -->
        <h4 class="text-xs uppercase text-gray-500 font-semibold tracking-wider">Risque (R = SL × ATR H1)</h4>
        <div class="space-y-3">
          <div class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">SL (1R) × ATR H1</span>
            <input v-model.number="store.straddleRaw['sl_mult']" type="number" :step="0.1" :min="0.1"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
          <div class="flex items-center justify-between gap-4">
            <span class="text-gray-300 text-xs">Trailing (× R, dès TP2)</span>
            <input v-model.number="store.straddleRaw['trailing_r']" type="number" :step="0.1" :min="0.1"
              class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
          </div>
        </div>

        <p class="text-[11px] text-gray-500 leading-relaxed">
          R est mesuré sur l'ATR H1 (volatilité normale de l'actif) et non sur la
          compression M1 pré-annonce — un R microscopique faisait égorger les jambes
          par le spike initial (constat Gate 3 26/08). TP1 = 1R (BE à l'entrée) et
          TP2 = 2R (BE à TP1 + trailing) sont canoniques. Time-stop 60 min.
        </p>
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

const savingStraddle = ref(false)
const msgStraddle = ref<{ ok: boolean; text: string } | null>(null)
async function sauvegarderStraddle() {
  savingStraddle.value = true; msgStraddle.value = null
  try { await store.saveStraddle(store.straddleRaw); msgStraddle.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgStraddle.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingStraddle.value = false; setTimeout(() => msgStraddle.value = null, 3000) }
}
</script>
