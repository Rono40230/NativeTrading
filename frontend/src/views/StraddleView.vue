<template>
  <div class="flex flex-col gap-5">
    <!-- En-tête -->
    <div>
      <h1 class="text-xl font-bold text-white">⚡ Straddle — Créneaux de volatilité</h1>
      <p class="text-sm text-gray-400 mt-1">
        Le LLM analyse l'historique OHLCV et identifie les créneaux récurrents de forte volatilité bidirectionnelle.
        <RouterLink to="/straddle/backtest" class="text-yellow-400 hover:underline ml-1">→ Backtest</RouterLink>
      </p>
    </div>

    <!-- Panneau de lancement -->
    <div class="glass-card p-4 flex flex-wrap gap-4 items-end">
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Asset</label>
        <select v-model="asset" class="glass-select">
          <option v-for="a in assetsDisponibles" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Période d'analyse</label>
        <select v-model="periode" class="glass-select">
          <option value="3m">3 mois</option>
          <option value="6m">6 mois</option>
          <option value="1a">1 an</option>
          <option value="2a">2 ans</option>
        </select>
      </div>

      <button
        class="btn-primary disabled:opacity-50"
        :disabled="chargement"
        @click="analyser"
      >
        {{ chargement ? '⏳ Analyse LLM…' : '🔍 Analyser' }}
      </button>

      <button class="btn-secondary" @click="chargerCreneaux">
        🔄 Actualiser la liste
      </button>
    </div>

    <!-- Message résultat analyse -->
    <div
      v-if="dernierResultat"
      class="rounded-lg px-4 py-3 text-sm border"
      :class="dernierResultat.nb_retenus > 0
        ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
        : 'bg-amber-500/10 border-amber-500/30 text-amber-300'"
    >
      <span v-if="dernierResultat.nb_retenus > 0">
        ✅ {{ dernierResultat.nb_retenus }} créneau(x) identifié(s) sur
        {{ dernierResultat.nb_analyses }} bougies analysées
      </span>
      <span v-else>
        ⚠️ Aucun créneau retenu sur {{ dernierResultat.nb_analyses }} bougies —
        conviction insuffisante ou volatilité trop faible sur cet asset.
      </span>
    </div>

    <!-- Tableau des créneaux -->
    <div class="glass-card overflow-x-auto">
      <div class="p-4 border-b border-white/10 flex items-center justify-between">
        <h2 class="text-sm font-semibold text-white">📊 Créneaux identifiés</h2>
        <span class="text-xs text-gray-400">{{ creneauxFiltres.length }} créneau(x)</span>
      </div>

      <!-- Filtres -->
      <div class="p-3 border-b border-white/10 flex gap-2 flex-wrap">
        <button
          v-for="f in filtresStatut"
          :key="f.val"
          class="px-3 py-1 text-xs rounded-full border transition-all"
          :class="filtreStatut === f.val
            ? 'bg-yellow-600/30 border-yellow-500/50 text-yellow-300'
            : 'border-white/10 text-gray-400 hover:border-white/30'"
          @click="filtreStatut = f.val"
        >{{ f.label }}</button>
      </div>

      <div v-if="chargementListe" class="p-8 text-center text-gray-500">Chargement…</div>

      <div v-else-if="creneauxFiltres.length === 0" class="p-10 text-center text-gray-500">
        <p class="text-3xl mb-2">⚡</p>
        <p class="text-sm">Aucun créneau — lancez une analyse LLM pour démarrer.</p>
      </div>

      <table v-else class="w-full text-sm">
        <thead>
          <tr class="border-b border-white/10 text-xs text-gray-400 uppercase">
            <th class="text-left px-4 py-3">Asset</th>
            <th class="text-left px-4 py-3">Jour</th>
            <th class="text-left px-4 py-3">Créneau UTC</th>
            <th class="text-center px-4 py-3">ATR ×</th>
            <th class="text-center px-4 py-3">Fréquence</th>
            <th class="text-center px-4 py-3">Conviction</th>
            <th class="text-left px-4 py-3">Raison LLM</th>
            <th class="text-center px-4 py-3">Statut</th>
            <th class="text-center px-4 py-3">Backtest</th>
            <th class="px-4 py-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="c in creneauxFiltres"
            :key="c.id"
            class="border-b border-white/5 hover:bg-white/5 transition-colors"
          >
            <td class="px-4 py-3 font-bold text-white">{{ c.asset }}</td>
            <td class="px-4 py-3 text-gray-300">{{ nomJour(c.jour_semaine) }}</td>
            <td class="px-4 py-3 text-yellow-300 font-mono">{{ c.heure_debut }}–{{ c.heure_fin }}</td>
            <td class="px-4 py-3 text-center">
              <span :class="couleurAtr(c.atr_moyen)" class="font-semibold">
                {{ c.atr_moyen != null ? c.atr_moyen.toFixed(2) + '×' : '—' }}
              </span>
            </td>
            <td class="px-4 py-3 text-center text-gray-300">
              {{ c.frequence != null ? (c.frequence * 100).toFixed(0) + '%' : '—' }}
            </td>
            <td class="px-4 py-3 text-center">
              <span :class="couleurConviction(c.llm_conviction)" class="font-bold">
                {{ c.llm_conviction ?? '—' }}
              </span>
            </td>
            <td class="px-4 py-3 text-gray-400 text-xs max-w-sm truncate" :title="c.llm_raison ?? ''">
              {{ c.llm_raison ?? '—' }}
            </td>
            <td class="px-4 py-3 text-center">
              <select
                :value="c.statut"
                class="bg-gray-800 border border-white/10 rounded px-2 py-1 text-xs text-white"
                @change="changerStatut(c, ($event.target as HTMLSelectElement).value)"
              >
                <option value="a_tester">🔍 À tester</option>
                <option value="valide">✅ Validé</option>
                <option value="invalide">❌ Invalide</option>
              </select>
            </td>
            <td class="px-4 py-3 text-center text-xs">
              <template v-if="c.backtest_winrate != null">
                <span class="text-emerald-400">{{ (c.backtest_winrate * 100).toFixed(0) }}% WR</span>
                <br />
                <span class="text-gray-400">PF {{ c.backtest_profit_factor?.toFixed(2) ?? '—' }}</span>
              </template>
              <span v-else class="text-gray-600">–</span>
            </td>
            <td class="px-4 py-3 text-center">
              <RouterLink
                :to="`/straddle/backtest?asset=${c.asset}&heure=${c.heure_debut}&jour=${c.jour_semaine ?? ''}`"
                class="text-xs text-blue-400 hover:underline"
              >
                🧪 Tester
              </RouterLink>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { apiService } from '@/services/api.service'
