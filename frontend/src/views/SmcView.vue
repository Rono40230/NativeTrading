<template>
  <StrategyShell
    titre="Trades SMC en cours"
    icone="📐"
    etat="Officielle"
    route-definition="/smc/definition"
    lexique="smc"
  >
    <template #setups>
      <SetupsFormationPanel strategie="SMC" />
    </template>
    <template #encours>
      <SignauxTableau strategie="SMC" />
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
