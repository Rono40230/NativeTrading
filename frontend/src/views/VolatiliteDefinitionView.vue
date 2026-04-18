<template>
  <div class="flex flex-col gap-4 p-6 h-full w-full">

    <!-- Header -->
    <div class="flex items-baseline gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">⚡ Stratégie Volatilité</h1>
      <span class="text-gray-500 text-base">Straddle — définition, mécanique et rôle de l'IA</span>
    </div>

    <!-- Ligne 1 : Concept + Paramètres actifs -->
    <div class="grid grid-cols-[3fr_2fr] gap-4 items-stretch shrink-0">

      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest mb-3">Concept</div>
        <p class="text-gray-300 text-base leading-relaxed">
          La stratégie Volatilité ouvre des positions
          <span class="text-white font-medium">LONG + SHORT simultanées</span> lors d'un éclatement de volatilité
          extrême (annonces macro, sessions d'ouverture). Le mouvement est capturé dans les deux sens —
          seule la direction qui atteint son TP1 est conservée, l'autre est coupée. Le risque total est
          <span class="text-white font-medium">limité à 2% du capital</span> (1% par direction).
        </p>
      </div>

      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="mb-4">
          <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest">Paramètres actifs</div>
        </div>
        <div v-if="params" class="flex flex-wrap gap-3">
          <div v-for="p in paramCards" :key="p.label" class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5">
            <span class="text-xs text-gray-500">{{ p.label }}</span>
            <span class="text-white font-bold text-xl">{{ p.value }}</span>
          </div>
        </div>
        <div v-else class="text-sm text-gray-500 animate-pulse">Chargement…</div>
      </div>

    </div>

    <!-- Ligne 2 : 3 colonnes -->
    <div class="grid grid-cols-3 gap-4 items-stretch flex-1 min-h-0">

      <!-- Col 1 : Phases / Mécanique -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest mb-4">Mécanique d'exécution</div>
        <div class="flex flex-col gap-4 flex-1 justify-between">
          <div v-for="phase in phases" :key="phase.id" class="rounded-lg bg-black/20 border border-white/5 px-4 py-4">
            <div class="flex items-center gap-2 mb-2">
              <span class="text-xl leading-none">{{ phase.icon }}</span>
              <span class="text-white font-semibold text-base">{{ phase.label }}</span>
            </div>
            <p class="text-gray-500 text-sm mb-2.5">{{ phase.description }}</p>
            <div class="space-y-1.5">
              <div v-for="c in phase.details" :key="c"
                class="text-sm text-gray-300 pl-3 border-l-2 border-yellow-500/40 leading-snug">{{ c }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 2 : Déclencheurs -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest mb-4">Conditions de déclenchement</div>
        <div class="flex flex-col gap-3 flex-1 justify-between">
          <div v-for="s in conditions" :key="s.label" class="rounded-lg bg-black/20 border border-white/5 px-4 py-3">
            <div class="text-base font-semibold text-white mb-1.5">{{ s.label }}</div>
            <div class="text-sm text-gray-400 leading-relaxed">{{ s.detail }}</div>
          </div>
          <div class="rounded-lg bg-black/30 border border-yellow-500/20 px-4 py-3">
            <div class="text-xs text-gray-500 mb-2">Risk management</div>
            <div class="space-y-1">
              <div class="text-sm text-gray-300">1% capital par direction (LONG + SHORT)</div>
              <div class="text-sm text-gray-300">Total exposé : 2% maximum</div>
              <div class="text-sm text-gray-300">Max 1 straddle simultané</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 3 : Rôle de l'IA -->
      <div class="flex flex-col gap-4 overflow-y-auto min-h-0">

        <div class="rounded-xl border border-yellow-500/30 bg-yellow-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl">⚡</span>
            <span class="text-white font-semibold text-base">Signal temps réel</span>
            <span class="ml-auto text-sm text-yellow-400 bg-yellow-500/10 px-2.5 py-0.5 rounded-full">Boucle surveillance</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-4">
            Décide en temps réel d'entrer un straddle ou non lors d'un éclatement de volatilité.
            Retourne direction suggérée, conviction et SL/TP ajustés.
          </p>
          <div class="space-y-2 mb-4">
            <div v-for="r in signalRegles" :key="r.label" class="flex items-center gap-2 text-sm">
              <span :class="r.couleur" class="shrink-0">{{ r.icon }}</span>
              <span class="text-gray-300">{{ r.label }}</span>
            </div>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-3">
            <div class="text-xs text-gray-500 mb-2">Barème de conviction</div>
            <div class="flex gap-5">
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-green-400 font-bold">≥ 75</span>
                <span class="text-gray-400">Entre ✅</span>
              </div>
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-red-400 font-bold">&lt; 75</span>
                <span class="text-gray-400">Passe 🚫</span>
              </div>
            </div>
          </div>
        </div>

        <div class="rounded-xl border border-purple-500/30 bg-purple-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl">📊</span>
            <span class="text-white font-semibold text-base">Analyse stratégique IA</span>
            <span class="ml-auto text-sm text-purple-400 bg-purple-500/10 px-2.5 py-0.5 rounded-full">Sur demande</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-3">
            Analyse les backtests Straddle pour recommander l'ajustement des créneaux temporels et des multiplicateurs ATR.
          </p>
          <div class="space-y-1.5">
            <div v-for="o in analyseOutputs" :key="o" class="text-sm text-gray-300 flex items-center gap-2">
              <span class="text-purple-400 shrink-0">→</span>{{ o }}
            </div>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const strategyStore = useStrategyParamsStore()
const params = ref<Record<string, number> | null>(null)
onMounted(async () => {
  try { await strategyStore.charger(); params.value = { ...strategyStore.straddleRaw } } catch { /* silencieux */ }
})

const paramCards = computed(() => params.value ? [
  { label: 'ATR seuil',   value: `${params.value.atr_seuil}×` },
  { label: 'ATR période', value: params.value.atr_periode },
  { label: 'TP1',         value: `${params.value.tp_mult_1}×` },
  { label: 'TP2',         value: `${params.value.tp_mult_2}×` },
  { label: 'TP3',         value: `${params.value.tp_mult_3}×` },
  { label: 'SL',          value: `${params.value.sl_mult}×` },
  { label: 'Horizon',     value: `${params.value.horizon_bougies}b` },
] : [])

const phases = [
  { id: 'declenchement', icon: '🔥', label: 'Déclenchement',
    description: 'L\'ATR dépasse le seuil configuré — volatilité extrême détectée.',
    details: ['ATR > seuil × ATR moyen 14p', 'Sur créneau horaire ciblé', 'IA valide l\'opportunité'] },
  { id: 'entree', icon: '↕️', label: 'Entrée simultanée',
    description: 'Deux ordres opposés sont ouverts instantanément.',
    details: ['LONG : SL = ATR × 0.5× / TP1 = ATR × TP1', 'SHORT : miroir symétrique', 'Taille = 1% capital chacun'] },
  { id: 'gestion', icon: '△', label: 'Gestion pyramidale',
    description: 'La direction validée est conservée sur 3 TP, l\'autre coupée.',
    details: ['TP1 atteint → clôture 50% + trailing', 'TP2 → clôture 30% supplémentaire', 'TP3 → solde final'] },
]

const conditions = [
  { label: 'ATR explosif',      detail: `ATR courant > ${1.5}× ATR moyen 14 périodes` },
  { label: 'Créneau horaire',   detail: 'Ouverture Londres 08h, NY 14h30, ou événement macro planifié' },
  { label: 'IA indécise',       detail: 'Modèle ML ne donne pas de direction claire (biais < 0.55)' },
  { label: 'Pas de position',   detail: 'Aucun straddle déjà ouvert sur cet actif' },
]

const signalRegles = [
  { icon: '🚫', couleur: 'text-red-400',    label: 'ATR < seuil → ne pas entrer' },
  { icon: '🚫', couleur: 'text-red-400',    label: 'Drawdown jour > 3% → bloquer' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'Spread élevé → réduire taille' },
  { icon: '✅', couleur: 'text-green-400',  label: 'Peut ajuster SL/TP selon volatilité' },
]

const analyseOutputs = [
  'Créneaux les plus profitables (win rate, PF)',
  'Multiplicateurs ATR optimaux observés',
  '3 à 5 recommandations de créneaux/paramètres',
]
</script>