import type { StraddleCreneau } from '@/services/api.types'
import { useAssetsStore } from '@/stores/assets.store'
import { useAlerteStore } from '@/stores/alerte.store'

const assetsStore = useAssetsStore()
const alerteStore = useAlerteStore()

// Assets disponibles : Forex, Métaux, BTC/ETH uniquement
const assetsDisponibles = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['XAUUSD', 'XAGUSD', 'EURUSD', 'GBPUSD', 'USDJPY', 'BTCUSDT', 'ETHUSDT']
  return liste
    .filter(a => a.type !== 'crypto' || ['BTC', 'ETH'].includes(a.id))
    .map(a => a.id)
})

const asset = ref('XAUUSD')
const periode = ref('6m')
const chargement = ref(false)
const chargementListe = ref(false)
const creneaux = ref<StraddleCreneau[]>([])
const dernierResultat = ref<{ nb_analyses: number; nb_retenus: number } | null>(null)
const filtreStatut = ref<'tous' | 'a_tester' | 'valide' | 'invalide'>('tous')

const filtresStatut = [
  { val: 'tous',     label: 'Tous' },
  { val: 'a_tester', label: '🔍 À tester' },
  { val: 'valide',   label: '✅ Validés' },
  { val: 'invalide', label: '❌ Invalides' },
] as const

const creneauxFiltres = computed(() =>
  filtreStatut.value === 'tous'
    ? creneaux.value
    : creneaux.value.filter(c => c.statut === filtreStatut.value)
)

const JOURS = ['Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi', 'Dimanche']
function nomJour(jour: number | null): string {
  if (jour == null) return 'Tous'
  return JOURS[jour] ?? `J${jour}`
}

function couleurAtr(v: number | null): string {
  if (v == null) return 'text-gray-500'
  if (v >= 1.8) return 'text-red-400'
  if (v >= 1.4) return 'text-yellow-400'
  return 'text-gray-400'
}

function couleurConviction(v: number | null): string {
  if (v == null) return 'text-gray-500'
  if (v >= 80) return 'text-emerald-400'
  if (v >= 65) return 'text-yellow-400'
  return 'text-red-400'
}

async function chargerCreneaux() {
  chargementListe.value = true
  try {
    creneaux.value = await apiService.getStraddleCreneaux()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Straddle: ${(e as Error).message}`)
  } finally {
    chargementListe.value = false
  }
}

async function analyser() {
  chargement.value = true
  dernierResultat.value = null
  try {
    const res = await apiService.analyserStraddle(asset.value, periode.value)
    creneaux.value = res.creneaux
    dernierResultat.value = { nb_analyses: res.nb_analyses, nb_retenus: res.nb_retenus }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Analyse Straddle échouée: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function changerStatut(c: StraddleCreneau, statut: string) {
  try {
    await apiService.patchStraddleCreneau(c.id, { statut })
    const idx = creneaux.value.findIndex(x => x.id === c.id)
    if (idx !== -1) creneaux.value[idx] = { ...creneaux.value[idx], statut: statut as StraddleCreneau['statut'] }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Mise à jour échouée: ${(e as Error).message}`)
  }
}

onMounted(() => chargerCreneaux())
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white; }
.btn-primary { @apply px-5 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-semibold transition-all; }
.btn-secondary { @apply px-4 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-white text-sm transition-all; }
</style>
