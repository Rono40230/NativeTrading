<template>
  <StrategyShell
    titre="Stratégie Straddle"
    icone="⚡"
    etat="Observation"
    route-definition="/straddle/definition"
    lexique="straddle"
    route-parametres="/parametres?strategie=straddle"
    libelle-definition="📖 Caractéristiques du Straddle"
    :afficher-lexique="false"
    teinte="bg-amber-500/5"
    :ordre-poses="nbOrdresPoses(signauxActifs, 'straddle')"
    :titre-encours="`${nbEncours} ${nbEncours > 1 ? 'signaux' : 'signal'} en cours`"
  >
    <template #setups>
      <StraddleAgendaPanel />
      <div class="mt-3">
        <SignauxEnAttente :signaux="signauxActifs" strategie="straddle" />
      </div>
      <div class="text-[10px] text-white mt-2">XAU · BTC · NAS100 · SP500 armés au branchement MT5</div>
    </template>
    <template #encours>
      <SignauxTableau ref="tableauRef" strategie="straddle" remplis-seuls @nb-signaux="nbEncours = $event" @signaux-actifs="signauxActifs = $event" />
    </template>
    <template #historique-actions>
      <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="ouvrirAnalyse">📊 Analyse</button>
    </template>
    <template #historique>
      <div class="text-sm text-white flex flex-wrap items-center gap-x-3 mb-2">
        <span>{{ historique.signauxFiltres.value.length }} passe{{ historique.signauxFiltres.value.length > 1 ? 's' : '' }}</span>
        <span v-if="historique.totaux.value.ref !== null" class="font-mono text-emerald-400">Σ palier {{ formatR(historique.totaux.value.ref) }}</span>
        <span v-if="historique.totaux.value.realise !== null" class="font-mono text-white">Σ réalisé {{ formatR(historique.totaux.value.realise) }}</span>
        <span v-if="historique.totaux.value.jamaisRemplis > 0" class="text-white">· {{ historique.totaux.value.jamaisRemplis }} jamais remplis</span>
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
import { nbOrdresPoses } from '@/composables/useSignalFormat'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SignauxEnAttente from '@/components/common/SignauxEnAttente.vue'
import StraddleAgendaPanel from '@/components/common/StraddleAgendaPanel.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useHistoriqueStrategie } from '@/composables/useHistoriqueStrategie'
import { formatR } from '@/composables/useSignalFormat'

const historique = useHistoriqueStrategie('straddle')
const nbEncours = ref(0)
const signauxActifs = ref<InstanceType<typeof SignauxTableau> extends never ? never : import('@/services/api.service').Signal[]>([])
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
