<template>
  <div class="flex flex-col gap-4 p-6 h-full w-full">

    <!-- Header -->
    <div class="flex items-baseline gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🚀 Stratégie Rockets</h1>
      <span class="text-gray-500 text-base">Définition, logique de détection et rôle de l'IA</span>
    </div>

    <!-- Ligne 1 : Concept + Paramètres actifs -->
    <div class="grid grid-cols-[3fr_2fr] gap-4 items-stretch shrink-0">

      <!-- Concept -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-3">Concept</div>
        <p class="text-gray-300 text-base leading-relaxed">
          Rockets capture les <span class="text-white font-medium">mouvements explosifs</span> après une
          <span class="text-white font-medium">compression de volatilité</span> : range serré, ATR faible,
          volume se contractant → énergie accumulée — puis cassure avec spike de volume déclenchant un
          mouvement directionnel violent et rapide. Plus la compression est longue, plus le breakout est violent.
        </p>
      </div>

      <!-- Paramètres actifs -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="mb-4">
          <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest">Paramètres actifs</div>
        </div>
        <div v-if="config" class="flex flex-wrap gap-3">
          <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5">
            <span class="text-xs text-gray-500">Score min</span>
            <span class="text-white font-bold text-xl">{{ config.score_min }}</span>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5">
            <span class="text-xs text-gray-500">RSI</span>
            <span class="text-white font-bold text-xl">{{ config.rsi_min }}–{{ config.rsi_max }}</span>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5">
            <span class="text-xs text-gray-500">Vol. ratio min</span>
            <span class="text-white font-bold text-xl">{{ config.ratio_volume_min }}×</span>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-0.5">
            <span class="text-xs text-gray-500">Vol. marché</span>
            <span class="text-white font-bold text-xl">{{ (config.vol_marche_min / 1_000_000).toFixed(0) }}M$</span>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-2.5 flex flex-col gap-1.5">
            <span class="text-xs text-gray-500">Phases actives</span>
            <div class="flex gap-1.5 flex-wrap">
              <span v-for="p in config.phases_actives" :key="p"
                class="text-sm bg-orange-500/10 text-orange-300 border border-orange-500/20 px-2.5 py-0.5 rounded-full">{{ p }}</span>
            </div>
          </div>
        </div>
      </div>

    </div>

    <!-- Ligne 2 : 3 colonnes — remplit l'espace restant -->
    <div class="grid grid-cols-3 gap-4 items-stretch flex-1 min-h-0">

      <!-- Col 1 : 3 phases -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-4">Les 3 phases détectées</div>
        <div class="flex flex-col gap-4 flex-1 justify-between">
          <div v-for="phase in phases" :key="phase.id"
            class="rounded-lg bg-black/20 border border-white/5 px-4 py-4">
            <div class="flex items-center gap-2 mb-2">
              <span class="text-xl leading-none">{{ phase.icon }}</span>
              <span class="text-white font-semibold text-base">{{ phase.label }}</span>
            </div>
            <p class="text-gray-500 text-sm mb-3">{{ phase.description }}</p>
            <div class="space-y-1.5">
              <div v-for="c in phase.criteres" :key="c"
                class="text-sm text-gray-300 pl-3 border-l-2 border-orange-500/40 leading-snug">{{ c }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 2 : Scoring -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-4">Système de scoring</div>
        <div class="flex flex-col gap-3 flex-1 justify-between">
          <div v-for="s in scoring" :key="s.label"
            class="rounded-lg bg-black/20 border border-white/5 px-4 py-3">
            <div class="text-base font-semibold text-white mb-1.5">{{ s.label }}</div>
            <div class="text-sm text-gray-400 leading-relaxed">{{ s.detail }}</div>
          </div>
        </div>
      </div>

      <!-- Col 3 : Rôle de l'IA -->
      <div class="flex flex-col gap-4 overflow-y-auto min-h-0">

        <!-- Filtre temps réel -->
        <div class="rounded-xl border border-blue-500/30 bg-blue-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl leading-none">⚡</span>
            <span class="text-white font-semibold text-base">Filtre IA temps réel</span>
            <span class="ml-auto text-sm text-blue-400 bg-blue-500/10 px-2.5 py-0.5 rounded-full">Avant chaque signal</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-4">
            Avant de sauvegarder un candidat, l'IA valide ou rejette le signal et retourne
            une conviction (0–100) + une raison en 120 caractères.
          </p>
          <div class="space-y-2 mb-4">
            <div v-for="r in filtreRegles" :key="r.label" class="flex items-center gap-2 text-sm">
              <span :class="r.couleur" class="shrink-0">{{ r.icon }}</span>
              <span class="text-gray-300">{{ r.label }}</span>
            </div>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-3">
            <div class="text-xs text-gray-500 mb-2">Barème de conviction</div>
            <div class="flex gap-5">
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-green-400 font-bold">80–100</span>
                <span class="text-gray-400">Validé ✅</span>
              </div>
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-yellow-400 font-bold">65–79</span>
                <span class="text-gray-400">Validé ✅</span>
              </div>
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-red-400 font-bold">&lt; 65</span>
                <span class="text-gray-400">Rejeté 🚫</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Analyse stratégique -->
        <div class="rounded-xl border border-purple-500/30 bg-purple-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl leading-none">📊</span>
            <span class="text-white font-semibold text-base">Analyse stratégique IA</span>
            <span class="ml-auto text-sm text-purple-400 bg-purple-500/10 px-2.5 py-0.5 rounded-full">Sur demande</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-4">
            Analyse les signaux clôturés (≥ 5 trades) pour évaluer la performance globale
            et recommander des ajustements de paramètres.
          </p>
          <div class="space-y-2 mb-4">
            <div v-for="o in analyseOutputs" :key="o"
              class="text-sm text-gray-300 flex items-center gap-2">
              <span class="text-purple-400 shrink-0">→</span>{{ o }}
            </div>
          </div>
          <div class="rounded-lg bg-black/30 px-4 py-3">
            <div class="text-xs text-gray-500 mb-2">Recommandations portent sur</div>
            <div class="flex flex-wrap gap-1.5">
              <span v-for="t in recommendationTypes" :key="t"
                class="text-sm bg-purple-500/10 text-purple-300 border border-purple-500/20 px-2.5 py-0.5 rounded-full">{{ t }}</span>
            </div>
          </div>
        </div>

      </div>

    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'
import type { RocketsConfig } from '@/services/api.types'

const strategyStore = useStrategyParamsStore()
const config = ref<RocketsConfig | null>(null)

onMounted(async () => {
  try { await strategyStore.charger(); config.value = { ...strategyStore.rocketsRaw } as RocketsConfig } catch { /* silencieux */ }
})

const phases = [
  {
    id: 'prelancement', icon: '🔋', label: 'Pré-lancement',
    description: 'Compression de volatilité — l\'actif accumule de l\'énergie dans un range serré.',
    criteres: ['ATR ratio < 0.80', 'Volume se contractant', '≥ 5 bougies en compression']
  },
  {
    id: 'breakout', icon: '💥', label: 'Breakout',
    description: 'Cassure de la résistance avec conviction — mouvement directionnel explosif.',
    criteres: ['Volume spike > 1.5×', 'ATR ratio > 1.0', 'RSI idéal 55–75', 'Change 1h > 0%']
  },
  {
    id: 'momentum', icon: '⚡', label: 'Compression Momentum',
    description: 'Compression avec élan 1h — mouvement déjà amorcé.',
    criteres: ['Change 1h > 0.5%', 'Score ≥ 15', 'Phase compression active']
  },
]

const scoring = [
  { label: 'Ratio volume', detail: '≥ 2.0× fort | 1.5–2.0× acceptable | < 1.5× signal faible' },
  { label: 'RSI', detail: '55–75 idéal | > 85 surachat → invalider' },
  { label: 'ATR ratio', detail: '> 1.2 bonne expansion | < 0.8 compression' },
  { label: 'Tendance EMA', detail: 'EMA20 > EMA50 = haussier confirmé (+10 conviction)' },
  { label: 'Compression', detail: '≥ 10 bougies = forte (+10) | ≥ 5 = significative (+5)' },
  { label: 'Ratio corps', detail: '> 0.7 corps fort ✅ | < 0.3 rejet par mèche ❌' },
]

const filtreRegles = [
  { icon: '🚫', couleur: 'text-red-400', label: 'RSI > 85 → invalider (surachat extrême)' },
  { icon: '🚫', couleur: 'text-red-400', label: 'Ratio corps < 0.3 → invalider ou dégrader' },
  { icon: '🚫', couleur: 'text-red-400', label: 'Compression < 3 bougies en prelancement → invalider' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'Tendance baissière (EMA) → −20 conviction' },
  { icon: '✅', couleur: 'text-green-400', label: 'Peut suggérer SL/TP1 ajustés si justifié' },
]

const analyseOutputs = [
  'Synthèse de la performance globale (2–3 phrases)',
  'Meilleur setup observé (phase, score, RSI, volume)',
  'Pire setup à éviter',
  '3 à 6 recommandations classées par impact',
]

const recommendationTypes = ['seuil_score', 'filtre_phase', 'coefficients_atr', 'filtre_rsi', 'filtre_volume', 'mode_entree']
</script>
