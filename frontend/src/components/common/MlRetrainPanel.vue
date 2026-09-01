<template>
  <div class="space-y-4 flex flex-col h-full">
    <!-- Barre de progression (uniquement pendant l'entraînement) -->
    <div v-if="store.retrainState?.en_cours" class="space-y-2">
      <div class="flex justify-between text-xs text-white">
        <span class="font-medium text-white">
          {{ store.retrainState.nb_combinaisons_total > 0
            ? `${store.retrainState.nb_combinaisons_done} / ${store.retrainState.nb_combinaisons_total} combinaisons`
            : 'Initialisation…' }}
        </span>
        <span>{{ elapsed }}s écoulées</span>
      </div>
      <div class="w-full bg-gray-700 rounded-full h-2.5 overflow-hidden">
        <div
          class="h-2.5 rounded-full bg-blue-500 transition-all duration-700"
          :style="{ width: progres + '%' }"
        />
      </div>
      <div class="flex items-center justify-between text-xs">
        <span class="text-blue-400 font-mono truncate max-w-xs">
          <template v-if="store.retrainState.combinaison_en_cours">
            ⚙ {{ store.retrainState.combinaison_en_cours }}
          </template>
        </span>
        <span class="text-white font-semibold tabular-nums">{{ progres }}%</span>
      </div>
    </div>

    <!-- DASHBOARD PERSISTANT DES MODÈLES ML -->
    <div class="space-y-4">

      <!-- Métriques globales (Performance & Santé) -->
      <div v-if="store.retrainState && store.retrainState.accuracy_avant > 0" 
           class="flex items-center bg-black/20 border border-white/10 rounded-lg p-4 justify-around mb-6">
        <div class="text-center">
          <p class="text-[10px] text-white mb-1 uppercase tracking-wide">Précision Avant</p>
          <p class="text-base font-bold text-white tabular-nums">{{ (store.retrainState.accuracy_avant * 100).toFixed(1) }}%</p>
        </div>
        <span class="text-white text-lg mx-2">→</span>
        <div class="text-center">
          <p class="text-[10px] text-white mb-1 uppercase tracking-wide">Précision Après</p>
          <p class="text-base font-bold tabular-nums"
            :class="(store.retrainState.accuracy_apres ?? 0) >= store.retrainState.accuracy_avant ? 'text-emerald-400' : 'text-red-400'">
            {{ ((store.retrainState.accuracy_apres ?? 0) * 100).toFixed(1) }}%
          </p>
        </div>
        <div class="w-px h-10 bg-white/10 mx-4"></div>
        <div class="text-center flex flex-col items-center">
          <p class="text-xs text-white mb-1 uppercase tracking-wide">État du Pipeline</p>
          <div class="flex items-center gap-2">
            <div class="w-2.5 h-2.5 rounded-full" :class="store.retrainState.overfitting || store.retrainState.rolled_back ? 'bg-red-500 animate-pulse' : 'bg-emerald-500'"></div>
            <p class="text-sm font-bold" :class="store.retrainState.overfitting || store.retrainState.rolled_back ? 'text-red-400' : 'text-emerald-400'">
              {{ store.retrainState.rolled_back ? 'ROLLBACK / OVERFIT' : 'STABLE & SAIN' }}
            </p>
          </div>
          <p v-if="store.retrainState.gap_train_wf !== null" class="text-[10px] text-white mt-1">
            Gap Validation: {{ (store.retrainState.gap_train_wf * 100).toFixed(1) }}%
          </p>
        </div>
      </div>

      <!-- Informations par stratégie (Top Features) -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-2 flex-1">
        <div v-for="strat in strategiesData" :key="strat.id" class="bg-black/20 rounded-lg p-2 relative overflow-hidden border border-white/10">
          <!-- Header -->
          <div class="flex items-center justify-between border-b border-white/10 pb-1 mb-2">
            <div class="flex items-center gap-2">
              <span class="text-lg">{{ strat.icon }}</span>
              <h4 class="font-bold text-sm text-white">{{ strat.label }}</h4>
            </div>
            <!-- Pastille de présence du modèle -->
            <div class="flex items-center gap-1.5" :title="strat.features.length > 0 ? 'Modèle entraîné en base' : 'Modèle vierge/par défaut'">
              <div class="w-2 h-2 rounded-full" :class="strat.features.length > 0 ? 'bg-blue-400' : 'bg-gray-600'"></div>
            </div>
          </div>

          <!-- Top Variables -->
          <div class="space-y-1">
            <h5 class="text-[9px] uppercase font-bold text-white flex justify-between">
              <span>Top 3 Prédictif</span>
              <span>Poids</span>
            </h5>
            
            <template v-if="strat.features.length > 0">
              <div v-for="(f, idx) in strat.features.slice(0, 3)" :key="f.feature_idx" class="space-y-0.5">
                <div class="flex justify-between items-center text-[10px]">
                  <span class="text-white font-medium truncate">{{ traduireFeature(f.feature_nom) }}</span>
                  <span class="text-white tabular-nums">{{ (f.importance * 100).toFixed(1) }}%</span>
                </div>
                <div class="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                  <div class="h-full bg-blue-500/80 rounded-full transition-all"
                       :style="{ width: Math.round((f.importance / strat.max) * 100) + '%' }"></div>
                </div>
              </div>
            </template>
            <div v-else class="text-center text-xs text-white py-6">
              Pas de données ML.<br>En attente du premier entraînement.
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useMlInsightsStore } from '@/stores/mlInsights.store'
import { apiService } from '@/services/api.service'

