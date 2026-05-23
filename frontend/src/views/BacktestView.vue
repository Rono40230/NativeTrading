<template>
  <div class="h-full flex flex-col gap-4 overflow-hidden">

    <!-- En-tête -->
    <div class="shrink-0 flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold flex items-center gap-2">🧪 Backtest</h1>
      </div>
      <div v-if="store.chargement" class="text-xs text-blue-400 animate-pulse">⏳ Calcul en cours…</div>
    </div>

    <!-- ── Barre de configuration horizontale ──────────────────────────── -->
    <section class="shrink-0 glass-card rounded-xl border border-white/10 bg-white/5 p-3">
      <div class="flex flex-wrap items-end gap-4">

        <div class="flex flex-col gap-1 min-w-[140px]">
          <label class="text-[10px] uppercase text-gray-500 font-semibold">Asset</label>
          <select v-model="form.asset" class="champ-form">
            <option v-for="a in assetsDisponibles" :key="a.id" :value="a.id">{{ a.id }}</option>
          </select>
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[10px] uppercase text-gray-500 font-semibold">Timeframe</label>
          <select v-model="form.timeframe" class="champ-form">
            <option v-for="tf in TIMEFRAMES" :key="tf" :value="tf">{{ tf }}</option>
          </select>
        </div>

        <div class="flex flex-col gap-1 w-24">
          <label class="text-[10px] uppercase text-gray-500 font-semibold">Jours</label>
          <input v-model.number="form.nb_jours" type="number" min="7" max="365" class="champ-form" />
        </div>

        <div class="flex flex-col gap-1 w-28">
          <label class="text-[10px] uppercase text-gray-500 font-semibold">Capital ($)</label>
          <input v-model.number="form.capital" type="number" min="100" class="champ-form" />
        </div>

        <div class="flex flex-col gap-1 w-28">
          <label class="text-[10px] uppercase text-gray-500 font-semibold">Risque / trade (%)</label>
          <input v-model.number="form.risquePct" type="number" min="0.1" max="5" step="0.1" class="champ-form" />
        </div>

        <button
          class="px-5 py-2 rounded-lg font-semibold text-sm transition-all disabled:opacity-40 shrink-0"
          :class="store.chargement ? 'bg-gray-700' : 'bg-blue-600 hover:bg-blue-500'"
          :disabled="store.chargement"
          @click="lancer"
        >
          {{ store.chargement ? '⏳' : '▶ Lancer' }}
        </button>

      </div>
    </section>

    <!-- ── Tableau comparatif Straddle vs SMC ─────────────────────────── -->
    <section
      v-if="store.resultStraddle || store.resultSmc"
      class="shrink-0 glass-card rounded-xl border border-blue-500/20 bg-blue-500/5 p-3"
    >
      <h3 class="text-xs font-semibold text-blue-300 mb-2">📊 Comparatif</h3>
      <table class="w-full text-xs">
        <thead>
          <tr class="text-gray-400 border-b border-white/10">
            <th class="text-left pb-2 pr-4 font-semibold text-gray-300">Stratégie</th>
            <th
              v-for="col in colonnes"
              :key="col.key"
              class="text-right pb-2 px-2 font-normal"
            >
              <div class="relative inline-block group cursor-help">
                <span class="border-b border-dotted border-gray-600 leading-tight">{{ col.label }}</span>
                <!-- Tooltip -->
                <div class="pointer-events-none invisible group-hover:visible opacity-0 group-hover:opacity-100 transition-opacity duration-150 absolute z-50 bottom-full right-0 mb-2 w-64 rounded-lg bg-gray-900 border border-white/20 p-3 shadow-2xl text-left normal-case font-normal text-gray-300 text-[11px] leading-relaxed whitespace-normal">
                  <p class="font-semibold text-white mb-1">{{ col.label }}</p>
                  <p :class="col.echelle ? 'mb-1.5' : ''">{{ col.tooltip }}</p>
                  <div v-if="col.echelle" class="border-t border-white/10 pt-1.5 space-y-0.5">
                    <div v-for="ligne in col.echelle" :key="ligne" :class="echelleColor(ligne)">{{ ligne }}</div>
                  </div>
                </div>
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in lignesComparatif"
            :key="row.label"
            class="border-b border-white/5 hover:bg-white/5"
          >
            <td class="py-2 pr-4 font-semibold text-gray-200 whitespace-nowrap">{{ row.label }}</td>
            <td
              v-for="col in colonnes"
              :key="col.key"
              class="text-right px-2 font-mono whitespace-nowrap"
              :class="row.couleurs[col.key]"
            >
              {{ row.valeurs[col.key] }}
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- ── 2 colonnes résultats ─────────────────────────────────────────── -->
    <div class="flex gap-4 flex-1 min-h-0 overflow-hidden">

      <!-- Vide initial -->
      <div
        v-if="!store.resultStraddle && !store.resultSmc && !store.chargement"
        class="flex-1 flex items-center justify-center text-gray-500 text-sm"
      >
        Sélectionnez un asset et un timeframe, puis lancez le backtest.
      </div>

      <template v-else>
        <!-- Colonne Straddle -->
        <div class="flex-1 min-w-0 overflow-y-auto custom-scrollbar pr-1">
          <ColonneBacktest
            titre="⚡ Straddle"
            :result="store.resultStraddle"
            :recommandations="store.recoStraddle"
            :duree_ms="store.duree_straddle"
            :chargement="store.chargement"
          />
        </div>

        <!-- Séparateur -->
        <div class="w-px bg-white/10 shrink-0 self-stretch" />

        <!-- Colonne SMC -->
        <div class="flex-1 min-w-0 overflow-y-auto custom-scrollbar pr-1">
          <ColonneBacktest
            titre="📐 SMC"
            :result="store.resultSmc"
            :recommandations="store.recoSmc"
            :duree_ms="store.duree_smc"
            :chargement="store.chargement"
          />
        </div>
      </template>

    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, computed } from 'vue'
