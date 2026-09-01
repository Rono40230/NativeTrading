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
      <SignauxTableau ref="tableauRef" strategie="Rockets" />
    </template>
    <template #historique>
      <div class="flex justify-end mb-2">
        <button class="btn-sm bg-purple-700 hover:bg-purple-600" title="Analyse de performance approfondie" @click="ouvrirAnalyse">📊 Analyse</button>
      </div>
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
