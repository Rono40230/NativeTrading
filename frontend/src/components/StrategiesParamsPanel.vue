<template>
  <!-- 3 colonnes : Straddle | SMC | Rockets -->
  <div class="grid grid-cols-3 gap-4">

    <!-- ── Straddle ──────────────────────────────────────────────────────── -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">⚡ Stratégie volatilité</h2>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            :disabled="savingStraddle" @click="sauvegarderStraddle">
            {{ savingStraddle ? '…' : 'Enregistrer' }}
          </button>
          <span v-if="msgStraddle" :class="msgStraddle.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">
            {{ msgStraddle.text }}
          </span>
        </div>
      </div>
      <div v-if="store.loading || !store.isLoaded" class="text-gray-400 text-xs">Chargement…</div>
      <div v-else class="space-y-1">
        <div v-for="f in straddleFields" :key="f.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ f.label }}</label>
          <input v-model.number="store.straddleRaw[f.key]" type="number" :step="f.step" :min="f.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500" />
        </div>
        <div class="flex items-center justify-between gap-2 pt-1 border-t border-white/5">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">Sécurisation</label>
          <button :class="store.straddleRaw['vente_partielle'] ? 'bg-emerald-700 text-emerald-200' : 'bg-gray-600 text-gray-300'"
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
            @click="store.straddleRaw['vente_partielle'] = !store.straddleRaw['vente_partielle']">
            {{ store.straddleRaw['vente_partielle'] ? 'Option 1 — Partielle ⅓' : 'Option 2 — Lot entier' }}
          </button>
        </div>
      </div>
    </div>

    <!-- ── SMC Directionnel ──────────────────────────────────────────────── -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">🎯 Stratégie SMC</h2>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            :disabled="savingSmc" @click="sauvegarderSmc">
            {{ savingSmc ? '…' : 'Enregistrer' }}
          </button>
          <span v-if="msgSmc" :class="msgSmc.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">
            {{ msgSmc.text }}
          </span>
        </div>
      </div>
      <div v-if="store.loading || !store.isLoaded" class="text-gray-400 text-xs">Chargement…</div>
      <div v-else class="space-y-1">
        <div v-for="f in smcFields" :key="f.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ f.label }}</label>
          <input v-model.number="store.smcRaw[f.key]" type="number" :step="f.step" :min="f.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500" />
        </div>
        <div class="flex items-center justify-between gap-2 pt-1 border-t border-white/5">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">Sécurisation</label>
          <button :class="store.smcRaw['vente_partielle'] ? 'bg-emerald-700 text-emerald-200' : 'bg-gray-600 text-gray-300'"
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
            @click="store.smcRaw['vente_partielle'] = !store.smcRaw['vente_partielle']">
            {{ store.smcRaw['vente_partielle'] ? 'Option 1 — Partielle ⅓' : 'Option 2 — Lot entier' }}
          </button>
        </div>
        <div class="flex items-center justify-between gap-2 pt-1">
          <label class="text-[11px] text-gray-400 whitespace-nowrap" title="London 07h-10h · NY 13h30-16h30 UTC">Kill Zone ICT</label>
          <button :class="store.smcRaw['kill_zone_filtre'] ? 'bg-emerald-700 text-emerald-200' : 'bg-amber-700 text-amber-200'"
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
            @click="store.smcRaw['kill_zone_filtre'] = !store.smcRaw['kill_zone_filtre']">
            {{ store.smcRaw['kill_zone_filtre'] ? '🕐 Activée' : '⚠️ Désactivée' }}
          </button>
        </div>
      </div>
    </div>

    <!-- ── Rockets ───────────────────────────────────────────────────────── -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">🚀 Stratégie Rockets</h2>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            :disabled="savingRockets" @click="sauvegarderRockets">
            {{ savingRockets ? '…' : 'Enregistrer' }}
          </button>
          <span v-if="msgRockets" :class="msgRockets.ok ? 'text-green-400' : 'text-red-400'" class="text-xs">
            {{ msgRockets.text }}
          </span>
        </div>
      </div>
      <div v-if="store.loading || !store.isLoaded" class="text-gray-400 text-xs">Chargement…</div>
      <div v-else class="space-y-1">
        <div v-for="f in rocketsFields" :key="f.key" class="flex items-center justify-between gap-2">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">{{ f.label }}</label>
          <input v-model.number="store.rocketsRaw[f.key]" type="number" :step="f.step" :min="f.min"
            class="bg-gray-700 text-white rounded px-2 py-0.5 w-20 text-xs text-right focus:outline-none focus:ring-1 focus:ring-blue-500" />
        </div>
        <div class="flex items-center justify-between gap-2 pt-1 border-t border-white/5">
          <label class="text-[11px] text-gray-400 whitespace-nowrap">Sécurisation</label>
          <button :class="store.rocketsRaw['vente_partielle'] ? 'bg-emerald-700 text-emerald-200' : 'bg-gray-600 text-gray-300'"
            class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
            @click="store.rocketsRaw['vente_partielle'] = !store.rocketsRaw['vente_partielle']">
            {{ store.rocketsRaw['vente_partielle'] ? 'Option 1 — Partielle ⅓' : 'Option 2 — Lot entier' }}
          </button>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const store = useStrategyParamsStore()

