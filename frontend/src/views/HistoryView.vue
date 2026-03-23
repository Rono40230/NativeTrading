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

      <!-- Direction buttons -->
      <div class="flex gap-1">
        <button
          v-for="d in directionsOpts" :key="d.val"
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreDirection === d.val }"
          :disabled="rocketsMode"
          @click="filtreDirection = d.val"
        >{{ d.label }}</button>
      </div>

      <!-- Stratégie buttons -->
      <div class="flex gap-1">
        <button
          v-for="s in strategiesOpts" :key="s.val"
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStrategie === s.val }"
          @click="filtreStrategie = s.val"
        >{{ s.label }}</button>
      </div>

      <button class="btn-sm ml-auto" @click="charger">🔄 Actualiser</button>
      <button v-if="filtreStrategie !== ''" class="btn-sm bg-purple-700 hover:bg-purple-600" @click="analyseOuverte = true">📊 Analyse {{ labelStrategie }}</button>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-hidden" style="max-height: calc(100vh - 240px); overflow-y: auto;">
      <div v-if="chargement" class="text-center text-gray-500 py-10">Chargement...</div>
      <div v-else-if="!listeActive.length" class="text-center text-gray-500 py-10">
        Aucun signal correspondant aux filtres
      </div>

      <!-- Tableau Rockets -->
      <table v-else-if="rocketsMode" class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-4 py-3 text-left">#</th>
            <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('ticker')">Ticker <span class="tri-icone">{{ icone('ticker') }}</span></th>
            <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('phase')">Phase <span class="tri-icone">{{ icone('phase') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('score')">Score <span class="tri-icone">{{ icone('score') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_entree')">Entrée <span class="tri-icone">{{ icone('prix_entree') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('stop_loss')">SL <span class="tri-icone">{{ icone('stop_loss') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('target')">TP1 <span class="tri-icone">{{ icone('target') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('target2')">TP2 <span class="tri-icone">{{ icone('target2') }}</span></th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('target3')">TP3 <span class="tri-icone">{{ icone('target3') }}</span></th>
            <th class="px-4 py-3 text-right">Prix actuel</th>
            <th class="px-4 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_verdict')">Sortie <span class="tri-icone">{{ icone('prix_verdict') }}</span></th>
            <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('verdict')">Verdict <span class="tri-icone">{{ icone('verdict') }}</span></th>
            <th class="px-4 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('cree_le')">Date <span class="tri-icone">{{ icone('cree_le') }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, i) in rocketsTries" :key="r.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-4 py-3 text-gray-500">{{ i + 1 }}</td>
            <td class="px-4 py-3 font-semibold text-white">{{ r.ticker }}</td>
            <td class="px-4 py-3">
              <span class="badge" :class="classePhase(r.phase)">{{ r.phase }}</span>
            </td>
            <td class="px-4 py-3 text-right font-mono">{{ r.score }}</td>
            <td class="px-4 py-3 text-right font-mono">{{ formatNombre(r.prix_entree) }}</td>
            <td class="px-4 py-3 text-right font-mono text-red-400">{{ formatNombre(r.stop_loss) }}</td>
            <td class="px-4 py-3 text-right font-mono text-emerald-400">{{ formatNombre(r.target) }}</td>
            <td class="px-4 py-3 text-right font-mono text-emerald-300">{{ r.target2 ? formatNombre(r.target2) : '—' }}</td>
            <td class="px-4 py-3 text-right font-mono text-emerald-200">{{ r.target3 ? formatNombre(r.target3) : '\u2014' }}</td>
            <td class="px-4 py-3 text-right font-mono">
              <span v-if="marketStore.prixLive[r.ticker]" :class="classePrixActuel(r)">{{ formatNombre(marketStore.prixLive[r.ticker]) }}</span>
              <span v-else class="text-gray-600">—</span>
            </td>
            <td class="px-4 py-3 text-right font-mono text-white">{{ r.prix_verdict ? formatNombre(r.prix_verdict) : '\u2014' }}</td>
            <td class="px-4 py-3">
              <span class="badge" :class="classeVerdict(r.verdict)">{{ labelVerdict(r) }}</span>
            </td>
            <td class="px-4 py-3 text-gray-500 text-xs">{{ r.cree_le.slice(0, 16).replace('T', ' ') }}</td>
          </tr>
        </tbody>
      </table>

      <!-- Tableau Signaux standard -->
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-3 py-3 text-left">#</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('asset')">Asset <span class="tri-icone">{{ icone('asset') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('timeframe')">TF <span class="tri-icone">{{ icone('timeframe') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('direction')">Direction <span class="tri-icone">{{ icone('direction') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('score')">Score <span class="tri-icone">{{ icone('score') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_entree')">Entrée <span class="tri-icone">{{ icone('prix_entree') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('stop_loss')">SL <span class="tri-icone">{{ icone('stop_loss') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp1')">TP1 <span class="tri-icone">{{ icone('tp1') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp2')">TP2 <span class="tri-icone">{{ icone('tp2') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp3')">TP3 <span class="tri-icone">{{ icone('tp3') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('verdict')">Résultat <span class="tri-icone">{{ icone('verdict') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('strategie')">Stratégie <span class="tri-icone">{{ icone('strategie') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('cree_le')">Date <span class="tri-icone">{{ icone('cree_le') }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, i) in signauxTries" :key="s.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-3 py-3 text-gray-500">{{ i + 1 }}</td>
            <td class="px-3 py-3 font-semibold text-white">{{ s.asset }}</td>
            <td class="px-3 py-3 text-gray-400">{{ s.timeframe }}</td>
            <td class="px-3 py-3">
              <span class="badge" :class="s.direction === 'LONG' ? 'badge-green' : 'badge-red'">{{ s.direction }}</span>
            </td>
            <td class="px-3 py-3 text-right font-mono text-gray-300">{{ s.score.toFixed(0) }}</td>
            <td class="px-3 py-3 text-right font-mono text-white">{{ formatNombre(s.prix_entree) }}</td>
            <td class="px-3 py-3 text-right font-mono text-red-400">{{ formatNombre(s.stop_loss) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-400">{{ formatNombre(s.take_profit[0]) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-300">{{ s.take_profit[1] ? formatNombre(s.take_profit[1]) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-200">{{ s.take_profit[2] ? formatNombre(s.take_profit[2]) : '—' }}</td>
            <td class="px-3 py-3">
              <span class="badge" :class="classeVerdictSignal(s.verdict)">{{ labelVerdictSignal(s.verdict) }}</span>
            </td>
            <td class="px-3 py-3 text-gray-400 text-xs">{{ s.strategie }}</td>
            <td class="px-3 py-3 text-gray-500 text-xs">{{ formatDate(s.cree_le) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Compteur -->
    <div class="text-sm text-gray-400">
      {{ listeActive.length }} entrée{{ listeActive.length > 1 ? 's' : '' }}
    </div>

    <!-- Modale Analyse Straddle -->
    <div v-if="analyseOuverte && filtreStrategie === 'Straddle'" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70" @click.self="analyseOuverte = false">
      <div class="rounded-xl border border-white/10 p-6 w-full max-w-lg text-center" style="background: #0d1117;">
        <div class="flex justify-end mb-4"><button class="text-gray-400 hover:text-white text-xl" @click="analyseOuverte = false">×</button></div>
        <div class="text-4xl mb-4">⚡</div>
        <p class="text-lg font-semibold text-white mb-2">Analyse Straddle</p>
        <p class="text-gray-400">Cette section sera définie prochainement.</p>
      </div>
    </div>

    <!-- Modale Analyse SMC -->
    <div v-if="analyseOuverte && filtreStrategie === 'SmcDirectional'" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70" @click.self="analyseOuverte = false">
      <div class="rounded-xl border border-white/10 p-6 w-full max-w-lg text-center" style="background: #0d1117;">
        <div class="flex justify-end mb-4"><button class="text-gray-400 hover:text-white text-xl" @click="analyseOuverte = false">×</button></div>
        <div class="text-4xl mb-4">🧠</div>
        <p class="text-lg font-semibold text-white mb-2">Analyse SMC Directionnel</p>
        <p class="text-gray-400">Cette section sera définie prochainement.</p>
      </div>
    </div>

    <!-- Modale Analyse Rockets -->
    <RocketsAnalyseModal
      v-if="filtreStrategie === 'Rockets'"
      :open="analyseOuverte"
      :rockets="rockets"
      @close="analyseOuverte = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import type { RocketSignalHistorique } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'
import { useMarketStore } from '@/stores/market.store'
import RocketsAnalyseModal from '@/components/RocketsAnalyseModal.vue'

const marketStore = useMarketStore()

const alerteStore = useAlerteStore()
const signaux  = ref<Signal[]>([])
const rockets  = ref<RocketSignalHistorique[]>([])
const chargement    = ref(false)
const analyseOuverte = ref(false)
const filtreAsset   = ref('')
const filtreDirection = ref('')
const filtreStrategie = ref('')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}

function icone(col: string): string {
  if (triColonne.value !== col) return '\u21c5'
  return triDir.value === 'asc' ? '\u2191' : '\u2193'
}

const directionsOpts = [
  { val: '', label: 'Toutes' },
  { val: 'LONG', label: '📈 LONG' },
  { val: 'SHORT', label: '📉 SHORT' },
]
const strategiesOpts = [
  { val: '', label: 'Toutes' },
  { val: 'Straddle', label: '⚡ Straddle' },
  { val: 'SmcDirectional', label: '🧠 SMC' },
  { val: 'Rockets', label: '🚀 Rockets' },
]

const rocketsMode = computed(() => filtreStrategie.value === 'Rockets')

const labelStrategie = computed(() => {
  if (filtreStrategie.value === 'Rockets') return 'Rockets'
  if (filtreStrategie.value === 'Straddle') return 'Straddle'
  if (filtreStrategie.value === 'SmcDirectional') return 'SMC Directionnel'
  return ''
})

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

function appliquerTri<T extends Record<string, unknown>>(liste: T[], col: string): T[] {
  if (!col) return liste
  return [...liste].sort((a, b) => {
    let va: unknown = a[col] ?? ''
    let vb: unknown = b[col] ?? ''
    if (typeof va === 'string') va = va.toLowerCase()
    if (typeof vb === 'string') vb = vb.toLowerCase()
    const cmp = (va as string | number) < (vb as string | number) ? -1 : (va as string | number) > (vb as string | number) ? 1 : 0
    return triDir.value === 'asc' ? cmp : -cmp
  })
}

const signauxTries = computed(() => {
  const col = triColonne.value
  const liste = signalsFiltres.value as Signal[]
  if (!col) return liste
  return [...liste].sort((a, b) => {
    let va: unknown, vb: unknown
    if (col === 'tp1') { va = a.take_profit[0] ?? 0; vb = b.take_profit[0] ?? 0 }
    else if (col === 'tp2') { va = a.take_profit[1] ?? 0; vb = b.take_profit[1] ?? 0 }
    else if (col === 'tp3') { va = a.take_profit[2] ?? 0; vb = b.take_profit[2] ?? 0 }
    else { va = (a as unknown as Record<string, unknown>)[col] ?? ''; vb = (b as unknown as Record<string, unknown>)[col] ?? '' }
    if (typeof va === 'string') va = va.toLowerCase()
    if (typeof vb === 'string') vb = vb.toLowerCase()
    const cmp = (va as string | number) < (vb as string | number) ? -1 : (va as string | number) > (vb as string | number) ? 1 : 0
    return triDir.value === 'asc' ? cmp : -cmp
  })
})

const rocketsTries = computed(() =>
  appliquerTri(rockets.value as unknown as Record<string, unknown>[], triColonne.value) as unknown as RocketSignalHistorique[]
)

watch(rocketsMode, (val) => { triColonne.value = ''; if (val) charger() })

function formatDate(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' })
}

function formatNombre(v: number | undefined): string {
  if (v === undefined || v === null) return '—'
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  if (v >= 1) return v.toFixed(4)
  return v.toFixed(6)
}

function classeVerdictSignal(verdict: string | null): string {
  if (verdict === 'TP3') return 'badge-green'
  if (verdict === 'TP2') return 'badge-green'
  if (verdict === 'TP1') return 'badge-blue'
  if (verdict === 'SL')  return 'badge-red'
  if (verdict === 'expire') return 'badge-gray'
  return 'badge-yellow'
}

function labelVerdictSignal(verdict: string | null): string {
  if (verdict === 'TP3') return '✅ TP3'
  if (verdict === 'TP2') return '✅ TP2'
  if (verdict === 'TP1') return '🟡 TP1'
  if (verdict === 'SL')  return '❌ SL'
  if (verdict === 'expire') return '⏰ Expiré'
  return '⏳ Actif'
}

function classePhase(phase: string): string {
  if (phase.toLowerCase().includes('break')) return 'badge-green'
  if (phase.toLowerCase().includes('bull')) return 'badge-blue'
  if (phase.toLowerCase().includes('bear')) return 'badge-red'
  return 'badge-yellow'
}

function classeVerdict(verdict: string | null): string {
  if (verdict === 'TP1' || verdict === 'TP2' || verdict === 'TP3' || verdict === 'confirme') return 'badge-green'
  if (verdict === 'invalide') return 'badge-red'
  if (verdict === 'expire')   return 'badge-gray'
  return 'badge-yellow'
}

function labelVerdict(r: RocketSignalHistorique): string {
  const v = r.verdict
  if (v === 'invalide') return '\u274c \u22121R'
  if (v === 'TP1' || v === 'confirme') return '\u2705 +1R'
  if (v === 'TP2') return '\u2705 +2R'
  if (v === 'TP3') {
    const risk = r.prix_entree - r.stop_loss
    if (risk > 0 && r.prix_verdict) {
      const ratio = ((r.prix_verdict - r.prix_entree) / risk).toFixed(1)
      return `\u2705 +${ratio}R`
    }
    return '\u2705 +TP3'
  }
  if (v === 'expire') return '\u23f0 D\u00e9lai 6h d\u00e9pass\u00e9'
  return '\u23f3 En cours'
}

async function charger() {
  chargement.value = true
  // Déconnecter les WS avant rechargement pour forcer la reconnexion
  marketStore.deconnecterPrixLiveAssets()
  try {
    if (rocketsMode.value) {
      rockets.value = await apiService.historiqueRockets(200)
      const tickers = [...new Set(rockets.value.map(r => r.ticker))]
      // Fallback REST : charger le dernier close pour tous les tickers immédiatement
      await Promise.allSettled(
        tickers.map(async ticker => {
          try {
            const candles = await apiService.getCandles(ticker, 'M1', 1)
            if (candles.length > 0) marketStore.prixLive[ticker] = candles[candles.length - 1].close
          } catch { /* ticker non supporté, ignoré */ }
        })
      )
      // Puis connexion WS pour les mises à jour en direct
      marketStore.connecterPrixLiveAssets(tickers)
    } else {
      signaux.value = await apiService.getSignaux(500)
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Erreur chargement: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

function classePrixActuel(r: RocketSignalHistorique): string {
  const prix = marketStore.prixLive[r.ticker]
  if (!prix) return 'text-gray-400'
  if (prix <= r.stop_loss) return 'text-red-400'
  if (r.target3 && prix >= r.target3) return 'text-emerald-200'
  if (r.target2 && prix >= r.target2) return 'text-emerald-300'
  if (prix >= r.target) return 'text-emerald-400'
  return 'text-blue-300'
}

onMounted(charger)
onUnmounted(() => marketStore.deconnecterPrixLiveAssets())
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.glass-select { @apply bg-white border border-gray-300 text-black rounded-lg px-3 py-2; }
.glass-select option { @apply text-black bg-white; }
.glass-select:disabled { @apply opacity-40 cursor-not-allowed; }
.btn-outline { @apply border border-gray-600 text-gray-300 hover:bg-gray-700 px-3 py-2 rounded-lg transition-all; }
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 disabled:opacity-40 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
.filtre-btn { @apply text-xs px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white transition-all disabled:opacity-30 disabled:cursor-not-allowed; }
.filtre-btn-actif { @apply bg-blue-600/30 border-blue-500/50 text-blue-300; }
.badge { @apply text-xs font-bold px-2 py-0.5 rounded-full; }
.badge-green  { @apply bg-emerald-900/60 text-emerald-300; }
.badge-red    { @apply bg-red-900/60 text-red-300; }
.badge-yellow { @apply bg-yellow-900/60 text-yellow-300; }
.badge-blue   { @apply bg-blue-900/60 text-blue-300; }
.badge-gray   { @apply bg-gray-700/60 text-gray-400; }
</style>

