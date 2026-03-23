<template>
  <div class="flex flex-col gap-5">
    <!-- En-tête -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold text-white">🧪 Straddle — Backtest de créneaux</h1>
        <p class="text-sm text-gray-400 mt-1">
          Valide statistiquement les créneaux identifiés par le LLM.
        </p>
      </div>
      <RouterLink to="/straddle" class="text-yellow-400 text-sm hover:underline">
        ← Retour créneaux
      </RouterLink>
    </div>

    <!-- Paramètres backtest -->
    <div class="glass-card p-4 flex flex-wrap gap-4 items-end">
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Asset</label>
        <select v-model="params.asset" class="glass-select">
          <option v-for="a in assetsDisponibles" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Heure début UTC</label>
        <input v-model="params.heure_debut" type="text" placeholder="14:00" class="glass-input" />
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Heure fin UTC</label>
        <input v-model="params.heure_fin" type="text" placeholder="16:00" class="glass-input" />
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Jour (optionnel)</label>
        <select v-model="params.jour_semaine" class="glass-select">
          <option :value="null">Tous les jours</option>
          <option v-for="(j, i) in JOURS" :key="i" :value="i">{{ j }}</option>
        </select>
      </div>
      <button class="btn-primary disabled:opacity-50" :disabled="chargement" @click="lancerBacktest">
        {{ chargement ? '⏳ Backtest…' : '▶ Lancer le backtest' }}
      </button>
    </div>

    <!-- Résultats -->
    <div v-if="resultats" class="glass-card p-5 space-y-4">
      <h2 class="text-sm font-semibold text-white">📊 Résultats backtest</h2>

      <!-- KPIs -->
      <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
        <div class="rounded-xl bg-white/5 border border-white/10 p-4 text-center">
          <p class="text-xs text-gray-400 mb-1">Trades</p>
          <p class="text-2xl font-bold text-white">{{ resultats.total_trades }}</p>
        </div>
        <div class="rounded-xl bg-white/5 border border-white/10 p-4 text-center">
          <p class="text-xs text-gray-400 mb-1">Win Rate</p>
          <p class="text-2xl font-bold" :class="resultats.win_rate >= 0.5 ? 'text-emerald-400' : 'text-red-400'">
            {{ (resultats.win_rate * 100).toFixed(1) }}%
          </p>
        </div>
        <div class="rounded-xl bg-white/5 border border-white/10 p-4 text-center">
          <p class="text-xs text-gray-400 mb-1">Profit Factor</p>
          <p class="text-2xl font-bold" :class="resultats.profit_factor >= 1.2 ? 'text-emerald-400' : 'text-red-400'">
            {{ resultats.profit_factor.toFixed(2) }}
          </p>
        </div>
        <div class="rounded-xl bg-white/5 border border-white/10 p-4 text-center">
          <p class="text-xs text-gray-400 mb-1">Drawdown max</p>
          <p class="text-2xl font-bold text-red-400">
            {{ (resultats.max_drawdown * 100).toFixed(1) }}%
          </p>
        </div>
      </div>

      <!-- Verdict -->
      <div
        class="rounded-xl px-4 py-3 text-sm border"
        :class="verdictClass"
      >{{ verdictTexte }}</div>

      <!-- Sauvegarder dans le créneau -->
      <div v-if="creneauId != null" class="flex justify-end">
        <button class="btn-secondary" @click="sauvegarder">
          💾 Sauvegarder les résultats dans le créneau
        </button>
      </div>
    </div>

    <!-- État vide -->
    <div v-else class="glass-card p-10 text-center text-gray-500">
      <p class="text-3xl mb-2">🧪</p>
      <p class="text-sm">Configurez les paramètres et lancez le backtest.</p>
      <p class="text-xs mt-1 text-gray-600">
        Le backtest applique la stratégie Straddle (Long + Short simultané) sur les bougies historiques H1.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { apiService } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import { useAlerteStore } from '@/stores/alerte.store'

const route = useRoute()
const assetsStore = useAssetsStore()
const alerteStore = useAlerteStore()

const JOURS = ['Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi', 'Dimanche']

const assetsDisponibles = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['XAUUSD', 'XAGUSD', 'EURUSD', 'GBPUSD', 'USDJPY', 'BTCUSDT', 'ETHUSDT']
  return liste
    .filter(a => a.type !== 'crypto' || ['BTC', 'ETH'].includes(a.id))
    .map(a => a.id)
})

const params = ref({
  asset: (route.query.asset as string) || 'XAUUSD',
  heure_debut: (route.query.heure as string) || '14:00',
  heure_fin: '16:00',
  jour_semaine: route.query.jour ? Number(route.query.jour) : null as number | null,
})

const creneauId = ref<number | null>(null)
const chargement = ref(false)
const resultats = ref<{
  total_trades: number
  win_rate: number
  profit_factor: number
  max_drawdown: number
  roi_pct: number
} | null>(null)

const verdictClass = computed(() => {
  if (!resultats.value) return ''
  const r = resultats.value
  if (r.win_rate >= 0.55 && r.profit_factor >= 1.3)
    return 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
  if (r.win_rate >= 0.45 && r.profit_factor >= 1.0)
    return 'bg-amber-500/10 border-amber-500/30 text-amber-300'
  return 'bg-red-500/10 border-red-500/30 text-red-300'
})

const verdictTexte = computed(() => {
  if (!resultats.value) return ''
  const r = resultats.value
  if (r.win_rate >= 0.55 && r.profit_factor >= 1.3)
    return `✅ Créneau validé — WR ${(r.win_rate * 100).toFixed(0)}%, PF ${r.profit_factor.toFixed(2)} — à inclure dans la stratégie Straddle.`
  if (r.win_rate >= 0.45 && r.profit_factor >= 1.0)
    return `⚠️ Résultats mitigés — surveiller sur davantage de données avant de valider.`
  return `❌ Créneau non concluant — WR ${(r.win_rate * 100).toFixed(0)}% insuffisant. Ne pas utiliser.`
})

async function lancerBacktest() {
  chargement.value = true
  try {
    const res = await apiService.runBacktest(params.value.asset, 'H1', 2000, 1000)
    resultats.value = {
      total_trades: res.total_trades,
      win_rate: res.win_rate,
      profit_factor: res.profit_factor,
      max_drawdown: res.max_drawdown_pct / 100,
      roi_pct: res.roi_pct,
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Backtest échoué: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function sauvegarder() {
  if (!resultats.value || creneauId.value == null) return
  try {
    await apiService.patchStraddleCreneau(creneauId.value, {
      backtest_winrate: resultats.value.win_rate,
      backtest_profit_factor: resultats.value.profit_factor,
      statut: resultats.value.win_rate >= 0.55 && resultats.value.profit_factor >= 1.3 ? 'valide' : 'invalide',
    })
    alerteStore.afficherSucces('Résultats sauvegardés dans le créneau')
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Sauvegarde échouée: ${(e as Error).message}`)
  }
}

onMounted(() => {
  if (route.query.id) creneauId.value = Number(route.query.id)
  if (route.query.fin) params.value.heure_fin = route.query.fin as string
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white; }
.glass-input { @apply bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white w-24; }
.btn-primary { @apply px-5 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-semibold transition-all; }
.btn-secondary { @apply px-4 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-white text-sm transition-all; }
</style>
