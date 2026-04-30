<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-y-auto">

    <!-- Header -->
    <div class="flex items-baseline gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🎯 Stratégie SMC</h1>
      <span class="text-gray-500 text-base hidden sm:inline">Définition, confluences et rôle de l'IA</span>
    </div>

    <!-- Barre santé -->
    <DefinitionSanteBar :warm-start="false" seuil-llm="70/100 (calibré)" class="shrink-0" />

    <!-- Ligne 1 : Concept + Paramètres actifs -->
    <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-4 items-stretch shrink-0">

      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4">
        <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest mb-3">Concept</div>
        <p class="text-gray-300 text-sm leading-relaxed">
          SMC Directionnel trade la <span class="text-white font-medium">confluence Smart Money</span> :
          alignement de la structure de marché, des
          <DefinitionTerme definition="Zone de prix où les institutionnels ont placé de larges ordres avant une impulsion — point de re-test potentiel.">Order Blocks</DefinitionTerme>,
          des imbalances et du
          <DefinitionTerme definition="Niveaux de retrace 38.2%, 50%, 61.8% — zones d'intérêt institutionnel pour les entrées.">Fibonacci</DefinitionTerme>.
          Signal déclenché si score ≥ 70/100. Position directionnelle, pyramidale sur 3 TP.
        </p>
      </div>

      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4">
        <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest mb-3">Paramètres actifs</div>
        <div v-if="params" class="flex flex-wrap gap-2">
          <DefinitionParamCard v-for="p in paramCards" :key="p.label" :label="p.label" :value="p.value" :badge="p.badge" />
        </div>
        <div v-else class="text-sm text-gray-500 animate-pulse">Chargement…</div>
      </div>

    </div>

    <!-- Ligne 2 : responsive 3 colonnes -->
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4 items-stretch flex-1">

      <!-- Col 1 : Confluences -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full justify-start">
        <button class="flex items-center gap-2 w-full text-left" @click="colOpen[0] = !colOpen[0]">
          <span class="text-xs font-semibold text-blue-400 uppercase tracking-widest flex-1">Les 5 confluences SMC</span>
          <span class="text-gray-500 text-xs xl:hidden">{{ colOpen[0] ? '▲' : '▼' }}</span>
        </button>
        <div :class="['flex flex-col gap-3 mt-3', !colOpen[0] && 'hidden xl:flex']">
          <div v-for="c in confluences" :key="c.id" class="rounded-lg bg-black/20 border border-white/5 px-3 py-3">
            <div class="flex items-center gap-2 mb-1.5">
              <span class="text-lg leading-none">{{ c.icon }}</span>
              <span class="text-white font-semibold text-sm">{{ c.label }}</span>
              <span class="ml-auto text-xs font-bold text-green-400">+{{ c.points }}pts <span v-if="baremes" class="text-green-400/50 text-[9px]">live</span></span>
            </div>
            <p class="text-gray-500 text-xs mb-1.5">{{ c.description }}</p>
            <div class="flex flex-wrap gap-1.5 mt-2">
              <span v-for="r in c.regles" :key="r" class="text-[10px] sm:text-xs bg-blue-500/10 text-blue-200 border border-blue-500/20 px-2 py-0.5 rounded-md whitespace-nowrap">{{ r }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 2 : Scoring -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-5 py-4 flex flex-col h-full justify-start">
        <button class="flex items-center gap-2 w-full text-left" @click="colOpen[1] = !colOpen[1]">
          <span class="text-xs font-semibold text-blue-400 uppercase tracking-widest flex-1">Système de scoring</span>
          <span class="text-gray-500 text-xs xl:hidden">{{ colOpen[1] ? '▲' : '▼' }}</span>
        </button>
        <div :class="['flex flex-col gap-2 mt-3', !colOpen[1] && 'hidden xl:flex']">
          <DefinitionScoringRow v-for="s in scoring" :key="s.label" :label="s.label" :detail="s.detail" :max-pts="s.maxPts" :is-dynamic="s.isDynamic" />
          <div class="rounded-lg bg-black/30 border border-blue-500/20 px-4 py-2.5 mt-1">
            <div class="text-xs text-gray-500 mb-1.5">Seuil de déclenchement</div>
            <div class="flex gap-5">
              <div class="flex items-center gap-1.5 text-sm"><span class="text-green-400 font-bold">≥ 70</span><span class="text-gray-400">Signal ✅</span></div>
              <div class="flex items-center gap-1.5 text-sm"><span class="text-red-400 font-bold">&lt; 70</span><span class="text-gray-400">Ignoré 🚫</span></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 3 : Rôle de l'IA -->
      <div class="flex flex-col gap-3 lg:col-span-2 xl:col-span-1 h-full">

        <div class="rounded-xl border border-blue-500/30 bg-blue-500/5 px-5 py-4">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-lg">⚡</span>
            <span class="text-white font-semibold text-sm">Filtre IA temps réel</span>
            <span class="ml-auto text-xs text-blue-400 bg-blue-500/10 px-2 py-0.5 rounded-full">Avant chaque signal</span>
          </div>
          <p class="text-gray-400 text-xs leading-relaxed mb-3">Valide ou rejette chaque signal SMC candidat. Retourne conviction 0–100 + raison.</p>
          <div class="space-y-1.5">
            <DefinitionLlmRegle v-for="r in filtreRegles" :key="r.label" v-bind="r" />
          </div>
        </div>

        <div class="rounded-xl border border-cyan-500/30 bg-cyan-500/5 px-5 py-3">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-lg">🤖</span>
            <span class="text-white font-semibold text-sm">Signal auto</span>
            <span class="ml-auto text-xs text-cyan-400 bg-cyan-500/10 px-2 py-0.5 rounded-full">Boucle 15 min</span>
          </div>
          <p class="text-gray-400 text-xs leading-relaxed">
            SL sur <DefinitionTerme definition="Dernière bougie avant impulsion institutionnelle — point de re-test.">OB</DefinitionTerme> identifié.
            TP pyramidal : TP1 = <span class="text-white font-semibold">ATR×1.5</span> | TP2 = ATR×2.5 | TP3 = ATR×4.0
          </p>
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
import type { SmcBaremes } from '@/services/api.types'
import DefinitionParamCard from '@/components/common/DefinitionParamCard.vue'
import DefinitionScoringRow from '@/components/common/DefinitionScoringRow.vue'
import DefinitionLlmRegle from '@/components/common/DefinitionLlmRegle.vue'
import DefinitionTerme from '@/components/common/DefinitionTerme.vue'
import DefinitionSanteBar from '@/components/common/DefinitionSanteBar.vue'

const strategyStore = useStrategyParamsStore()
const params = ref<Record<string, number> | null>(null)
const baremes = ref<SmcBaremes | null>(null)
const colOpen = ref([true, true])

onMounted(async () => {
  try { await strategyStore.charger(); params.value = { ...strategyStore.smcRaw } } catch { /* silencieux */ }
  try { baremes.value = await apiService.getSmcBaremes() } catch { /* silencieux */ }
})

const paramCards = computed(() => params.value ? [
  { label: 'Score min',    value: params.value.score_min,             badge: undefined },
  { label: 'ATR période',  value: params.value.atr_periode,           badge: undefined },
  { label: 'TP1',          value: `${params.value.atr_tp1}×`,         badge: 'formula' as const },
  { label: 'TP2',          value: `${params.value.atr_tp2}×`,         badge: 'formula' as const },
  { label: 'TP3',          value: `${params.value.atr_tp3}×`,         badge: 'formula' as const },
  { label: 'SL',           value: `${params.value.atr_sl}×`,          badge: 'formula' as const },
  { label: 'Horizon',      value: `${params.value.horizon_bougies}b`, badge: undefined },
] : [])

const confluences = computed(() => [
  { id: 'tendance', icon: '📈', label: 'Structure de tendance', points: baremes.value?.tendance ?? 25,
    description: 'Biais directionnel basé sur la succession de sommets et creux.',
    regles: ['Haussier : HH + HL (force 2/2 = max pts)', 'Baissier : LH + LL', 'Indécis → signal annulé'] },
  { id: 'ob', icon: '🟦', label: 'Order Block', points: baremes.value?.order_block ?? 25,
    description: 'Dernière bougie avant impulsion institutionnelle — zone de re-test.',
    regles: ['Volume élevé sur la bougie', 'Impulsion nette après', 'Prix revient dans la zone'] },
  { id: 'imb', icon: '⬜', label: 'Imbalance (FVG)', points: baremes.value?.imbalance ?? 15,
    description: 'Gap de prix sans retrace — liquidité non distribuée.',
    regles: [`1 zone alignée = ${Math.round((baremes.value?.imbalance ?? 15) / 2)}pts | 2+ zones = ${baremes.value?.imbalance ?? 15}pts`, 'Pas de retrace complète', 'Dans la direction du biais'] },
  { id: 'ifvg', icon: '🔷', label: 'IFVG', points: baremes.value?.ifvg ?? 20,
    description: 'Fair Value Gap avec break of structure — confluence SMC avancée.',
    regles: [`1 IFVG aligné = ${Math.round((baremes.value?.ifvg ?? 20) / 2)}pts | 2+ = ${baremes.value?.ifvg ?? 20}pts`, 'FVG validé + BOS dans la direction', 'Aligné avec l\'Order Block'] },
  { id: 'fib', icon: '🌀', label: 'Fibonacci', points: baremes.value?.fibonacci ?? 15,
    description: 'Niveaux de retrace institutionnels : 38.2%, 50%, 61.8%.',
    regles: ['38.2% — retrace légère', '50% — niveau équilibré', '61.8% — golden ratio'] },
])

const scoring = computed(() => [
  { label: 'Structure tendance',  detail: `Force 2/2 = +${baremes.value?.tendance ?? 25}pts | Force 1/2 = +${Math.round((baremes.value?.tendance ?? 25) / 2)}pts | Indécis = annulé`, maxPts: baremes.value?.tendance ?? 25, isDynamic: !!baremes.value },
  { label: 'Order Block actif',   detail: `Bougie OB alignée dans la zone = +${baremes.value?.order_block ?? 25}pts`, maxPts: baremes.value?.order_block ?? 25, isDynamic: !!baremes.value },
  { label: 'Imbalance ouverte',   detail: `2 zones = +${baremes.value?.imbalance ?? 15}pts | 1 zone = +${Math.round((baremes.value?.imbalance ?? 15) / 2)}pts`, maxPts: baremes.value?.imbalance ?? 15, isDynamic: !!baremes.value },
  { label: 'IFVG confirmé',       detail: `2+ IFVG = +${baremes.value?.ifvg ?? 20}pts | 1 IFVG = +${Math.round((baremes.value?.ifvg ?? 20) / 2)}pts`, maxPts: baremes.value?.ifvg ?? 20, isDynamic: !!baremes.value },
  { label: 'Fibonacci',           detail: `Prix sur 38.2/50/61.8% = +${baremes.value?.fibonacci ?? 15}pts`, maxPts: baremes.value?.fibonacci ?? 15, isDynamic: !!baremes.value },
  { label: 'Kill Zone (ICT)',     detail: 'Session active (LDN 07h–10h UTC, NY 13h–16h UTC) | Renforce conviction LLM', maxPts: undefined, isDynamic: false },
  { label: 'Liquidity Sweep',     detail: 'Chasse liquidités détectée | Transmis au LLM (contexte ICT)', maxPts: undefined, isDynamic: false },
])

const filtreRegles = [
  { icon: '🚫', couleur: 'text-red-400',    label: 'Score < 70 → rejet (seuil calibré dynamiquement par asset/TF/catégorie)' },
  { icon: '🚫', couleur: 'text-red-400',    label: 'Tendance indécise (Direction::Both) → signal annulé en amont du LLM' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'Kill Zone inactive → conviction LLM réduite' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'Sweep absent → contexte défavorable transmis au LLM' },
  { icon: '✅', couleur: 'text-green-400',  label: 'Peut affiner SL sur l\'OB identifié' },
]

const analyseOutputs = [
  'Performance par confluence (win rate par type)',
  'Confluences les plus fiables sur la période',
  '3 à 5 recommandations de réglage du score_min',
]
</script>

