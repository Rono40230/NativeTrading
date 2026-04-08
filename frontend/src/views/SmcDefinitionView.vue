<template>
  <div class="flex flex-col gap-4 p-6 h-full w-full">

    <!-- Header -->
    <div class="flex items-baseline gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🎯 Stratégie SMC</h1>
      <span class="text-gray-500 text-base">Définition, confluences et rôle de l'IA</span>
    </div>

    <!-- Ligne 1 : Concept + Paramètres actifs -->
    <div class="grid grid-cols-[3fr_2fr] gap-4 items-stretch shrink-0">

      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest mb-3">Concept</div>
        <p class="text-gray-300 text-base leading-relaxed">
          SMC Directionnel trade la <span class="text-white font-medium">confluence Smart Money</span> :
          alignement de la structure de marché, des Order Blocks, des imbalances et du Fibonacci pour
          identifier les zones d'intérêt institutionnel. Un signal n'est déclenché que si le
          <span class="text-white font-medium">score de confluence atteint ≥ 70/100</span>.
          La position est unique et directionnelle — pyramidale sur 3 TP.
        </p>
      </div>

      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5">
        <div class="mb-4">
          <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest">Paramètres actifs</div>
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

      <!-- Col 1 : Confluences -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest mb-4">Les 5 confluences SMC</div>
        <div class="flex flex-col gap-4 flex-1 justify-between">
          <div v-for="c in confluences" :key="c.id" class="rounded-lg bg-black/20 border border-white/5 px-4 py-4">
            <div class="flex items-center gap-2 mb-2">
              <span class="text-xl leading-none">{{ c.icon }}</span>
              <span class="text-white font-semibold text-base">{{ c.label }}</span>
              <span class="ml-auto text-xs font-bold text-blue-300">+{{ c.points }}pts</span>
            </div>
            <p class="text-gray-500 text-sm mb-2.5">{{ c.description }}</p>
            <div class="space-y-1.5">
              <div v-for="r in c.regles" :key="r"
                class="text-sm text-gray-300 pl-3 border-l-2 border-blue-500/40 leading-snug">{{ r }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 2 : Scoring -->
      <div class="rounded-xl border border-white/10 bg-white/5 px-6 py-5 flex flex-col overflow-y-auto">
        <div class="text-xs font-semibold text-blue-400 uppercase tracking-widest mb-4">Système de scoring</div>
        <div class="flex flex-col gap-3 flex-1 justify-between">
          <div v-for="s in scoring" :key="s.label" class="rounded-lg bg-black/20 border border-white/5 px-4 py-3">
            <div class="text-base font-semibold text-white mb-1.5">{{ s.label }}</div>
            <div class="text-sm text-gray-400 leading-relaxed">{{ s.detail }}</div>
          </div>
          <div class="rounded-lg bg-black/30 border border-blue-500/20 px-4 py-3">
            <div class="text-xs text-gray-500 mb-2">Seuil de déclenchement</div>
            <div class="flex gap-5">
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-green-400 font-bold">≥ 70</span>
                <span class="text-gray-400">Signal ✅</span>
              </div>
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-red-400 font-bold">&lt; 70</span>
                <span class="text-gray-400">Ignoré 🚫</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Col 3 : Rôle de l'IA -->
      <div class="flex flex-col gap-4 overflow-y-auto min-h-0">

        <div class="rounded-xl border border-blue-500/30 bg-blue-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl">⚡</span>
            <span class="text-white font-semibold text-base">Filtre IA temps réel</span>
            <span class="ml-auto text-sm text-blue-400 bg-blue-500/10 px-2.5 py-0.5 rounded-full">Avant chaque signal</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-4">
            Valide ou rejette chaque signal SMC candidat. Retourne conviction 0–100 + raison.
          </p>
          <div class="space-y-2 mb-4">
            <div v-for="r in filtreRegles" :key="r.label" class="flex items-center gap-2 text-sm">
              <span :class="r.couleur" class="shrink-0">{{ r.icon }}</span>
              <span class="text-gray-300">{{ r.label }}</span>
            </div>
          </div>
        </div>

        <div class="rounded-xl border border-cyan-500/30 bg-cyan-500/5 px-6 py-5 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl">🤖</span>
            <span class="text-white font-semibold text-base">Génération signal JSON</span>
            <span class="ml-auto text-sm text-cyan-400 bg-cyan-500/10 px-2.5 py-0.5 rounded-full">POST /ia/signal</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed">
            Génère un signal structuré complet : direction, SL, TP pyramidal (3 niveaux), confluences détectées.
          </p>
        </div>

        <div class="rounded-xl border border-purple-500/30 bg-purple-500/5 px-6 py-5 flex-1 flex flex-col">
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xl">📊</span>
            <span class="text-white font-semibold text-base">Analyse stratégique IA</span>
            <span class="ml-auto text-sm text-purple-400 bg-purple-500/10 px-2.5 py-0.5 rounded-full">Sur demande</span>
          </div>
          <p class="text-gray-400 text-sm leading-relaxed mb-3">
            Analyse les trades SMC clôturés pour évaluer la performance des confluences et recommander des ajustements.
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
  try { await strategyStore.charger(); params.value = { ...strategyStore.smcRaw } } catch { /* silencieux */ }
})

