<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-y-auto">

    <!-- Header -->
    <div class="flex items-baseline gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">⚡ Stratégie Volatilité</h1>
      <span class="text-gray-500 text-base hidden sm:inline">Straddle — définition, mécanique et rôle de l'IA</span>
    </div>

    <!-- Barre santé -->
    <DefinitionSanteBar :warm-start="estWarmStart" :seuil-llm="scoreLlmEffectif + '/10'" class="shrink-0" />

    <!-- Ligne 1 : Concept + Paramètres actifs -->
    <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-4 items-stretch shrink-0">

      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4">
        <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest mb-3">Concept</div>
        <p class="text-gray-300 text-sm leading-relaxed">
          La stratégie Volatilité ouvre des positions
          <span class="text-white font-medium">LONG + SHORT simultanées</span> lors d'un éclatement de
          <DefinitionTerme definition="Average True Range — mesure la volatilité réelle d'une bougie, intégrant les gaps overnight.">ATR</DefinitionTerme>
          extrême (annonces macro, sessions d'ouverture).
          Ce pattern
          <DefinitionTerme definition="Stratégie options/trading qui profite de la volatilité quelle que soit la direction — deux positions opposées simultanées.">Straddle</DefinitionTerme>
          capture la direction gagnante et coupe l'autre.
          Risque total limité à <span class="text-white font-medium">2% du capital</span>.
        </p>
      </div>

      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4">
        <div class="text-xs font-semibold text-yellow-400 uppercase tracking-widest mb-3">Paramètres actifs</div>
        <div v-if="params" class="flex flex-wrap gap-2">
          <DefinitionParamCard v-for="p in paramCards" :key="p.label" :label="p.label" :value="p.value" :badge="p.badge" />
        </div>
        <div v-else class="text-sm text-gray-500 animate-pulse">Chargement…</div>
      </div>

    </div>

    <!-- Ligne 2 : responsive 3 colonnes -->
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4 items-stretch flex-1">

      <!-- Col 1 : Mécanique -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full justify-start">
        <button class="flex items-center gap-2 w-full text-left" @click="colOpen[0] = !colOpen[0]">
          <span class="text-xs font-semibold text-yellow-400 uppercase tracking-widest flex-1">Mécanique d'exécution</span>
          <span class="text-gray-500 text-xs xl:hidden">{{ colOpen[0] ? '▲' : '▼' }}</span>
        </button>
        <div :class="['flex flex-col gap-3 mt-3', !colOpen[0] && 'hidden xl:flex']">
          <div v-for="phase in phases" :key="phase.id" class="rounded-lg bg-black/20 border border-white/5 px-3 py-3">
            <div class="flex items-center gap-2 mb-1.5">
              <span class="text-lg leading-none">{{ phase.icon }}</span>
              <span class="text-white font-semibold text-sm">{{ phase.label }}</span>
            </div>
            <p class="text-gray-500 text-xs mb-1.5">{{ phase.description }}</p>
            <div class="flex flex-wrap gap-1.5 mt-2">
              <span v-for="c in phase.details" :key="c" class="text-[10px] sm:text-xs bg-yellow-500/10 text-yellow-200 border border-yellow-500/20 px-2 py-0.5 rounded-md whitespace-nowrap">{{ c }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 2 : Conditions -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full justify-start">
        <button class="flex items-center gap-2 w-full text-left" @click="colOpen[1] = !colOpen[1]">
          <span class="text-xs font-semibold text-yellow-400 uppercase tracking-widest flex-1">Conditions de déclenchement</span>
          <span class="text-gray-500 text-xs xl:hidden">{{ colOpen[1] ? '▲' : '▼' }}</span>
        </button>
        <div :class="['flex flex-col gap-2 mt-3', !colOpen[1] && 'hidden xl:flex']">
          <div v-for="s in conditions" :key="s.label" class="flex flex-col xl:flex-row xl:items-center gap-1.5 xl:gap-3 rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <span class="text-xs xl:text-sm font-semibold text-white whitespace-nowrap xl:w-28 shrink-0">{{ s.label }}</span>
            <div class="flex flex-wrap gap-1.5 flex-1">
              <span v-for="badge in s.detail.split(' | ')" :key="badge" class="text-[10px] xl:text-xs bg-white/5 text-gray-300 border border-white/10 px-2 py-0.5 rounded-md">{{ badge }}</span>
            </div>
          </div>
          <div class="rounded-lg bg-black/30 border border-yellow-500/20 px-4 py-2.5 mt-1">
            <div class="text-xs text-gray-500 mb-1.5">Risk management</div>
            <div class="space-y-1 text-xs text-gray-300">
              <div>1% capital par direction (LONG + SHORT)</div>
              <div>Total exposé : 2% maximum</div>
              <div>Max 1 straddle simultané</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 3 : IA -->
      <div class="flex flex-col gap-3 lg:col-span-2 xl:col-span-1 h-full">

        <div class="rounded-xl border border-yellow-500/30 bg-yellow-500/5 px-5 py-4">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-lg">⚡</span>
            <span class="text-white font-semibold text-sm">Signal temps réel</span>
            <span class="ml-auto text-xs text-yellow-400 bg-yellow-500/10 px-2 py-0.5 rounded-full">Boucle surveillance</span>
          </div>
          <p class="text-gray-400 text-xs leading-relaxed mb-3">
            Décide d'entrer un
            <DefinitionTerme definition="Stratégie deux positions opposées — profite de la volatilité quelle que soit la direction.">Straddle</DefinitionTerme>
            lors d'un éclatement de volatilité. Retourne direction, conviction et SL/TP ajustés.
          </p>
          <div class="space-y-1.5 mb-3">
            <DefinitionLlmRegle v-for="r in signalRegles" :key="r.label" v-bind="r" />
          </div>
          <div class="rounded-lg bg-black/30 px-3 py-2.5">
            <div class="text-xs text-gray-500 mb-1">Seuil conviction LLM</div>
            <div class="flex items-center gap-2 text-sm">
              <span class="text-yellow-400 font-bold">{{ scoreLlmEffectif }}/10</span>
              <span class="text-gray-400 text-xs">calibré par asset / catégorie</span>
            </div>
            <div class="text-xs mt-1" :class="estWarmStart ? 'text-amber-500' : 'text-gray-500'">
              {{ estWarmStart ? 'Warm start actif (&lt;5 feedbacks)' : 'Calibration active' }}
            </div>
          </div>
        </div>

        <div class="rounded-xl border border-purple-500/30 bg-purple-500/5 px-5 py-3 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-lg">📊</span>
            <span class="text-white font-semibold text-sm">Analyse stratégique IA</span>
            <span class="ml-auto text-xs text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded-full">Sur demande</span>
          </div>
          <div class="space-y-1">
            <div v-for="o in analyseOutputs" :key="o" class="text-xs text-gray-300 flex items-center gap-2">
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
import { apiService } from '@/services/api.service'
import type { StraddleSeuilsEffectifs } from '@/services/api.types'
import DefinitionParamCard from '@/components/common/DefinitionParamCard.vue'
import DefinitionLlmRegle from '@/components/common/DefinitionLlmRegle.vue'
import DefinitionTerme from '@/components/common/DefinitionTerme.vue'
import DefinitionSanteBar from '@/components/common/DefinitionSanteBar.vue'

const strategyStore = useStrategyParamsStore()
const params = ref<Record<string, number> | null>(null)
const seuilsEffectifs = ref<StraddleSeuilsEffectifs | null>(null)
const colOpen = ref([true, true])

onMounted(async () => {
  try { await strategyStore.charger(); params.value = { ...strategyStore.straddleRaw } } catch { /* silencieux */ }
  try { seuilsEffectifs.value = await apiService.getStraddleSeuilsEffectifs() } catch { /* silencieux */ }
})

const paramCards = computed(() => params.value ? [
  { label: 'ATR seuil',   value: `${params.value.atr_seuil}×`,         badge: undefined },
  { label: 'ATR période', value: params.value.atr_periode,             badge: undefined },
  { label: 'SL',          value: '±0.5×ATR',                           badge: 'formula' as const },
  { label: 'TP1',         value: '+2.0×ATR',                           badge: 'formula' as const },
  { label: 'TP2',         value: '+3.5×ATR',                           badge: 'formula' as const },
  { label: 'TP3',         value: '+5.0×ATR',                           badge: 'formula' as const },
  { label: 'Horizon',     value: `${params.value.horizon_bougies}b`,   badge: undefined },
] : [])

const scoreLlmEffectif = computed(() => seuilsEffectifs.value?.score_llm?.toFixed(1) ?? '5.5')
const estWarmStart = computed(() => seuilsEffectifs.value ? seuilsEffectifs.value.score_llm <= 5.5 : true)

const phases = [
  { id: 'declenchement', icon: '🔥', label: 'Déclenchement',
    description: 'L\'ATR dépasse le seuil configuré — volatilité extrême détectée.',
    details: ['ATR > seuil × ATR moyen 14p', 'Sur créneau horaire ciblé', 'IA valide l\'opportunité'] },
  { id: 'entree', icon: '↕️', label: 'Entrée simultanée',
    description: 'Deux ordres opposés ouverts instantanément.',
    details: ['LONG : SL = prix − 0.5×ATR | TP1 = prix + 2.0×ATR', 'SHORT : SL = prix + 0.5×ATR | TP1 = prix − 2.0×ATR', 'TP2 = ±3.5×ATR | TP3 = ±5.0×ATR | 1% capital/direction'] },
  { id: 'gestion', icon: '△', label: 'Gestion pyramidale',
    description: 'Direction validée conservée sur 3 TP, l\'autre coupée.',
    details: ['TP1 atteint → clôture 50% + trailing', 'TP2 → clôture 30% supplémentaire', 'TP3 → solde final'] },
]

const conditions = [
  { label: 'ATR explosif', detail: 'ATR ≥ seuil UI × ATR moyen 14p | Catégorie auto alignée' },
  { label: 'Créneau horaire', detail: 'Londres 08h / NY 14h30 UTC | Événement macro HIGH impact < 90 min' },
  { label: 'ML indécis', detail: 'Indécis = Straddle éligible | Si directionnel fort → skip' },
  { label: 'Anti-doublon', detail: 'Aucun signal actif dans les 30 dernières minutes sur l\'asset/TF' },
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
