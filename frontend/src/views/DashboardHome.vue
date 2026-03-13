<template>
  <div class="space-y-6">
    <!-- KPI Cards -->
    <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div class="glass-card p-5">
        <p class="label">Capital</p>
        <p class="kpi-value">{{ formatEur(capital) }}</p>
        <p class="label mt-1 text-xs">Portefeuille initial</p>
        <router-link to="/settings" class="text-xs text-emerald-400 hover:underline mt-1 block">Modifier →</router-link>
      </div>

      <div class="glass-card p-5">
        <p class="label">Status Backend</p>
        <p class="kpi-value" :class="backendOk ? 'text-emerald-400' : 'text-red-400'">
          {{ backendOk ? '🟢 Online' : '🔴 Offline' }}
        </p>
        <p class="label mt-1 text-xs">API localhost:8080</p>
      </div>

      <div class="glass-card p-5">
        <p class="label">BTC/USDT</p>
        <p class="kpi-value text-blue-400">
          {{ btcPrix ? formatUsd(btcPrix) : '—' }}
        </p>
        <p class="label mt-1 text-xs text-gray-500">{{ btcPrix ? 'Temps réel' : 'Chargement...' }}</p>
      </div>

      <div class="glass-card p-5">
        <p class="label">ETH/USDT</p>
        <p class="kpi-value text-purple-400">
          {{ ethPrix ? formatUsd(ethPrix) : '—' }}
        </p>
        <p class="label mt-1 text-xs text-gray-500">{{ ethPrix ? 'Temps réel' : 'Chargement...' }}</p>
      </div>
    </div>

    <!-- ML Prediction + System Status -->
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <!-- Prédiction IA -->
      <div class="glass-card p-5">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
          Prédiction IA — {{ settingsStore.assetActif }} {{ settingsStore.timeframeActif }}
        </h2>
        <div v-if="signalStore.prediction" class="space-y-3">
          <div class="flex items-center gap-3">
            <span
              class="text-2xl font-bold"
              :class="directionColor(signalStore.prediction.direction)"
            >
              {{ signalStore.prediction.direction.toUpperCase() }}
            </span>
            <span
              class="px-2 py-1 rounded text-xs font-medium"
              :class="signalStore.prediction.est_confiant ? 'bg-emerald-500/20 text-emerald-300' : 'bg-yellow-500/20 text-yellow-300'"
            >
              {{ signalStore.prediction.est_confiant ? '✓ Confiant' : '⚠ Indécis' }}
            </span>
          </div>
          <div class="w-full bg-gray-700 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all"
              :class="signalStore.prediction.est_confiant ? 'bg-emerald-500' : 'bg-yellow-500'"
              :style="{ width: `${(signalStore.prediction.confiance * 100).toFixed(0)}%` }"
            />
          </div>
          <p class="text-xs text-gray-400">
            Confiance: {{ (signalStore.prediction.confiance * 100).toFixed(1) }}%
            — Modèle: {{ signalStore.prediction.modele_pret ? '✓ Entraîné' : '⏳ Non entraîné' }}
          </p>
          <button
            class="mt-3 w-full py-2 px-3 rounded-lg text-xs font-semibold transition-all"
            :class="entraineEnCours
              ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
              : 'bg-blue-600 hover:bg-blue-500 text-white'"
            :disabled="entraineEnCours"
            @click="lancerEntrainement"
          >
            {{ entraineEnCours ? '⏳ Entraînement...' : '🧠 Entraîner RF + LSTM' }}
          </button>
        </div>
        <div v-else class="text-gray-500 text-sm">Chargement prédiction...</div>
      </div>

      <!-- Statut système -->
      <div class="glass-card p-5">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
          Statut système
        </h2>
        <div class="space-y-3">
          <div class="flex justify-between items-center">
            <span class="text-gray-300 text-sm">Backend API</span>
            <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm">
              {{ backendOk ? '🟢 Online' : '🔴 Offline' }}
            </span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-gray-300 text-sm">ML Engine</span>
            <span
              :class="signalStore.prediction?.modele_pret ? 'text-emerald-400' : 'text-yellow-400'"
              class="text-sm"
            >
              {{ signalStore.prediction?.modele_pret ? '🟢 Prêt' : '🟡 Non entraîné' }}
            </span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-gray-300 text-sm">Binance Feed</span>
            <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-sm">
              {{ btcPrix ? '🟢 Connecté' : '🔴 Offline' }}
            </span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-gray-300 text-sm">Base de données</span>
            <span class="text-emerald-400 text-sm">🟢 SQLite</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Score SMC -->
    <SmcScoreCard
      :score-smc="signalStore.scoreSmc"
      :asset="settingsStore.assetActif"
      :timeframe="settingsStore.timeframeActif"
    />

    <!-- Métriques performance (backtest BTC) -->
    <div v-if="metriques" class="grid grid-cols-2 gap-4 lg:grid-cols-4">
      <div class="glass-card p-4"><p class="label">Win Rate</p><p class="kpi-value text-emerald-400">{{ metriques.win_rate.toFixed(1) }}%</p></div>
      <div class="glass-card p-4"><p class="label">ROI Backtest</p><p class="kpi-value" :class="metriques.roi_pct >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ metriques.roi_pct.toFixed(1) }}%</p></div>
      <div class="glass-card p-4"><p class="label">Total Trades</p><p class="kpi-value">{{ metriques.total_trades }}</p></div>
      <div class="glass-card p-4"><p class="label">Max Drawdown</p><p class="kpi-value text-red-400">{{ metriques.max_drawdown_pct.toFixed(1) }}%</p></div>
    </div>

    <!-- Derniers signaux -->
    <div class="glass-card p-5">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
        Derniers signaux
      </h2>
      <div v-if="signalStore.chargement" class="text-gray-500 text-sm text-center py-4">
        Chargement...
      </div>
      <div v-else-if="signalStore.signaux.length === 0" class="text-gray-500 text-sm text-center py-6">
        Aucun signal enregistré — lancez une stratégie pour commencer
      </div>
      <div v-else class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-gray-500 text-xs uppercase border-b border-white/10">
              <th class="pb-2 text-left">Asset</th>
              <th class="pb-2 text-left">TF</th>
              <th class="pb-2 text-left">Direction</th>
              <th class="pb-2 text-right">Score</th>
              <th class="pb-2 text-right">Entrée</th>
              <th class="pb-2 text-left">Stratégie</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="signal in signalStore.signaux.slice(0, 8)"
              :key="signal.id"
              class="border-b border-white/5 hover:bg-white/5"
            >
              <td class="py-2 font-medium">{{ signal.asset }}</td>
              <td class="py-2 text-gray-400">{{ signal.timeframe }}</td>
              <td class="py-2">
                <span
                  class="px-2 py-0.5 rounded text-xs"
                  :class="badgeDirection(signal.direction)"
                >
                  {{ signal.direction }}
                </span>
              </td>
              <td class="py-2 text-right">{{ signal.score.toFixed(1) }}</td>
              <td class="py-2 text-right font-mono">{{ formatUsd(signal.prix_entree) }}</td>
              <td class="py-2 text-gray-400 text-xs">{{ signal.strategie }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useMarketStore } from '@/stores/market.store'
