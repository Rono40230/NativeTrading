<template>
  <div class="flex flex-col gap-4">
    <!-- Veille scanner live -->
    <VeilleRockets
      :signaux="veille.signaux.value"
      :total-candidats="veille.totalCandidats.value"
      :chargement="veille.chargement.value"
      :erreur="veille.erreur.value"
      :progression="veille.progression.value"
    />

    <!-- Tableau + onglets -->
    <div class="glass-card">
      <div class="flex items-center border-b border-white/10">
        <button
          v-for="t in tabs"
          :key="t.id"
          class="px-5 py-3 text-sm font-medium transition-colors"
          :class="onglet === t.id ? 'text-white border-b-2 border-emerald-400' : 'text-gray-400 hover:text-white'"
          @click="onglet = t.id"
        >{{ t.label }}</button>
      </div>

      <div class="p-0 overflow-x-auto" v-if="onglet === 'historique'">
        <RocketsTableau
          :rockets="rocketsFiltrés"
          :prix-actuels="prixActuels"
          :tri-colonne="triColonne"
          :tri-dir="triDir"
          @trier-par="trierPar"
        />
      </div>
      <div class="p-4" v-else-if="onglet === 'analyse'">
        <RocketsAnalyseLlm />
      </div>
      <div class="p-4" v-else-if="onglet === 'reglages'">
        <RocketsReglages />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import VeilleRockets from '@/components/common/VeilleRockets.vue'
import RocketsTableau from '@/components/common/RocketsTableau.vue'
import RocketsReglages from '@/components/common/RocketsReglages.vue'
import RocketsAnalyseLlm from '@/components/common/RocketsAnalyseLlm.vue'
import { useVeilleRockets } from '@/composables/useVeilleRockets'
import { useRocketsHistory } from '@/composables/useRocketsHistory'

const veille = useVeilleRockets()
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')
const rocketsMode = computed(() => true)
const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('')

const { rockets, prixActuels, chargerRockets, rocketsFiltrés } =
  useRocketsHistory(rocketsMode, filtreStatut, triColonne, triDir)

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}

const onglet = ref<'historique' | 'reglages' | 'analyse'>('historique')
const tabs = [
  { id: 'historique' as const, label: '📜 Historique' },
  { id: 'analyse'   as const, label: '🧠 Analyse IA' },
  { id: 'reglages'  as const, label: '⚙️ Réglages' },
]

onMounted(() => { veille.demarrer(); chargerRockets() })
onUnmounted(() => veille.arreter())
</script>
