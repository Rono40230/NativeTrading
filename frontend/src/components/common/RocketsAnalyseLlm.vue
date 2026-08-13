<template>
  <div class="flex flex-col gap-4">
    <!-- Actions -->
    <div class="flex items-center justify-between flex-shrink-0">
      <div class="text-xs text-gray-500">
        <span v-if="analyse">Dernière analyse : <span class="text-gray-300">{{ dateAnalyse }}</span> sur <span class="text-white font-bold">{{ analyse.nb_trades }}</span> trades</span>
        <span v-else class="italic">Aucune analyse disponible</span>
      </div>
      <button
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all"
        :class="chargement ? 'bg-white/10 text-gray-400 cursor-not-allowed' : 'bg-blue-600/30 text-blue-300 hover:bg-blue-600/50 border border-blue-500/30'"
        :disabled="chargement"
        @click="relancerAnalyse"
      >
        <span v-if="chargement">⏳ Analyse en cours…</span>
        <span v-else>🔄 Relancer l'analyse</span>
      </button>
    </div>

    <!-- Erreur -->
    <div v-if="erreur" class="rounded-lg bg-red-900/30 border border-red-500/30 px-4 py-3 text-xs text-red-300">
      {{ erreur }}
    </div>

    <!-- Pas de données -->
    <div v-else-if="!analyse" class="rounded-lg bg-white/5 border border-white/10 px-4 py-6 text-center text-xs text-gray-500 italic">
      Aucune analyse LLM disponible. Cliquez sur "Relancer l'analyse" pour générer une première analyse.<br>
      <span class="text-gray-600 mt-1 block">Minimum {{ MIN_TRADES }} trades clôturés requis.</span>
    </div>

    <template v-else>
      <!-- Synthèse -->
      <div class="rounded-lg border border-blue-500/20 bg-blue-950/30 px-4 py-3">
        <div class="text-[10px] text-blue-400 font-semibold uppercase tracking-widest mb-1.5">Synthèse</div>
        <p class="text-xs text-gray-200 leading-relaxed">{{ analyse.synthese }}</p>
      </div>

      <!-- Meilleur / Pire setup -->
      <div v-if="analyse.meilleur_setup || analyse.pire_setup" class="grid grid-cols-2 gap-3">
        <div v-if="analyse.meilleur_setup" class="rounded-lg border border-emerald-500/20 bg-emerald-950/20 px-3 py-2.5">
          <div class="text-[9px] text-emerald-400 font-semibold uppercase tracking-widest mb-1">✅ Meilleur setup</div>
          <p class="text-xs text-gray-300 leading-relaxed">{{ analyse.meilleur_setup }}</p>
        </div>
        <div v-if="analyse.pire_setup" class="rounded-lg border border-red-500/20 bg-red-950/20 px-3 py-2.5">
          <div class="text-[9px] text-red-400 font-semibold uppercase tracking-widest mb-1">❌ À éviter</div>
          <p class="text-xs text-gray-300 leading-relaxed">{{ analyse.pire_setup }}</p>
        </div>
      </div>

      <!-- Recommandations -->
      <div>
        <h3 class="text-[10px] text-gray-500 font-semibold uppercase tracking-widest mb-2">
          Recommandations ({{ recommandations.length }})
        </h3>
        <div class="flex flex-col gap-2">
          <div
            v-for="(r, i) in recommandationsTri"
            :key="i"
            class="rounded-lg border px-3 py-2.5 flex gap-3 items-start"
            :class="classeReco(r.priorite)"
          >
            <span class="shrink-0 text-base mt-0.5">{{ iconeType(r.type) }}</span>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-0.5">
                <span class="text-[9px] font-bold uppercase tracking-widest px-1.5 py-0.5 rounded-full" :class="classePriorite(r.priorite)">
                  {{ r.priorite }}
                </span>
                <span class="text-[9px] text-gray-500">{{ labelType(r.type) }}</span>
              </div>
              <p class="text-xs text-gray-200 leading-relaxed">{{ r.description }}</p>
              <p v-if="r.impact_estime" class="text-[10px] text-gray-400 mt-1 italic">Impact estimé : {{ r.impact_estime }}</p>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { RocketAnalyseLlm, RocketRecommandation } from '@/services/api.types'
import { formatParis } from '@/utils/date'
import { apiService } from '@/services/api.service'

const MIN_TRADES = 5

const analyse = ref<RocketAnalyseLlm | null>(null)
const chargement = ref(false)
const erreur = ref<string | null>(null)

onMounted(async () => {
  analyse.value = await apiService.getDerniereAnalyseLlmRockets()
})

async function relancerAnalyse() {
  chargement.value = true
  erreur.value = null
  try {
    analyse.value = await apiService.lancerAnalyseLlmRockets()
  } catch (e: any) {
    erreur.value = e?.response?.data?.error ?? e?.message ?? 'Erreur lors de l\'analyse'
  } finally {
    chargement.value = false
  }
}

const recommandations = computed<RocketRecommandation[]>(() => {
  if (!analyse.value) return []
  try {
    return JSON.parse(analyse.value.recommandations) as RocketRecommandation[]
  } catch {
    return []
  }
})

const recommandationsTri = computed(() =>
  [...recommandations.value].sort((a, b) => {
    const ord = { haute: 0, moyenne: 1, faible: 2 }
    return (ord[a.priorite] ?? 3) - (ord[b.priorite] ?? 3)
  })
)

const dateAnalyse = computed(() => {
  if (!analyse.value) return ''
  return formatParis(new Date(analyse.value.cree_le), {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  })
})

function classeReco(priorite: string) {
  if (priorite === 'haute') return 'border-red-500/25 bg-red-950/20'
  if (priorite === 'moyenne') return 'border-yellow-500/25 bg-yellow-950/15'
  return 'border-white/10 bg-white/[0.03]'
}

function classePriorite(priorite: string) {
  if (priorite === 'haute') return 'bg-red-900/60 text-red-300'
  if (priorite === 'moyenne') return 'bg-yellow-900/60 text-yellow-300'
  return 'bg-white/10 text-gray-400'
}

function iconeType(type: string) {
  const m: Record<string, string> = {
    seuil_score: '🎯', filtre_phase: '🔍', trailing_stop: '📍',
    filtre_rsi: '📊', filtre_volume: '📈', mode_entree: '🚪', autre: '💡',
  }
  return m[type] ?? '💡'
}

function labelType(type: string) {
  const m: Record<string, string> = {
    seuil_score: 'Seuil score', filtre_phase: 'Filtre phase',
    trailing_stop: 'Trailing stop', filtre_rsi: 'Filtre RSI',
    filtre_volume: 'Filtre volume', mode_entree: 'Mode d\'entrée', autre: 'Autre',
  }
  return m[type] ?? type
}
</script>