const store = useMlInsightsStore()

interface FeatureImportance {
  feature_idx: number
  feature_nom: string
  importance: number
}

// ── Dictionnaire de traduction des features ML en Bon Français ───────
const dictionnaireFeatures: Record<string, string> = {
  // OHLCV
  "open_rel": "Ouverture de bougie",
  "high_rel": "Mèche Haute (Distance)",
  "low_rel": "Mèche Basse (Distance)",
  "rendement_1": "Variation immédiate",
  "volume_rel": "Anomalie de Volume",
  // Spreads
  "range_rel": "Ampleur totale",
  "corps_rel": "Taille du corps",
  "meche_haute": "Rejet vendeur",
  "meche_basse": "Rejet acheteur",
  "rendement_5": "Variation sur 5 bougies",
  // EMA
  "ema9_rel": "Écart Moyenne Rapide",
  "ema21_rel": "Écart Moyenne Normale",
  "ema50_rel": "Écart Moyenne Lente",
  "spread_ema9_21": "Spread EMA 9/21",
  "spread_ema21_50": "Spread EMA 21/50",
  "pente_ema21": "Tendance (Pente)",
  // RSI
  "rsi14": "Pression RSI",
  "rsi_surachat": "Zone Surachat RSI",
  "rsi_survente": "Zone Survente RSI",
  // ATR
  "atr14_rel": "Volatilité (ATR)",
  "atr_vs_moyenne": "Volatilité anormale",
  "atr_extreme_150pct": "Pic Volatilité Extrême",
  "atr_moyen_rel": "Volatilité Moyenne",
  // MACD
  "macd_ligne": "MACD (Force)",
  "macd_signal": "MACD (Signal)",
  "macd_histogramme": "MACD (Histogramme)",
  "macd_croise_haut": "Croisement Haussier",
  "macd_croise_bas": "Croisement Baissier",
  // Bollinger
  "bb_largeur": "Compression Bollinger",
  "bb_position": "Position dans le Range",
  "bb_au_dessus_sup": "Cassure Haussière BB",
  "bb_en_dessous_inf": "Cassure Baissière BB",
  "bb_pct_b": "Bollinger %B",
  // Momentum
  "rdt_1": "Élan (1 bougie)",
  "rdt_2": "Élan (2 bougies)",
  "rdt_3": "Élan (3 bougies)",
  "rdt_4": "Élan (4 bougies)",
  "rdt_5": "Élan (5 bougies)",
  "momentum_10": "Momentum (10 min)",
  "momentum_20": "Momentum (20 min)",
  "momentum_30": "Momentum (30 min)",
  "momentum_50": "Momentum (50 min)",
  // Volume history
  "vol_1": "Pression Vol (T-1)",
  "vol_2": "Pression Vol (T-2)",
  "vol_3": "Pression Vol (T-3)",
  "vol_4": "Pression Vol (T-4)",
  "vol_5": "Pression Vol (T-5)",
  // Patterns
  "ratio_corps_range": "Compression (Doji)",
  "trois_haussiers": "3 Soldats Blancs",
  "trois_baissiers": "3 Corbeaux Noirs",
  "englobante_haussiere": "Englobante Haussière",
  "momentum_ema9_50": "Momentum EMA 9/50",
  // SMC spécifiques
  "smc_tendance": "Direct. Tendance (SMC)",
  "smc_order_block": "Ordres Institutionnels (OB)",
  "smc_ifvg": "Inversion FVG",
  "smc_fibonacci": "Retracement Fibonacci",
  "smc_imbalance": "Imbalance (Déséquilibre)",
  "smc_kill_zone": "Séances Horaires (KZ)",
  "smc_sweep": "Chasse aux Liquidités",
  // Straddle spécifiques
  "ratio_atr": "Puissance du Breakout",
  "straddle_categorie": "Catégorie de Breakout",
  "straddle_session": "Volume selon Session",
  // LLM
  "score_llm": "Conviction IA",
  "score_min": "Constante de Filtre"
}

