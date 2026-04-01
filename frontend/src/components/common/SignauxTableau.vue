<template>
  <div class="flex flex-col gap-4">

    <!-- Filtres -->
    <div class="glass-card p-3 flex items-center gap-3 flex-wrap">
      <div class="flex gap-1">
        <button class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStatut === '' }" @click="filtreStatut = ''">Tous</button>
        <button class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStatut === 'en_cours' }" @click="filtreStatut = 'en_cours'">⏳ En cours</button>
        <button class="filtre-btn" :class="{ 'filtre-btn-actif': filtreStatut === 'cloturees' }" @click="filtreStatut = 'cloturees'">✅ Clôturées</button>
      </div>
      <span class="text-xs text-gray-500 ml-2">{{ listeActive.length }} signal{{ listeActive.length !== 1 ? 's' : '' }}</span>
      <div class="flex gap-2 ml-auto">
        <button class="btn-sm" @click="charger">🔄 Actualiser</button>
        <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="analyseOuverte = true">📊 Analyse</button>
      </div>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-hidden" style="max-height: calc(100vh - 280px); overflow-y: auto;">
      <div v-if="chargement" class="text-center text-gray-500 py-10">Chargement…</div>
      <div v-else-if="!listeActive.length" class="text-center text-gray-500 py-10">Aucun signal correspondant</div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-3 py-3 text-left">#</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('asset')">Asset <span>{{ icone('asset') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('timeframe')">TF / Phase <span>{{ icone('timeframe') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('direction')">Direction <span>{{ icone('direction') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('score')">Score <span>{{ icone('score') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_entree')">Entrée <span>{{ icone('prix_entree') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('stop_loss')">SL <span>{{ icone('stop_loss') }}</span></th>
            <th class="px-3 py-3 text-right">TP1</th>
            <th class="px-3 py-3 text-right">TP2</th>
            <th class="px-3 py-3 text-right">TP3</th>
            <th class="px-3 py-3 text-right">Prix actuel</th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_verdict')">Sortie <span>{{ icone('prix_verdict') }}</span></th>
            <th class="px-3 py-3 text-center">IA</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('verdict')">Résultat <span>{{ icone('verdict') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('cree_le')">Date <span>{{ icone('cree_le') }}</span></th>
            <th v-if="strategie === 'SmcDirectional'" class="px-3 py-3 text-center w-10"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, i) in signauxTries" :key="s.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-3 py-3 text-gray-500">{{ i + 1 }}</td>
            <td class="px-3 py-3 font-semibold text-white">{{ s.asset }}</td>
            <td class="px-3 py-3 text-gray-400">{{ s.timeframe }}</td>
            <td class="px-3 py-3">
              <span class="badge" :class="s.direction === 'LONG' ? 'badge-green' : s.direction === 'SHORT' ? 'badge-red' : 'badge-blue'">{{ s.direction }}</span>
            </td>
            <td class="px-3 py-3 text-right font-mono text-gray-300">{{ s.score.toFixed(0) }}</td>
            <td class="px-3 py-3 text-right font-mono text-white">{{ formatNombre(s.prix_entree) }}</td>
            <td class="px-3 py-3 text-right font-mono text-red-400">{{ formatNombre(s.stop_loss) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-400">{{ formatNombre(s.take_profit[0]) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-300">{{ s.take_profit[1] ? formatNombre(s.take_profit[1]) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-200">{{ s.take_profit[2] ? formatNombre(s.take_profit[2]) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono" :class="classePrix(s)">{{ prixStore.getPrix(s.asset) !== null ? formatNombre(prixStore.getPrix(s.asset)!) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono text-white">{{ s.prix_verdict ? formatNombre(s.prix_verdict) : '—' }}</td>
            <td class="px-3 py-3 text-center">
              <span v-if="s.llm_conviction !== null" class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help" :class="classeConviction(s.llm_conviction)" :title="s.llm_raison ?? ''">{{ s.llm_conviction }}</span>
              <span v-else class="text-gray-700 text-xs">—</span>
            </td>
            <td class="px-3 py-3">
              <span class="badge" :class="classeVerdictSignal(s.verdict)">{{ labelVerdictSignal(s.verdict) }}</span>
            </td>
            <td class="px-3 py-3 text-gray-500 text-xs">{{ formatDate(s.cree_le) }}</td>
            <td v-if="strategie === 'SmcDirectional'" class="px-3 py-3 text-center">
              <button class="text-blue-400 hover:text-blue-200 text-sm transition-colors" title="Analyser ce signal avec l'IA" @click="analyserSignal(s)">🔍</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Modales analyse -->
    <StraddleAnalyseModal v-if="strategie === 'Straddle'" :open="analyseOuverte" :signaux="signaux" @close="analyseOuverte = false" />
    <SmcAnalyseModal v-if="strategie === 'SmcDirectional'" :open="analyseOuverte" :signaux="signaux" @close="analyseOuverte = false" />
    <RocketsAnalyseModal v-if="strategie === 'Rockets'" :open="analyseOuverte" :rockets="rocketsRaw" @close="analyseOuverte = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import type { Signal, RocketSignalHistorique } from '@/services/api.types'
import { apiService } from '@/services/api.service'
import { usePrixStore } from '@/stores/prix.store'
import { formatDate, formatNombre, classeVerdictSignal, labelVerdictSignal } from '@/composables/useSignalFormat'
import { rocketToSignal } from '@/composables/useRocketsHistory'
import StraddleAnalyseModal from '@/components/common/StraddleAnalyseModal.vue'
import SmcAnalyseModal from '@/components/common/SmcAnalyseModal.vue'
import RocketsAnalyseModal from '@/components/RocketsAnalyseModal.vue'

const props = defineProps<{ strategie: 'SmcDirectional' | 'Straddle' | 'Rockets' }>()

const router = useRouter()

function analyserSignal(s: Signal) {
  router.push({
    path: '/smc/analyser',
    query: {
      asset: s.asset,
      tf: s.timeframe,
      dir: s.direction,
      entree: String(s.prix_entree),
      sl: String(s.stop_loss),
      tp1: String(s.take_profit[0] ?? 0),
      tp2: String(s.take_profit[1] ?? 0),
      tp3: String(s.take_profit[2] ?? 0),
    }
  })
}

const prixStore = usePrixStore()
const signaux = ref<Signal[]>([])
const rocketsRaw = ref<RocketSignalHistorique[]>([])
const chargement = ref(true)
const analyseOuverte = ref(false)
const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('en_cours')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

function trierPar(col: string) {
  if (triColonne.value === col) triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  else { triColonne.value = col; triDir.value = 'desc' }
}

function icone(col: string): string {
  if (triColonne.value !== col) return '⇅'
  return triDir.value === 'asc' ? '↑' : '↓'
}

function classeConviction(c: number | null): string {
  if (c === null) return 'bg-gray-700 text-gray-400'
  return c >= 70 ? 'bg-emerald-900 text-emerald-300 border border-emerald-600'
    : c >= 50 ? 'bg-yellow-900 text-yellow-300 border border-yellow-600'
    : 'bg-red-900 text-red-300 border border-red-600'
}

function classePrix(s: Signal): string {
  const prix = prixStore.getPrix(s.asset)
  if (!prix || s.direction === 'Both') return 'text-gray-400'
  const long = s.direction === 'LONG'
  if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'text-red-400'
  if (s.take_profit[2] && (long ? prix >= s.take_profit[2] : prix <= s.take_profit[2])) return 'text-emerald-200'
  if (s.take_profit[1] && (long ? prix >= s.take_profit[1] : prix <= s.take_profit[1])) return 'text-emerald-300'
  return (long ? prix >= s.take_profit[0] : prix <= s.take_profit[0]) ? 'text-emerald-400' : 'text-blue-300'
}

const listeActive = computed(() =>
  signaux.value.filter(s => {
    if (filtreStatut.value === 'en_cours') return s.verdict === null
    if (filtreStatut.value === 'cloturees') return s.verdict !== null
    return true
  })
)

const signauxTries = computed(() => {
  const col = triColonne.value
  if (!col) return listeActive.value
  return [...listeActive.value].sort((a, b) => {
    let va: unknown, vb: unknown
    if (col === 'tp1') { va = a.take_profit[0] ?? 0; vb = b.take_profit[0] ?? 0 }
    else { va = (a as Record<string, unknown>)[col] ?? ''; vb = (b as Record<string, unknown>)[col] ?? '' }
    if (typeof va === 'string') va = va.toLowerCase()
    if (typeof vb === 'string') vb = vb.toLowerCase()
    const cmp = (va as string | number) < (vb as string | number) ? -1 : (va as string | number) > (vb as string | number) ? 1 : 0
    return triDir.value === 'asc' ? cmp : -cmp
  })
})

async function charger() {
  chargement.value = true
  try {
    if (props.strategie === 'Rockets') {
      rocketsRaw.value = await apiService.historiqueRockets(500)
      signaux.value = rocketsRaw.value.map(rocketToSignal)
    } else {
      const data = await apiService.getSignaux(500)
      signaux.value = data.filter(s => s.strategie === props.strategie)
    }
  } catch { /* silencieux */ } finally {
    chargement.value = false
  }
}

onMounted(() => charger())
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
.filtre-btn { @apply text-xs px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white transition-all; }
.filtre-btn-actif { @apply bg-blue-600/30 border-blue-500/50 text-blue-300; }
.badge { @apply text-xs font-bold px-2 py-0.5 rounded-full; }
.badge-green { @apply bg-emerald-900/60 text-emerald-300; }
.badge-red   { @apply bg-red-900/60 text-red-300; }
.badge-blue  { @apply bg-blue-900/60 text-blue-300; }
</style>
