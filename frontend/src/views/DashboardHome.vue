<template>
  <div class="space-y-6">
    <!-- Ligne 1 : Capital + Statut Système + bande de prix -->
    <div class="flex flex-col gap-3">
      <!-- Capital et Statut sur la même ligne -->
      <div class="flex gap-4 items-stretch">
        <!-- Capital -->
        <div class="glass-card p-5 shrink-0">
          <p class="label">Capital</p>
          <p class="kpi-value">{{ formatUsd(capital) }}</p>
          <p class="label mt-1 text-xs">Portefeuille initial</p>
          <router-link to="/settings" class="text-xs text-emerald-400 hover:underline mt-1 block">Modifier →</router-link>
        </div>
        <!-- Statut Système -->
        <div class="glass-card p-5 flex-1">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Statut système</h2>
          <div class="grid grid-cols-3 gap-2">
            <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
              <span class="text-gray-500 text-[10px] uppercase tracking-wider">Backend API</span>
              <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">{{ backendOk ? '🟢 Online' : '🔴 Offline' }}</span>
            </div>
            <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
              <span class="text-gray-500 text-[10px] uppercase tracking-wider">Binance Feed</span>
              <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">{{ btcPrix ? '🟢 Connecté' : '🔴 Offline' }}</span>
            </div>
            <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
              <span class="text-gray-500 text-[10px] uppercase tracking-wider">IB Gateway</span>
              <span v-if="ibGatewayOk === null" class="text-gray-500 text-sm font-semibold animate-pulse">⏳ Vérif…</span>
              <span v-else :class="ibGatewayOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">{{ ibGatewayOk ? '🟢 Connecté' : '🔴 Déconnecté' }}</span>
            </div>
            <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
              <span class="text-gray-500 text-[10px] uppercase tracking-wider">ML Engine</span>
              <span :class="signalStore.prediction?.modele_pret ? 'text-emerald-400' : 'text-yellow-400'" class="text-sm font-semibold">
                {{ signalStore.prediction?.modele_pret ? '🟢 Prêt' : '🟡 Non entraîné' }}
              </span>
            </div>
            <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
              <span class="text-gray-500 text-[10px] uppercase tracking-wider">Base de données</span>
              <span class="text-emerald-400 text-sm font-semibold">🟢 SQLite</span>
            </div>
          </div>
        </div>
      </div>
      <!-- Bande de prix actifs -->
      <div class="flex gap-2 flex-wrap">
        <div v-for="a in assetsDisplay" :key="a.id" class="glass-card px-3 py-2.5 flex flex-col items-center flex-1 min-w-[80px]">
          <span class="text-[10px] text-gray-400 font-medium tracking-wide">{{ a.id }}</span>
          <span v-if="a.chargement" class="text-xs text-gray-500 mt-1 animate-pulse">…</span>
          <template v-else>
            <span class="text-sm font-bold mt-0.5">{{ a.prix !== null ? formatPrixAsset(a.prix) : '—' }}</span>
            <span v-if="a.variation !== null" class="text-[10px] font-medium mt-0.5" :class="a.variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ a.variation >= 0 ? '+' : '' }}{{ a.variation.toFixed(2) }}%
            </span>
          </template>
        </div>
      </div>
    </div>

    <!-- Horloges sessions de marché -->
    <MarketClocks />

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
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useSignalStore } from '@/stores/signal.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { useMarketStore } from '@/stores/market.store'
import { apiService } from '@/services/api.service'
import type { BacktestResults } from '@/services/api.service'
import MarketClocks from '@/components/common/MarketClocks.vue'

const signalStore = useSignalStore()
const settingsStore = useSettingsStore()
const marketStore = useMarketStore()
const alerteStore = useAlerteStore()

const capital = computed(() => settingsStore.capitalDepart)
const backendOk = ref(false)
const ibGatewayOk = ref<boolean | null>(null)
const metriques = ref<BacktestResults | null>(null)

const assetsAvecPrix = ref<{ id: string; prix: number | null; variation: number | null; chargement: boolean }[]>([])
// Prix temps réel pour BTC/ETH depuis le WS Binance (remplace le prix REST si disponible)
const assetsDisplay = computed(() =>
  assetsAvecPrix.value.map(a => ({
    ...a,
    prix: marketStore.prixLive[a.id] ?? a.prix,
    variation: marketStore.variationLive[a.id] !== undefined ? marketStore.variationLive[a.id] : a.variation,
    chargement: a.chargement && marketStore.prixLive[a.id] === undefined,
  }))
)
const btcPrix = computed(() => marketStore.prixLive['BTC'] ?? assetsAvecPrix.value.find(a => a.id === 'BTC')?.prix ?? null)
let intervalPrix: ReturnType<typeof setInterval> | null = null

async function chargerPrixActifs() {
  try {
    const liste = await apiService.obtenirAssets()
    assetsAvecPrix.value = liste.map(a => ({ id: a.id, prix: null as number | null, variation: null as number | null, chargement: true }))
    await Promise.allSettled(liste.map(async (a, i) => {
      try {
        const b = await apiService.getCandles(a.id, 'M15', 2)
        const curr = b.at(-1)?.close ?? null
        const prev = b.at(-2)?.close ?? null
        assetsAvecPrix.value[i] = { id: a.id, prix: curr, variation: curr && prev ? ((curr - prev) / prev) * 100 : null, chargement: false }
      } catch { assetsAvecPrix.value[i] = { id: a.id, prix: null, variation: null, chargement: false } }
    }))
  } catch { /* silencieux */ }
}

function formatUsd(v: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 }).format(v)
}

function formatPrixAsset(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  return v >= 10 ? v.toFixed(2) : v.toFixed(4)
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

  // Vérification IB Gateway
  try {
    const ib = await apiService.ibStatus()
    ibGatewayOk.value = ib.connecte
  } catch {
    ibGatewayOk.value = false
  }

  await Promise.allSettled([
    chargerPrixActifs(),
    signalStore.chargerSignaux(10),
    signalStore.chargerPrediction(settingsStore.assetActif, settingsStore.timeframeActif),
  ])

  // WS temps réel pour tous les actifs (Binance pour crypto, IB pour métaux/forex/indices)
  const tousLesAssets = assetsAvecPrix.value.map(a => a.id)
  if (tousLesAssets.length > 0) marketStore.connecterPrixLiveAssets(tousLesAssets)
  // Polling 60s pour maintenir les prix initiaux (fallback si WS IB offline le weekend)
  intervalPrix = setInterval(chargerPrixActifs, 60000)
})

onUnmounted(() => {
  if (intervalPrix !== null) clearInterval(intervalPrix)
  marketStore.deconnecterPrixLiveAssets()
})


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
