<template>
  <!-- 2 colonnes côte à côte : Straddle | SMC -->
  <div class="grid grid-cols-2 gap-4">

    <!-- Straddle -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">⚡ Straddle</h2>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            :disabled="savingStraddle"
            @click="sauvegarderStraddle"
          >{{ savingStraddle ? '…' : 'Enregistrer' }}</button>
          <span v-if="msgStraddle" :class="msgStraddle.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">
            {{ msgStraddle.text }}
          </span>
        </div>
      </div>

      <div v-if="loadingStraddle" class="text-gray-400 text-xs">Chargement…</div>

      <div v-else class="space-y-1">
        <div v-for="field in straddleFields" :key="field.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ field.label }}</label>
          <input
            v-model.number="straddleParams[field.key]"
            type="number"
            :step="field.step"
            :min="field.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
      </div>
    </div>

    <!-- SMC Directionnel -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">🎯 SMC Directionnel</h2>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            :disabled="savingSmc"
            @click="sauvegarderSmc"
          >{{ savingSmc ? '…' : 'Enregistrer' }}</button>
          <span v-if="msgSmc" :class="msgSmc.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">
            {{ msgSmc.text }}
          </span>
        </div>
      </div>

      <div v-if="loadingSmc" class="text-gray-400 text-xs">Chargement…</div>

      <div v-else class="space-y-1">
        <div v-for="field in smcFields" :key="field.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ field.label }}</label>
          <input
            v-model.number="smcParams[field.key]"
            type="number"
            :step="field.step"
            :min="field.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '../services/api.service'

// ── Straddle ──────────────────────────────────────────────────────────────────

const straddleFields = [
  { key: 'atr_periode',     label: 'Période ATR',          step: 1,   min: 5,   hint: 'Bougies pour calcul ATR (défaut 14)' },
  { key: 'atr_seuil',       label: 'Seuil ATR (×moyenne)', step: 0.1, min: 0.5, hint: 'ATR > seuil×moyenne pour déclencher (défaut 1.5)' },
  { key: 'tp_mult_1',       label: 'TP1 × ATR',            step: 0.1, min: 0.5, hint: 'Premier TP (défaut 2.0)' },
  { key: 'tp_mult_2',       label: 'TP2 × ATR',            step: 0.1, min: 0.5, hint: 'Deuxième TP (défaut 3.5)' },
  { key: 'tp_mult_3',       label: 'TP3 × ATR',            step: 0.1, min: 0.5, hint: 'Troisième TP (défaut 5.0)' },
  { key: 'sl_mult',         label: 'SL × ATR',             step: 0.1, min: 0.1, hint: 'Stop Loss (défaut 0.5)' },
  { key: 'horizon_bougies', label: 'Horizon (bougies)',    step: 1,   min: 2,   hint: 'Expiration live en bougies (défaut 48)' },
  { key: 'trailing_atr',    label: 'Trailing Stop × ATR',  step: 0.1, min: 0.0, hint: '0 = désactivé | sinon SL remonte au peak−ATR×n' },
]

const straddleParams = ref<Record<string, number>>({})
const loadingStraddle = ref(true)
const savingStraddle = ref(false)
const msgStraddle = ref<{ ok: boolean; text: string } | null>(null)

async function chargerStraddle() {
  try {
    straddleParams.value = await apiService.getStraddleParams()
  } catch (err: any) {
    msgStraddle.value = { ok: false, text: `Erreur chargement: ${err.message}` }
  } finally {
    loadingStraddle.value = false
  }
}

async function sauvegarderStraddle() {
  savingStraddle.value = true
  msgStraddle.value = null
  try {
    await apiService.putStraddleParams(straddleParams.value)
    msgStraddle.value = { ok: true, text: 'Sauvegardé ✓' }
  } catch (err: any) {
    msgStraddle.value = { ok: false, text: `Erreur: ${err.message}` }
  } finally {
    savingStraddle.value = false
  }
}

// ── SMC ──────────────────────────────────────────────────────────────────────

const smcFields = [
  { key: 'atr_periode',     label: 'Période ATR',       step: 1,   min: 5,  hint: 'Bougies pour calcul ATR (défaut 14)' },
  { key: 'score_min',       label: 'Score minimum',     step: 1,   min: 40, hint: 'Score SMC requis sur 100 (défaut 70)' },
  { key: 'atr_tp1',         label: 'TP1 × ATR',         step: 0.1, min: 0.5, hint: 'Premier TP pyramidal (défaut 1.5)' },
  { key: 'atr_tp2',         label: 'TP2 × ATR',         step: 0.1, min: 0.5, hint: 'Deuxième TP (défaut 3.0)' },
  { key: 'atr_tp3',         label: 'TP3 × ATR',         step: 0.1, min: 0.5, hint: 'Troisième TP (défaut 5.0)' },
  { key: 'atr_sl',          label: 'SL × ATR',          step: 0.1, min: 0.1, hint: 'Stop Loss (défaut 1.0)' },
  { key: 'horizon_bougies', label: 'Horizon (bougies)', step: 1,   min: 2,  hint: 'Expiration live en bougies (défaut 24)' },
]

const smcParams = ref<Record<string, number>>({})
const loadingSmc = ref(true)
const savingSmc = ref(false)
const msgSmc = ref<{ ok: boolean; text: string } | null>(null)

async function chargerSmc() {
  try {
    smcParams.value = await apiService.getSmcParams()
  } catch (err: any) {
    msgSmc.value = { ok: false, text: `Erreur chargement: ${err.message}` }
  } finally {
    loadingSmc.value = false
  }
}

async function sauvegarderSmc() {
  savingSmc.value = true
  msgSmc.value = null
  try {
    await apiService.putSmcParams(smcParams.value)
    msgSmc.value = { ok: true, text: 'Sauvegardé ✓' }
  } catch (err: any) {
    msgSmc.value = { ok: false, text: `Erreur: ${err.message}` }
  } finally {
    savingSmc.value = false
  }
}

onMounted(() => {
  chargerStraddle()
  chargerSmc()
})
</script>
