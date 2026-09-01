<template>
  <StrategyShell
    titre="Trades Rockets en cours"
    icone="🚀"
    etat="Observation"
    route-definition="/rockets/definition"
    lexique="rockets"
    :titre-encours="`${nbEncours} ${nbEncours > 1 ? 'signaux' : 'signal'} en cours`"
  >
    <template #setups>
      <RocketsScannerView embarque />
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
      <RocketsTableau
        :rockets="rocketsFiltrés"
        :prix-actuels="prixActuels"
        :tri-colonne="triColonne"
        :tri-dir="triDir"
        @trier-par="trierPar"
      />
    </template>
  </StrategyShell>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import StrategyShell from '@/components/common/StrategyShell.vue'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import SignauxEnAttente from '@/components/common/SignauxEnAttente.vue'
import RocketsScannerView from '@/views/RocketsScannerView.vue'
import RocketsTableau from '@/components/common/RocketsTableau.vue'
import { useRocketsHistory } from '@/composables/useRocketsHistory'

const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('cloturees')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')
const nbEncours = ref(0)
const signauxActifs = ref<InstanceType<typeof SignauxTableau> extends never ? never : import('@/services/api.service').Signal[]>([])
const tableauRef = ref<InstanceType<typeof SignauxTableau> | null>(null)
function ouvrirAnalyse() { tableauRef.value?.ouvrirAnalyse() }

const { rockets, prixActuels, chargerRockets, rocketsFiltrés } =
  useRocketsHistory(filtreStatut, triColonne, triDir)

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}

onMounted(() => { void chargerRockets() })
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
