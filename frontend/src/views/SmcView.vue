<template>
  <StrategyShell
    titre="Trades SMC en cours"
    icone="📐"
    etat="Officielle"
    route-definition="/smc/definition"
    lexique="smc"
    :titre-encours="`${nbEncours} signal${nbEncours > 1 ? 's' : ''} en cours`"
  >
    <template #setups>
      <SetupsFormationPanel strategie="SMC" />
    </template>
    <template #encours>
      <SignauxTableau ref="tableauRef" strategie="SMC" @nb-signaux="nbEncours = $event" />
    </template>
    <template #historique-actions>
      <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="ouvrirAnalyse">📊 Analyse</button>
    </template>
    <template #historique>
      <div class="text-sm text-gray-400 flex flex-wrap items-center gap-x-3 mb-2">
        <span>{{ historique.signauxFiltres.value.length }} trade{{ historique.signauxFiltres.value.length > 1 ? 's' : '' }}</span>
        <span v-if="historique.totaux.value.ref !== null" class="font-mono text-emerald-400">Σ palier {{ formatR(historique.totaux.value.ref) }}</span>
        <span v-if="historique.totaux.value.realise !== null" class="font-mono text-gray-500">Σ réalisé {{ formatR(historique.totaux.value.realise) }}</span>
        <span v-if="historique.totaux.value.jamaisRemplis > 0" class="text-gray-600">· {{ historique.totaux.value.jamaisRemplis }} jamais remplis</span>
      </div>
      <HistoryTable
        :signaux="historique.signauxFiltres.value"
        filtre-statut="cloturees"
        :tri-colonne="triColonne"
        :tri-dir="triDir"
        :mfe="historique.mfeParId.value"
        @trier-par="trierPar"
      />
    </template>
  </StrategyShell>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import StrategyShell from '@/components/common/StrategyShell.vue'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SetupsFormationPanel from '@/components/common/SetupsFormationPanel.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useHistoriqueStrategie } from '@/composables/useHistoriqueStrategie'
import { formatR } from '@/composables/useSignalFormat'

const historique = useHistoriqueStrategie('smc')
const nbEncours = ref(0)
const tableauRef = ref<InstanceType<typeof SignauxTableau> | null>(null)
function ouvrirAnalyse() { tableauRef.value?.ouvrirAnalyse() }
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

onMounted(() => { void historique.charger() })
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
