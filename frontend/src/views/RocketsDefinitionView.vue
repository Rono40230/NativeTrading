<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-y-auto">

    <!-- En-tête et Santé -->
    <div class="flex items-center gap-4 shrink-0 flex-wrap mb-2">
      <div class="flex items-baseline gap-3">
        <h1 class="text-2xl font-bold text-white">🚀 Stratégie Rockets</h1>
        <span class="text-gray-500 text-base hidden sm:inline">Définition, logique de détection et rôle de l'IA</span>
      </div>
      <DefinitionSanteBar :conviction-min="convictionEffective" />
    </div>

    <!-- 1. Concept (Pleine largeur) -->
    <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 shrink-0">
      <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-3">Le Concept</div>
      <p class="text-gray-300 text-sm leading-relaxed max-w-4xl">
        Rockets capture les <span class="text-white font-medium">mouvements explosifs</span> après une
        compression de volatilité : range serré,
        <DefinitionTerme definition="Average True Range — mesure la volatilité réelle d'une bougie.">ATR</DefinitionTerme>
        faible, volume contracté → énergie accumulée.
        Un
        <DefinitionTerme definition="Cassure d'un niveau clé (résistance/support) avec volume — signale un mouvement directionnel.">breakout</DefinitionTerme>
        avec spike de volume déclenche le signal. Le
        <DefinitionTerme definition="Relative Strength Index (0–100) — oscille pour mesurer la force d'un mouvement.">RSI</DefinitionTerme>
        valide la santé du mouvement.
      </p>
    </div>

    <!-- 2. Pipeline d'Exécution (Les 3 étapes) -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 items-stretch shrink-0">

      <!-- Étape 1 : Détection -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full bg-gradient-to-b from-black/0 to-black/20">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-4">Étape 1 : Détection</div>
        <div class="flex flex-col gap-3 flex-1 justify-between">
          <div v-for="phase in phases" :key="phase.id" class="rounded-lg bg-black/20 border border-white/5 px-3 py-3">
            <div class="flex items-center gap-2 mb-1.5">
              <span class="text-lg leading-none">{{ phase.icon }}</span>
              <span class="text-white font-semibold text-sm">{{ phase.label }}</span>
            </div>
            <p class="text-gray-500 text-xs mb-1.5">{{ phase.description }}</p>
            <div class="flex flex-wrap gap-1.5 mt-2">
              <span v-for="c in phase.criteres" :key="c" class="text-[10px] sm:text-xs bg-orange-500/10 text-orange-200 border border-orange-500/20 px-2 py-0.5 rounded-md whitespace-nowrap">{{ c }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Étape 2 : Évaluation (Scoring) -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full bg-gradient-to-b from-black/0 to-black/20">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-4">Étape 2 : Évaluation</div>
        <div class="flex flex-col gap-2 flex-1 justify-start">
          <div v-for="s in scoring" :key="s.label" class="flex flex-col xl:flex-row xl:items-center gap-1.5 xl:gap-3 rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <span class="text-xs xl:text-sm font-semibold text-white whitespace-nowrap xl:w-28 shrink-0">{{ s.label }}</span>
            <div class="flex flex-wrap gap-1.5">
              <span v-for="badge in s.detail.split(' | ')" :key="badge" class="text-[10px] xl:text-xs bg-white/5 text-gray-300 border border-white/10 px-2 py-0.5 rounded-md">{{ badge }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Étape 3 : Validation IA -->
      <div class="rounded-xl border border-blue-500/30 bg-blue-500/5 px-5 py-4 flex flex-col h-full bg-gradient-to-b from-blue-500/5 to-blue-500/10">
        <div class="flex items-center gap-2 mb-4">
          <span class="text-lg">⚡</span>
          <span class="text-white font-semibold text-sm uppercase tracking-widest flex-1">Étape 3 : Filtre IA</span>
        </div>
        <p class="text-gray-400 text-xs leading-relaxed mb-4">
          L'IA a le dernier mot. Elle valide ou rejette chaque candidat scoré.
        </p>
        <div class="space-y-1.5 flex-1 w-full flex flex-col justify-start">
          <DefinitionLlmRegle v-for="r in filtreRegles" :key="r.label" v-bind="r" />
        </div>
        <div class="rounded-lg bg-black/30 px-3 py-2.5 mt-4 border border-blue-500/20">
          <div class="text-xs text-gray-500 mb-1">Seuil conviction IA attendu</div>
          <div class="flex items-center gap-2 text-sm">
            <span class="text-yellow-400 font-bold">≥ {{ convictionEffective }}/100</span>
            <span class="text-gray-400 text-xs">Score IA min par phase</span>
          </div>
          <div class="text-xs text-gray-500 mt-0.5">+ Score technique min : {{ scoreMinEffectif }}</div>
        </div>
      </div>

    </div>

    <!-- 3. Paramètres & Évolution -->
    <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-4 items-stretch flex-1 mt-2">

      <!-- Paramètres de l'instance -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 h-full flex flex-col">
        <div class="text-xs font-semibold text-orange-400 uppercase tracking-widest mb-3">Vos Paramètres Courants</div>
        <div v-if="config" class="flex flex-wrap gap-2 flex-1 items-start content-start">
          <DefinitionParamCard label="Score min" :value="config.score_min" />
          <DefinitionParamCard label="RSI" :value="`${config.rsi_min}–${config.rsi_max}`" />
          <DefinitionParamCard label="Vol. ratio min" :value="`${config.ratio_volume_min}×`" />
          <DefinitionParamCard label="Vol. marché" :value="`${(config.vol_marche_min / 1_000_000).toFixed(0)}M$`" />
          <div class="rounded-lg bg-black/30 px-3 py-2 flex flex-col gap-1 inline-flex">
            <span class="text-xs text-gray-500">Phases actives</span>
            <div class="flex gap-1 flex-wrap">
              <span v-for="p in config.phases_actives" :key="p"
                class="text-[10px] bg-orange-500/10 text-orange-300 border border-orange-500/20 px-2 py-0.5 rounded-full">{{ p }}</span>
            </div>
          </div>
        </div>
        <div v-else class="text-sm text-gray-500 animate-pulse">Chargement…</div>
      </div>

      <!-- Analyse Stratégique -->
      <div class="rounded-xl border border-purple-500/30 bg-purple-500/5 px-5 py-4 h-full flex flex-col relative overflow-hidden">
        <div class="absolute right-0 top-0 w-32 h-32 bg-purple-500/10 blur-3xl rounded-full"></div>
        <div class="flex items-center gap-2 mb-3 relative z-10">
          <span class="text-lg">📊</span>
          <span class="text-white font-semibold text-sm uppercase tracking-widest flex-1">Analyse Stratégique</span>
        </div>
        <p class="text-gray-400 text-xs leading-relaxed mb-4 relative z-10">
          A posteriori, après exécution des trades, le système s'améliore :
        </p>
        <div class="space-y-1.5 mb-4 flex-1 relative z-10">
          <div v-for="o in analyseOutputs" :key="o" class="text-xs text-gray-300 flex items-center gap-2">
            <span class="text-purple-400 shrink-0">→</span>{{ o }}
          </div>
        </div>
        <div class="flex flex-wrap gap-1 relative z-10">
          <span v-for="t in recommendationTypes" :key="t"
            class="text-[10px] bg-purple-500/10 text-purple-300 border border-purple-500/20 px-2 py-0.5 rounded-full">{{ t }}</span>
        </div>
      </div>

    </div>

  </div>
</template>



<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'
import { apiService } from '@/services/api.service'
import type { RocketsConfig, RocketsSeuilsEffectifs } from '@/services/api.types'
import DefinitionParamCard from '@/components/common/DefinitionParamCard.vue'
import DefinitionLlmRegle from '@/components/common/DefinitionLlmRegle.vue'
import DefinitionTerme from '@/components/common/DefinitionTerme.vue'
import DefinitionSanteBar from '@/components/common/DefinitionSanteBar.vue'

const strategyStore = useStrategyParamsStore()
const config = ref<RocketsConfig | null>(null)
const seuilsEffectifs = ref<RocketsSeuilsEffectifs | null>(null)
const colOpen = ref([true, true])

onMounted(async () => {
  try { await strategyStore.charger(); config.value = { ...strategyStore.rocketsRaw } as RocketsConfig } catch { /* silencieux */ }
  try { seuilsEffectifs.value = await apiService.getRocketsSeuilsEffectifs() } catch { /* silencieux */ }
})

const convictionEffective = computed(() => seuilsEffectifs.value?.conviction_min ?? 65)
const scoreMinEffectif = computed(() => seuilsEffectifs.value?.score_min ?? 65)

const phases = [
  { id: 'prelancement', icon: '🔋', label: 'Pré-lancement',
    description: "Compression de volatilité — l'actif accumule de l'énergie dans un range serré.",
    criteres: ['ATR ratio < 0.80', 'Volume se contractant', '≥ 5 bougies en compression'] },
  { id: 'breakout', icon: '💥', label: 'Breakout',
    description: 'Cassure de la résistance avec conviction — mouvement directionnel explosif.',
    criteres: ['Volume spike > 1.5×', 'ATR ratio > 1.0', 'RSI idéal 55–75', 'Change 1h > 0%'] },
  { id: 'momentum', icon: '⚡', label: 'Compression Momentum',
    description: 'Compression avec élan 1h — mouvement déjà amorcé.',
    criteres: ['Change 1h > 0.5%', 'Score ≥ 15', 'Phase compression active'] },
]

const scoring = [
  { label: 'Ratio volume',   detail: '≥ 2.0× fort | 1.5–2.0× acceptable | < 1.5× signal faible' },
  { label: 'RSI',            detail: '55–75 idéal | > 85 surachat → invalider' },
  { label: 'ATR ratio',      detail: '> 1.2 bonne expansion | < 0.8 compression' },
  { label: 'Tendance EMA',   detail: 'EMA20 > EMA50 = haussier confirmé (+10 conviction)' },
  { label: 'Compression',    detail: '≥ 10 bougies = forte (+10) | ≥ 5 = significative (+5)' },
  { label: 'Ratio corps',    detail: '> 0.7 corps fort ✅ | < 0.3 rejet par mèche ❌' },
  { label: 'Trailing coeff', detail: 'LLM peut ajuster entre trailing_coeff_min et trailing_coeff_max (clampé config)' },
]

const filtreRegles = [
  { icon: '🚫', couleur: 'text-red-400',    label: 'RSI > 85 → invalider (surachat extrême)' },
  { icon: '🚫', couleur: 'text-red-400',    label: 'Ratio corps < 0.3 → invalider ou dégrader' },
  { icon: '🚫', couleur: 'text-red-400',    label: 'Compression < 3 bougies en prelancement → invalider' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'Tendance baissière (EMA) → −20 conviction' },
  { icon: '✅', couleur: 'text-green-400',  label: 'Peut suggérer SL/TP1 ajustés et trailing_coeff si justifié' },
]

const analyseOutputs = [
  'Synthèse de la performance globale (2–3 phrases)',
  'Meilleur setup observé (phase, score, RSI, volume)',
  'Pire setup à éviter',
  '3 à 6 recommandations classées par impact',
]

const recommendationTypes = ['seuil_score', 'filtre_phase', 'coefficients_atr', 'filtre_rsi', 'filtre_volume', 'mode_entree']
</script>
