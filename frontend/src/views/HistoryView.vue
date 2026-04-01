<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">📋 Historique des Signaux</h1>
      <button v-if="!rocketsMode" class="btn-outline text-sm" @click="exportOuvert = true">⬇ Export CSV/PDF</button>
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

      <div class="flex gap-1 ml-auto">
        <button
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStatut === 'en_cours' }"
          @click="filtreStatut = 'en_cours'">⏳ En cours</button>
        <button
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStatut === 'cloturees' }"
          @click="filtreStatut = 'cloturees'">✅ Clôturées</button>
      </div>
      <button class="btn-sm" @click="charger">🔄 Actualiser</button>
      <button v-if="filtreStrategie !== ''" class="btn-sm bg-purple-700 hover:bg-purple-600" @click="analyseOuverte = true">📊 Analyse {{ labelStrategie }}</button>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-hidden" style="max-height: calc(100vh - 240px); overflow-y: auto;">
      <div v-if="chargement" class="text-center text-gray-500 py-10">Chargement...</div>
      <div v-else-if="!listeActive.length" class="text-center text-gray-500 py-10">
        Aucun signal correspondant aux filtres
      </div>

      <!-- Tableau Rockets -->
      <RocketsTableau
        v-else-if="rocketsMode"
        :rockets="rocketsTries"
        :prix-actuels="prixActuels"
        :tri-colonne="triColonne"
        :tri-dir="triDir"
        @trier-par="trierPar"
      />

      <!-- Tableau Signaux standard -->
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-3 py-3 text-left">#</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('asset')">Asset <span class="tri-icone">{{ icone('asset') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('timeframe')">TF / Phase <span class="tri-icone">{{ icone('timeframe') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('direction')">Direction <span class="tri-icone">{{ icone('direction') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('score')">Score <span class="tri-icone">{{ icone('score') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_entree')">Entrée <span class="tri-icone">{{ icone('prix_entree') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('stop_loss')">SL <span class="tri-icone">{{ icone('stop_loss') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp1')">TP1 <span class="tri-icone">{{ icone('tp1') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp2')">TP2 <span class="tri-icone">{{ icone('tp2') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('tp3')">TP3 <span class="tri-icone">{{ icone('tp3') }}</span></th>
            <th class="px-3 py-3 text-right">Prix actuel</th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_verdict')">Sortie <span class="tri-icone">{{ icone('prix_verdict') }}</span></th>
            <th class="px-3 py-3 text-center">IA</th>
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
            <td class="px-3 py-3 text-right font-mono" :class="classePrixActuelSignal(s, prixStore.getPrix(s.asset))">{{ prixStore.getPrix(s.asset) !== null ? formatNombre(prixStore.getPrix(s.asset)!) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono text-white">{{ s.prix_verdict ? formatNombre(s.prix_verdict) : '—' }}</td>
            <td class="px-3 py-3 text-center"><span v-if="s.llm_conviction !== null" class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help" :class="classeConviction(s.llm_conviction)" :title="s.llm_raison ?? ''">{{ s.llm_conviction }}</span><span v-else class="text-gray-700 text-xs">—</span></td>
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

    <!-- Modales -->
    <ExportCsvModal :open="exportOuvert" :assets-dispos="assetsConnus" @close="exportOuvert = false" />
    <!-- Modale Analyse Straddle -->
    <StraddleAnalyseModal
      :open="analyseOuverte && filtreStrategie === 'Straddle'"
      :signaux="signaux"
      @close="analyseOuverte = false"
    />

    <!-- Modale Analyse SMC -->
    <SmcAnalyseModal
      :open="analyseOuverte && filtreStrategie === 'SmcDirectional'"
      :signaux="signaux"
      @close="analyseOuverte = false"
    />

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
import { ref, computed, onMounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { usePrixStore } from '@/stores/prix.store'
import RocketsAnalyseModal from '@/components/RocketsAnalyseModal.vue'
import RocketsTableau from '@/components/common/RocketsTableau.vue'
import { useRocketsHistory, rocketToSignal } from '@/composables/useRocketsHistory'
import { formatDate, formatNombre, classeVerdictSignal, labelVerdictSignal } from '@/composables/useSignalFormat'
import SmcAnalyseModal from '@/components/common/SmcAnalyseModal.vue'
import StraddleAnalyseModal from '@/components/common/StraddleAnalyseModal.vue'
import ExportCsvModal from '@/components/common/ExportCsvModal.vue'

const alerteStore = useAlerteStore()
const prixStore = usePrixStore()
const signaux  = ref<Signal[]>([])
const chargement    = ref(false)
const analyseOuverte = ref(false)
const exportOuvert   = ref(false)
const filtreAsset   = ref('')
const filtreDirection = ref('')
const filtreStrategie = ref('')
const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('en_cours')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

const rocketsMode = computed(() => filtreStrategie.value === 'Rockets')

const { rockets, prixActuels, chargerRockets, rocketsTries, rocketsFiltrés } =
  useRocketsHistory(rocketsMode, filtreStatut, triColonne, triDir)

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
function classeConviction(c: number | null): string {
  if (c === null) return 'bg-gray-700 text-gray-400'
  return c >= 70 ? 'bg-emerald-900 text-emerald-300 border border-emerald-600' : c >= 50 ? 'bg-yellow-900 text-yellow-300 border border-yellow-600' : 'bg-red-900 text-red-300 border border-red-600'
}
function classePrixActuelSignal(s: Signal, prix: number | null): string {
  if (!prix) return 'text-gray-400'
  const long = s.direction === 'LONG'
  if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'text-red-400'
  if (s.take_profit[2] && (long ? prix >= s.take_profit[2] : prix <= s.take_profit[2])) return 'text-emerald-200'
  if (s.take_profit[1] && (long ? prix >= s.take_profit[1] : prix <= s.take_profit[1])) return 'text-emerald-300'
  return (long ? prix >= s.take_profit[0] : prix <= s.take_profit[0]) ? 'text-emerald-400' : 'text-blue-300'
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

const labelStrategie = computed(() => {
  if (filtreStrategie.value === 'Rockets') return 'Rockets'
  if (filtreStrategie.value === 'Straddle') return 'Straddle'
  if (filtreStrategie.value === 'SmcDirectional') return 'SMC Directionnel'
  return ''
})

const assetsConnus = computed(() =>
  [...new Set(signaux.value.map(s => s.asset))].sort()
)

const signalsFiltres = computed(() =>
  signaux.value.filter(s =>
    (!filtreAsset.value || s.asset === filtreAsset.value) &&
    (!filtreDirection.value || s.direction === filtreDirection.value) &&
    (!filtreStrategie.value || s.strategie === filtreStrategie.value) &&
    (filtreStatut.value === 'en_cours' ? s.verdict === null : s.verdict !== null)
  )
)

const listeActive = computed(() =>
  rocketsMode.value ? rocketsFiltrés.value : signalsFiltres.value
)

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

watch(rocketsMode, (val) => { triColonne.value = ''; if (val) charger() })
watch(filtreStrategie, (val, old) => {
  // Recharger si on passe de/vers "Toutes" pour inclure/exclure les rockets
  if ((val === '' || old === '') && val !== 'Rockets' && old !== 'Rockets') charger()
})

async function charger() {
  chargement.value = true
  try {
    if (rocketsMode.value) {
      await chargerRockets()
    } else {
      const [signauxData] = await Promise.all([
        apiService.getSignaux(500),
        filtreStrategie.value === '' ? chargerRockets() : Promise.resolve(),
      ])
      const rocketsConverties = filtreStrategie.value === '' ? rockets.value.map(rocketToSignal) : []
      signaux.value = [...signauxData, ...rocketsConverties]
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Erreur chargement: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

onMounted(() => charger())
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