import { useBacktestStore } from '@/stores/backtest.store'
import { useAssetsStore }   from '@/stores/assets.store'
import ColonneBacktest      from '@/components/common/ColonneBacktest.vue'

const store       = useBacktestStore()
const assetsStore = useAssetsStore()

const TIMEFRAMES = ['M1', 'M5', 'M15', 'H1']

const assetsDisponibles = computed(() =>
  assetsStore.assets.length > 0
    ? assetsStore.assets.map(a => ({ id: a.id, label: a.nom }))
    : [{ id: 'BTC', label: 'BTC' }, { id: 'ETH', label: 'ETH' }, { id: 'XAUUSD', label: 'XAUUSD' }, { id: 'XAGUSD', label: 'XAGUSD' }]
)

const form = reactive({
  asset:     assetsStore.assets[0]?.id ?? 'XAUUSD',
  timeframe: 'M15',
  nb_jours:  90,
  capital:   10_000,
  risquePct: 2,
})

import type { BacktestResult }    from '@/services/api.backtest'
import { colonnes, echelleColor } from '@/composables/useBacktestColonnes'
import type { ColonneComp }       from '@/composables/useBacktestColonnes'

type LigneComp = { label: string; valeurs: Record<string, string>; couleurs: Record<string, string> }

const lignesComparatif = computed((): LigneComp[] => {
  const rows: LigneComp[] = []
  for (const [label, result] of [['⚡ Straddle', store.resultStraddle], ['📐 SMC', store.resultSmc]] as [string, BacktestResult | null][]) {
    if (!result) continue
    const valeurs: Record<string, string> = {}
    const couleurs: Record<string, string> = {}
    for (const col of colonnes) {
      valeurs[col.key] = col.valeur(result)
      couleurs[col.key] = col.couleur(result)
    }
    rows.push({ label, valeurs, couleurs })
  }
  return rows
})

async function lancer(): Promise<void> {
  await store.lancerComparaison({
    asset:      form.asset,
    timeframe:  form.timeframe,
    nb_jours:   form.nb_jours,
    capital:    form.capital,
    risque_pct: form.risquePct,
  })
}
</script>

<style scoped>
.champ-form {
  background: #ffffff;
  color: #111827;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  padding: 0.375rem 0.625rem;
  font-size: 0.875rem;
  height: 2.25rem;
  width: 100%;
  outline: none;
  cursor: pointer;
}
.champ-form:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
}
</style>