const paramCards = computed(() => params.value ? [
  { label: 'Score min',    value: params.value.score_min },
  { label: 'ATR période',  value: params.value.atr_periode },
  { label: 'TP1',          value: `${params.value.atr_tp1}×` },
  { label: 'TP2',          value: `${params.value.atr_tp2}×` },
  { label: 'TP3',          value: `${params.value.atr_tp3}×` },
  { label: 'SL',           value: `${params.value.atr_sl}×` },
  { label: 'Horizon',      value: `${params.value.horizon_bougies}b` },
] : [])

const confluences = [
  { id: 'tendance', icon: '📈', label: 'Structure de tendance', points: 20,
    description: 'Biais directionnel basé sur la succession de sommets et creux.',
    regles: ['Haussier : HH + HL', 'Baissier : LH + LL', 'Neutre → score réduit'] },
  { id: 'ob', icon: '🟦', label: 'Order Block', points: 25,
    description: 'Dernière bougie avant impulsion institutionnelle — zone de re-test.',
    regles: ['Volume élevé sur la bougie', 'Impulsion nette après', 'Prix revient dans la zone'] },
  { id: 'imb', icon: '⬜', label: 'Imbalance (FVG)', points: 20,
    description: 'Gap de prix ≥ 3 pips sans retrace — liquidité non distribuée.',
    regles: ['Gap ≥ 3 pips', 'Pas de retrace complète', 'Dans la direction du biais'] },
  { id: 'ifvg', icon: '🔷', label: 'IFVG', points: 20,
    description: 'Fair Value Gap avec break of structure — confluence SMC avancée.',
    regles: ['FVG validé + BOS', 'Aligné avec l\'Order Block', 'Timeframe cohérent'] },
  { id: 'fib', icon: '🌀', label: 'Fibonacci', points: 15,
    description: 'Niveaux de retrace institutionnels : 38.2%, 50%, 61.8%.',
    regles: ['38.2% — retrace légère', '50% — niveau équilibré', '61.8% — golden ratio'] },
]

const scoring = [
  { label: 'Structure tendance',  detail: 'Haussier/baissier clair = +20pts | Neutre = 0pts' },
  { label: 'Order Block actif',    detail: 'Bougie OB présente dans la zone = +25pts' },
  { label: 'Imbalance ouverte',    detail: 'Gap ≥ 3 pips non retracé = +20pts' },
  { label: 'IFVG confirmé',        detail: 'FVG + BOS alignés = +20pts' },
  { label: 'Niveau Fibonacci',     detail: 'Prix sur 38.2/50/61.8% = +15pts' },
]

const filtreRegles = [
  { icon: '🚫', couleur: 'text-red-400',    label: 'Score < 70 → rejet systématique' },
  { icon: '🚫', couleur: 'text-red-400',    label: 'Tendance neutre sans OB → invalider' },
  { icon: '⚠️', couleur: 'text-yellow-400', label: 'TP/SL divergents du scoring → −15 conviction' },
  { icon: '✅', couleur: 'text-green-400',  label: 'Peut affiner SL sur l\'OB identifié' },
]

const analyseOutputs = [
  'Performance par confluence (win rate par type)',
  'Confluences les plus fiables sur la période',
  '3 à 5 recommandations de réglage du score_min',
]
</script>