function traduireFeature(nom: string): string {
  return dictionnaireFeatures[nom] || nom
}
// ───────────────────────────────────────────────────────────────────

interface StrategyData {
  id: string
  label: string
  icon: string
  features: FeatureImportance[]
  max: number
}

const strategiesData = ref<StrategyData[]>([
  { id: 'rockets', label: 'Rockets', icon: '🚀', features: [], max: 1 },
  { id: 'smc', label: 'SMC Directionnel', icon: '📊', features: [], max: 1 },
  { id: 'straddle', label: 'Straddle', icon: '⚡', features: [], max: 1 },
])

async function chargerTopFeatures() {
  for (const strat of strategiesData.value) {
    try {
      strat.features = await apiService.getMlFeatureImportance(strat.id)
      strat.max = strat.features.length > 0 && strat.features[0].importance > 0 ? strat.features[0].importance : 1
    } catch {
      // Silencieux : les importances n'existent pas encore
    }
  }
}

const elapsed = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

// Progression basée sur le compteur réel renvoyé par le backend (0→100%)
// Fallback à 0% si le backend n'a pas encore communiqué le total (démarrage)
const progres = computed(() => {
  const s = store.retrainState
  if (!s?.en_cours) return 100
  const total = s.nb_combinaisons_total
  if (!total) return 0
  return Math.round((s.nb_combinaisons_done / total) * 100)
})

// Démarre/arrête le timer selon l'état du job
watch(
  () => store.retrainState?.en_cours,
  (enCours) => {
    if (enCours) {
      elapsed.value = store.retrainState?.demarre_le
        ? Math.max(0, Math.floor(Date.now() / 1000 - store.retrainState.demarre_le))
        : 0
      timer = setInterval(() => { elapsed.value++ }, 1000)
    } else {
      if (timer) { clearInterval(timer); timer = null }
      // Rafraîchir les features quand l'entraînement se termine
      if (store.retrainState?.job_id && store.retrainState?.termine_le) {
        chargerTopFeatures()
      }
    }
  },
  { immediate: true }
)

onMounted(() => { chargerTopFeatures() })
onUnmounted(() => { if (timer) clearInterval(timer) })
</script>