import { useSignalStore } from '@/stores/signal.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { apiService } from '@/services/api.service'
import SmcScoreCard from '@/components/common/SmcScoreCard.vue'
import type { BacktestResults } from '@/services/api.service'

const marketStore = useMarketStore()
const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const alerteStore = useAlerteStore()

const capital = computed(() => settingsStore.capitalDepart)
const backendOk = ref(false)
const entraineEnCours = ref(false)
const metriques = ref<BacktestResults | null>(null)

const btcPrix = computed(() => {
  const bougies = marketStore.getBougies('BTC', 'M15')
  return bougies.length > 0 ? bougies[bougies.length - 1].close : null
})

const ethPrix = computed(() => {
  const bougies = marketStore.getBougies('ETH', 'M15')
  return bougies.length > 0 ? bougies[bougies.length - 1].close : null
})

async function lancerEntrainement() {
  entraineEnCours.value = true
  alerteStore.afficher('Entraînement ML en cours (RF + LSTM)...', 'info', 120000)
  try {
    const res = await apiService.entrainerML(settingsStore.assetActif, settingsStore.timeframeActif, 1000)
    alerteStore.afficherSucces(`✅ ${res.message}`)
    await signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif)
  } catch (err: unknown) {
    alerteStore.afficherErreur(`Entraînement échoué: ${(err as Error).message}`)
  } finally {
    entraineEnCours.value = false
  }
}

function formatUsd(v: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 }).format(v)
}

function formatEur(v: number): string {
  return new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'EUR' }).format(v)
}

function directionColor(dir: string): string {
  if (dir.toLowerCase().includes('long')) return 'text-emerald-400'
  if (dir.toLowerCase().includes('short')) return 'text-red-400'
  return 'text-yellow-400'
}

function badgeDirection(dir: string): string {
  if (dir === 'Long') return 'bg-emerald-500/20 text-emerald-300'
  if (dir === 'Short') return 'bg-red-500/20 text-red-300'
  return 'bg-yellow-500/20 text-yellow-300'
}

onMounted(async () => {
  // Vérification santé backend
  try {
    await apiService.healthCheck()
    backendOk.value = true
  } catch {
    backendOk.value = false
  }

  // Chargement parallèle des données
  await Promise.allSettled([
    marketStore.chargerBougies('BTC', 'M15', 100),
    marketStore.chargerBougies('ETH', 'M15', 100),
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
    signalStore.chargerScoreSmc(settingsStore.assetActif, settingsStore.timeframeActif),
  ])
})

// Rafraîchir la prédiction quand l'actif ou le timeframe change dans ChartsView
watch(
  () => `${settingsStore.assetActif}_${settingsStore.timeframeActif}`,
  (_, ancien) => {
    if (ancien !== undefined) {
      signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif)
      signalStore.chargerScoreSmc(settingsStore.assetActif, settingsStore.timeframeActif)
    }
  }
)
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
.label {
  @apply text-xs text-gray-400 font-medium;
}
.kpi-value {
  @apply text-2xl font-bold text-white mt-1;
}
</style>
