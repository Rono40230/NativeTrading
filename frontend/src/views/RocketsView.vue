<template>
  <StrategyShell
    titre="Stratégie Rockets"
    icone="🚀"
    etat="Observation"
    route-definition="/rockets/definition"
    route-scanner="/rockets/scanner"
    lexique="rockets"
    teinte="bg-purple-500/5"
    route-parametres="/parametres?strategie=rockets"
    libelle-definition="📖 Caractéristiques des Rockets"
    :afficher-lexique="false"
    :titre-encours="`${nbEncours} ${nbEncours > 1 ? 'signaux' : 'signal'} en cours`"
  >
    <template #setups>
      <RocketsSetupsApercu />
      <div class="mt-3">
        <SignauxEnAttente :signaux="signauxActifs" strategie="Rockets" />
      </div>
    </template>
    <template #encours>
      <SignauxTableau ref="tableauRef" strategie="Rockets" remplis-seuls @nb-signaux="nbEncours = $event" @signaux-actifs="signauxActifs = $event" />
    </template>
    <template #historique-actions>
      <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="ouvrirAnalyse">📊 Analyse</button>
    </template>
    <template #historique>
      <div class="text-sm text-white flex flex-wrap items-center gap-x-3 mb-2">
        <span>{{ historique.signauxFiltres.value.length }} trade{{ historique.signauxFiltres.value.length > 1 ? 's' : '' }}</span>
        <span v-if="historique.totaux.value.ref !== null" class="font-mono text-emerald-400">Σ palier {{ formatR(historique.totaux.value.ref) }}</span>
        <span v-if="historique.totaux.value.realise !== null" class="font-mono text-white">Σ réalisé {{ formatR(historique.totaux.value.realise) }}</span>
      </div>
      <HistoryTable
        :signaux="historique.signauxTriés.value"
        filtre-statut="cloturees"
        :tri-colonne="historique.triColonne.value"
        :tri-dir="historique.triDir.value"
        :mfe="historique.mfeParId.value"
        :lots="historique.lotParId.value"
        :journal-comptes="historique.journalComptes.value"
        @trier-par="historique.trierPar"
        @journal-maj="historique.charger()"
      />
    </template>
  </StrategyShell>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import StrategyShell from '@/components/common/StrategyShell.vue'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SignauxEnAttente from '@/components/common/SignauxEnAttente.vue'
import RocketsSetupsApercu from '@/components/common/RocketsSetupsApercu.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useHistoriqueStrategie } from '@/composables/useHistoriqueStrategie'
import { formatR } from '@/composables/useSignalFormat'

// Historique unifié (§10, 05/09) : les trades rockets officiels vivent dans
// `signaux` — même table, mêmes colonnes, même tri que SMC/straddle.
const historique = useHistoriqueStrategie('rockets')
const nbEncours = ref(0)
const signauxActifs = ref<InstanceType<typeof SignauxTableau> extends never ? never : import('@/services/api.service').Signal[]>([])
const tableauRef = ref<InstanceType<typeof SignauxTableau> | null>(null)
function ouvrirAnalyse() { tableauRef.value?.ouvrirAnalyse() }

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void historique.charger()
  minuteur = setInterval(() => { void historique.charger() }, 5_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
