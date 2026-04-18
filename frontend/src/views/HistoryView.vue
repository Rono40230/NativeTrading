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
        <span class="text-xs text-gray-400 self-center">✅ Trades clôturés</span>
      </div>
      <button class="btn-sm" @click="charger">🔄 Actualiser</button>
      <button v-if="filtreStrategie !== ''" class="btn-sm bg-purple-700 hover:bg-purple-600" @click="analyseOuverte = true">📊 Analyse {{ labelStrategie }}</button>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-hidden" style="max-height: calc(100vh - 240px); overflow-y: auto;">
      <div v-if="chargement && !listeActive.length" class="text-center text-gray-500 py-10">Chargement...</div>
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
      <HistoryTable
        v-else
        :signaux="signauxTries"
        :filtre-statut="filtreStatut"
        :tri-colonne="triColonne"
        :tri-dir="triDir"
        @trier-par="trierPar"
      />
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
      :open="analyseOuverte && filtreStrategie === 'SMC'"
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
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { usePrixStore } from '@/stores/prix.store'
import RocketsAnalyseModal from '@/components/RocketsAnalyseModal.vue'
import RocketsTableau from '@/components/common/RocketsTableau.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useRocketsHistory, rocketToSignal } from '@/composables/useRocketsHistory'
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
const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('cloturees')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

const rocketsMode = computed(() => filtreStrategie.value === 'Rockets')

const { rockets, prixActuels, chargerRockets, rocketsTries, rocketsFiltrés } =
  useRocketsHistory(filtreStatut, triColonne, triDir)

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}

const directionsOpts = [
  { val: '', label: 'Toutes' },
  { val: 'LONG', label: '📈 LONG' },
  { val: 'SHORT', label: '📉 SHORT' },
]
const strategiesOpts = [
  { val: '', label: 'Toutes' },
  { val: 'Straddle', label: '⚡ Straddle' },
  { val: 'SMC', label: '🧠 SMC' },
  { val: 'Rockets', label: '🚀 Rockets' },
]

const labelStrategie = computed(() => {
  if (filtreStrategie.value === 'Rockets') return 'Rockets'
  if (filtreStrategie.value === 'Straddle') return 'Straddle'
  if (filtreStrategie.value === 'SMC') return 'SMC'
  return ''
})

const assetsConnus = computed(() =>
  [...new Set(signaux.value.map(s => s.asset))].sort()
)

const signalsFiltres = computed(() =>
  signaux.value.filter(s =>
    (!filtreAsset.value || s.asset === filtreAsset.value) &&
    (!filtreDirection.value || s.direction === filtreDirection.value) &&
    (!filtreStrategie.value || s.strategie === filtreStrategie.value || (filtreStrategie.value === 'SMC' && s.strategie === 'SMC Directionnel')) &&
    (filtreStatut.value === 'en_cours' ? s.statut !== 'Fermé' : s.statut === 'Fermé')
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
  // N'afficher le spinner que si le tableau est encore vide (premier chargement)
  if (!listeActive.value.length) chargement.value = true
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

let _pollInterval: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  charger()
  _pollInterval = setInterval(() => charger(), 30_000)
})

onUnmounted(() => {
  if (_pollInterval !== null) { clearInterval(_pollInterval); _pollInterval = null }
})
</script>

<style src="./HistoryView.css" />

