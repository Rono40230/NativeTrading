<template>
  <StrategyShell
    titre="Trades Rockets en cours"
    icone="🚀"
    etat="Observation"
    route-definition="/rockets/definition"
    lexique="rockets"
  >
    <template #setups>
      <RocketsScannerView embarque />
    </template>
    <template #encours>
      <SignauxTableau strategie="Rockets" />
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
import RocketsScannerView from '@/views/RocketsScannerView.vue'
import RocketsTableau from '@/components/common/RocketsTableau.vue'
import { useRocketsHistory } from '@/composables/useRocketsHistory'

const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('cloturees')
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')
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
