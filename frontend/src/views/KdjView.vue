<template>
  <StrategyShell
    titre="Stratégie KDJ/Halftrend"
    icone="📈"
    :etat="etat"
    lexique="kdj"
    :afficher-lexique="false"
    teinte="bg-cyan-500/5"
    :titre-encours="`${nbEncours} ${nbEncours > 1 ? 'positions' : 'position'} en cours`"
  >
    <template #encours>
      <SignauxTableau strategie="kdj_halftrend" remplis-seuls @nb-signaux="nbEncours = $event" />
    </template>
    <template #historique-actions>
      <button class="btn-sm bg-cyan-700 hover:bg-cyan-600" @click="router.push('/kdj/scanner')">🔭 Scanner tendance</button>
      <button class="btn-sm bg-purple-700 hover:bg-purple-600" @click="router.push('/kdj/analyse')">📊 Analyse</button>
    </template>
    <template #historique>
      <div class="text-sm text-white flex flex-wrap items-center gap-x-3 mb-2">
        <span>{{ historique.signauxFiltres.value.length }} trade{{ historique.signauxFiltres.value.length > 1 ? 's' : '' }}</span>
        <span v-if="historique.totaux.value.sommeR !== null" class="font-mono text-white" title="Σ R-distance des clôtures — la même valeur que le badge de la carte et le rapport d'activité">Σ R {{ formatR(historique.totaux.value.sommeR) }}</span>
        <span class="text-white">· moteur H1, miroir vérifié du Pine étalon</span>
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
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import StrategyShell from '@/components/common/StrategyShell.vue'
import SignauxTableau from '@/components/common/SignauxTableau.vue'
import HistoryTable from '@/components/common/HistoryTable.vue'
import { useHistoriqueStrategie } from '@/composables/useHistoriqueStrategie'
import { formatR } from '@/composables/useSignalFormat'
import { http } from '@/services/http.client'

const router = useRouter()
const historique = useHistoriqueStrategie('kdj_halftrend')
const nbEncours = ref(0)
const etat = ref('Construction')

onMounted(async () => {
  void historique.charger()
  try {
    const res = await http.get<{ id: string; etat: string }[]>('/api/strategies')
    const s = res.data.find((x) => x.id === 'kdj_halftrend')
    if (s) etat.value = s.etat
  } catch { /* registre indisponible — état par défaut */ }
})
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
