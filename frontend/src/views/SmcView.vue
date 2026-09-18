<template>
  <StrategyShell
    titre="Stratégie SMC"
    icone="📐"
    etat="Officielle"
    lexique="smc"
    teinte="bg-blue-500/5"
    :afficher-lexique="false"
    :ordre-poses="nbOrdresPoses(signauxActifs, 'SMC')"
    :titre-encours="`${nbEncours} ${nbEncours > 1 ? 'signaux' : 'signal'} en cours`"
  >
    <template #encours>
      <div class="mb-3">
        <SignauxEnAttente :signaux="signauxActifs" strategie="SMC" />
      </div>
      <SignauxTableau strategie="SMC" remplis-seuls @nb-signaux="nbEncours = $event" @signaux-actifs="signauxActifs = $event" />
    </template>
    <template #historique-actions>
      <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="router.push('/smc/analyse')">📊 Analyse</button>
    </template>
    <template #historique>
      <div class="text-sm text-white flex flex-wrap items-center gap-x-3 mb-2">
        <span>{{ historique.signauxFiltres.value.length }} trade{{ historique.signauxFiltres.value.length > 1 ? 's' : '' }}</span>
        <span v-if="historique.totaux.value.sommeR !== null" class="font-mono" :class="historique.totaux.value.sommeR >= 0 ? 'text-emerald-400' : 'text-red-400'" title="Σ R encaissés des clôtures — la même valeur que le badge de la carte et le rapport d'activité">Σ R {{ formatR(historique.totaux.value.sommeR) }}</span>
        
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
import { useRouter } from 'vue-router'
import StrategyShell from '@/components/common/StrategyShell.vue'
import { nbOrdresPoses } from '@/composables/useSignalFormat'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SignauxEnAttente from '@/components/common/SignauxEnAttente.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useHistoriqueStrategie } from '@/composables/useHistoriqueStrategie'
import { formatR } from '@/composables/useSignalFormat'

const router = useRouter()
const historique = useHistoriqueStrategie('smc')
const nbEncours = ref(0)
const signauxActifs = ref<InstanceType<typeof SignauxTableau> extends never ? never : import('@/services/api.service').Signal[]>([])


// Historique rafraîchi en continu (5 s) : une clôture détectée au tick par
// le backend apparaît à l'écran quasi en direct, plus seulement au montage.
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