// ── Straddle ──────────────────────────────────────────────────────────────────

const straddleFields = [
  { key: 'atr_periode',     label: 'Période ATR',         step: 1,     min: 5    },
  { key: 'atr_seuil',       label: 'Seuil ATR (×moy)',    step: 0.1,   min: 0.5  },
  { key: 'tp_mult_1',       label: 'TP1 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'tp_mult_2',       label: 'TP2 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'tp_mult_3',       label: 'TP3 × ATR',           step: 0.1,   min: 0.5  },
  { key: 'sl_mult',         label: 'SL × ATR',            step: 0.1,   min: 0.1  },
  { key: 'trailing_atr',    label: 'Trailing Stop × ATR', step: 0.1,   min: 0.0  },
]

const savingStraddle = ref(false)
const msgStraddle = ref<{ ok: boolean; text: string } | null>(null)

async function sauvegarderStraddle() {
  savingStraddle.value = true; msgStraddle.value = null
  try { await store.saveStraddle(store.straddleRaw); msgStraddle.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgStraddle.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingStraddle.value = false }
}

// ── SMC ──────────────────────────────────────────────────────────────────────

const smcFields = [
  { key: 'atr_periode',     label: 'Période ATR',      step: 1,   min: 5   },
  { key: 'score_min',       label: 'Score minimum',    step: 1,   min: 40  },
  { key: 'atr_tp1',         label: 'TP1 × ATR',        step: 0.1, min: 0.5 },
  { key: 'atr_tp2',         label: 'TP2 × ATR',        step: 0.1, min: 0.5 },
  { key: 'atr_tp3',         label: 'TP3 × ATR',        step: 0.1, min: 0.5 },
  { key: 'atr_sl',          label: 'SL × ATR',         step: 0.1, min: 0.1 },
]

const savingSmc = ref(false)
const msgSmc = ref<{ ok: boolean; text: string } | null>(null)

async function sauvegarderSmc() {
  savingSmc.value = true; msgSmc.value = null
  try { await store.saveSmc(store.smcRaw); msgSmc.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgSmc.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingSmc.value = false }
}

// ── Rockets ───────────────────────────────────────────────────────────────────

const rocketsFields = [
  { key: 'score_min',        label: 'Score minimum',    step: 1,     min: 20    },
  { key: 'rsi_max',          label: 'RSI max',          step: 1,     min: 50    },
  { key: 'rsi_min',          label: 'RSI min',          step: 1,     min: 0     },
  { key: 'ratio_volume_min', label: 'Volume ratio min', step: 0.1,   min: 1     },
  { key: 'vol_marche_min',   label: 'Vol. marché min',  step: 10000, min: 10000 },
  { key: 'sl_mult',          label: 'SL × ATR',         step: 0.1,   min: 0.1   },
  { key: 'tp_mult_1',        label: 'TP1 × ATR',        step: 0.1,   min: 0.5   },
  { key: 'tp_mult_2',        label: 'TP2 × ATR',        step: 0.1,   min: 0.5   },
  { key: 'tp_mult_3',        label: 'TP3 × ATR',        step: 0.1,   min: 0.5   },
  { key: 'trailing_atr',     label: 'Trailing Stop × ATR', step: 0.1, min: 0.5  },
]

const savingRockets = ref(false)
const msgRockets = ref<{ ok: boolean; text: string } | null>(null)

async function sauvegarderRockets() {
  savingRockets.value = true; msgRockets.value = null
  try { await store.saveRockets(store.rocketsRaw); msgRockets.value = { ok: true, text: 'Sauvegardé ✓' } }
  catch (err: any) { msgRockets.value = { ok: false, text: `Erreur: ${err.message}` } }
  finally { savingRockets.value = false }
}

onMounted(() => store.charger())
</script>
