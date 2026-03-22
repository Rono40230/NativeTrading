<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">📋 Historique des Signaux</h1>
      <a v-if="!rocketsMode" :href="exportUrl" target="_blank" class="btn-outline text-sm" title="Exporter CSV">⬇ Export CSV</a>
    </div>

    <!-- Filtres -->
    <div class="glass-card p-4 flex flex-wrap gap-3 items-center">
      <select v-model="filtreAsset" class="glass-select text-sm" :disabled="rocketsMode">
        <option value="">Tous les assets</option>
        <option v-for="a in assetsConnus" :key="a" :value="a">{{ a }}</option>
      </select>
      <select v-model="filtreDirection" class="glass-select text-sm" :disabled="rocketsMode">
        <option value="">Toutes directions</option>
        <option value="LONG">LONG</option>
        <option value="SHORT">SHORT</option>
      </select>
      <select v-model="filtreStrategie" class="glass-select text-sm">
        <option value="">Toutes stratégies</option>
        <option value="Straddle">Straddle</option>
        <option value="SmcDirectional">SMC Directionnel</option>
        <option value="Rockets">🚀 Rockets</option>
      </select>
      <button class="btn-sm ml-auto" @click="charger">🔄 Actualiser</button>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-hidden">
      <div v-if="chargement" class="text-center text-gray-500 py-10">Chargement...</div>
      <div v-else-if="!listePage.length" class="text-center text-gray-500 py-10">
        Aucun signal correspondant aux filtres
      </div>

      <!-- Tableau Rockets -->
      <table v-else-if="rocketsMode" class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-4 py-3 text-left">#</th>
            <th class="px-4 py-3 text-left">Ticker</th>
            <th class="px-4 py-3 text-left">Phase</th>
            <th class="px-4 py-3 text-right">Score</th>
            <th class="px-4 py-3 text-right">Entrée</th>
            <th class="px-4 py-3 text-right">SL</th>
            <th class="px-4 py-3 text-right">TP</th>
            <th class="px-4 py-3 text-left">Verdict</th>
            <th class="px-4 py-3 text-left">Date</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, i) in listePage" :key="r.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-4 py-3 text-gray-500">{{ offsetPage + i + 1 }}</td>
            <td class="px-4 py-3 font-semibold text-white">{{ r.ticker }}</td>
            <td class="px-4 py-3">
              <span class="badge" :class="classePhase(r.phase)">{{ r.phase }}</span>
            </td>
            <td class="px-4 py-3 text-right font-mono">{{ r.score }}</td>
            <td class="px-4 py-3 text-right font-mono">{{ r.prix_entree.toFixed(5) }}</td>
            <td class="px-4 py-3 text-right text-red-400 font-mono">{{ r.stop_loss.toFixed(5) }}</td>
            <td class="px-4 py-3 text-right text-emerald-400 font-mono">{{ r.target.toFixed(5) }}</td>
            <td class="px-4 py-3">
              <span class="badge" :class="classeVerdict(r.verdict)">{{ labelVerdict(r.verdict) }}</span>
            </td>
            <td class="px-4 py-3 text-gray-500 text-xs">{{ r.cree_le.slice(0, 16).replace('T', ' ') }}</td>
          </tr>
        </tbody>
      </table>

      <!-- Tableau Signaux standard -->
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-4 py-3 text-left">#</th>
            <th class="px-4 py-3 text-left">Asset</th>
            <th class="px-4 py-3 text-left">TF</th>
            <th class="px-4 py-3 text-left">Direction</th>
            <th class="px-4 py-3 text-right">Score</th>
            <th class="px-4 py-3 text-right">Entrée</th>
            <th class="px-4 py-3 text-right">SL</th>
            <th class="px-4 py-3 text-right">TP</th>
            <th class="px-4 py-3 text-left">Stratégie</th>
            <th class="px-4 py-3 text-left">Date</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, i) in listePage" :key="s.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-4 py-3 text-gray-500">{{ offsetPage + i + 1 }}</td>
            <td class="px-4 py-3 font-semibold text-white">{{ s.asset }}</td>
            <td class="px-4 py-3 text-gray-400">{{ s.timeframe }}</td>
            <td class="px-4 py-3">
              <span class="badge" :class="s.direction === 'LONG' ? 'badge-green' : 'badge-red'">{{ s.direction }}</span>
            </td>
            <td class="px-4 py-3 text-right font-mono">{{ s.score }}</td>
            <td class="px-4 py-3 text-right font-mono">{{ s.prix_entree.toFixed(2) }}</td>
            <td class="px-4 py-3 text-right text-red-400 font-mono">{{ s.stop_loss.toFixed(2) }}</td>
            <td class="px-4 py-3 text-right text-emerald-400 font-mono">{{ s.take_profit }}</td>
            <td class="px-4 py-3 text-gray-400">{{ s.strategie }}</td>
            <td class="px-4 py-3 text-gray-500 text-xs">{{ formatDate(s.cree_le) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div v-if="totalPages > 1" class="flex items-center justify-between">
      <span class="text-sm text-gray-400">
        {{ listeActive.length }} entrée{{ listeActive.length > 1 ? 's' : '' }} • Page {{ pageCourante + 1 }} / {{ totalPages }}
      </span>
      <div class="flex gap-2">
        <button class="btn-sm" :disabled="pageCourante === 0" @click="pageCourante--">← Préc.</button>
        <button class="btn-sm" :disabled="pageCourante >= totalPages - 1" @click="pageCourante++">Suiv. →</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import type { RocketSignalHistorique } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'

const alerteStore = useAlerteStore()
const signaux  = ref<Signal[]>([])
const rockets  = ref<RocketSignalHistorique[]>([])
const chargement    = ref(false)
const filtreAsset   = ref('')
const filtreDirection = ref('')
const filtreStrategie = ref('')
const pageCourante  = ref(0)
const PAR_PAGE = 8

const rocketsMode = computed(() => filtreStrategie.value === 'Rockets')

const assetsConnus = computed(() =>
  [...new Set(signaux.value.map(s => s.asset))].sort()
)

const exportUrl = apiService.exportSignauxUrl(500)

const signalsFiltres = computed(() =>
  signaux.value.filter(s =>
    (!filtreAsset.value || s.asset === filtreAsset.value) &&
    (!filtreDirection.value || s.direction === filtreDirection.value) &&
    (!filtreStrategie.value || s.strategie === filtreStrategie.value)
  )
)

const listeActive = computed(() =>
  rocketsMode.value ? rockets.value : signalsFiltres.value
)

const totalPages = computed(() => Math.ceil(listeActive.value.length / PAR_PAGE) || 1)
const offsetPage = computed(() => pageCourante.value * PAR_PAGE)
const listePage  = computed(() =>
  listeActive.value.slice(offsetPage.value, offsetPage.value + PAR_PAGE)
)

watch([filtreAsset, filtreDirection, filtreStrategie], () => { pageCourante.value = 0 })
watch(rocketsMode, (val) => { if (val) charger() })

function formatDate(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' })
}

function classePhase(phase: string): string {
  if (phase === 'breakout')     return 'badge-green'
  if (phase === 'prelancement') return 'badge-yellow'
  return 'badge-blue'
}

function classeVerdict(verdict: string | null): string {
  if (verdict === 'confirme') return 'badge-green'
  if (verdict === 'invalide') return 'badge-red'
  if (verdict === 'expire')   return 'badge-gray'
  return 'badge-yellow'
}

function labelVerdict(verdict: string | null): string {
  if (verdict === 'confirme') return '✅ Confirmé'
  if (verdict === 'invalide') return '❌ Invalidé'
  if (verdict === 'expire')   return '⏰ Expiré'
  return '⏳ En cours'
}

async function charger() {
  chargement.value = true
  try {
    if (rocketsMode.value) {
      rockets.value = await apiService.historiqueRockets(200)
    } else {
      signaux.value = await apiService.getSignaux(500)
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Erreur chargement: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

onMounted(charger)
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.glass-select:disabled { @apply opacity-40 cursor-not-allowed; }
.btn-outline { @apply border border-gray-600 text-gray-300 hover:bg-gray-700 px-3 py-2 rounded-lg transition-all; }
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 disabled:opacity-40 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
.badge { @apply text-xs font-bold px-2 py-0.5 rounded-full; }
.badge-green  { @apply bg-emerald-900/60 text-emerald-300; }
.badge-red    { @apply bg-red-900/60 text-red-300; }
.badge-yellow { @apply bg-yellow-900/60 text-yellow-300; }
.badge-blue   { @apply bg-blue-900/60 text-blue-300; }
.badge-gray   { @apply bg-gray-700/60 text-gray-400; }
</style>

