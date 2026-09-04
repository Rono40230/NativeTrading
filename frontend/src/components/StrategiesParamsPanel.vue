<template>
  <div class="space-y-4">
    <!-- Vue SMC : registre/niveaux + timeframes par asset, même ligne. -->
    <div v-if="filtre === 'SMC'" class="grid grid-cols-1 lg:grid-cols-2 gap-4 items-start">
      <SmcParamsCard v-if="parId('SMC')" :s="parId('SMC')!" />
      <SmcCouplesCard />
    </div>

    <!-- Autres vues : une carte par stratégie (filtre = une seule),
         timeframes SMC en dessous quand aucune stratégie n'est filtrée. -->
    <template v-else>
      <div class="grid grid-cols-1 gap-4" :class="{ 'md:grid-cols-3': !filtre }">
        <SmcParamsCard v-if="!filtre && parId('SMC')" :s="parId('SMC')!" />
        <StraddleParamsCard v-if="parId('straddle') && (!filtre || filtre === 'straddle')" :s="parId('straddle')!" />
        <RocketsParamsCard v-if="parId('rockets') && (!filtre || filtre === 'rockets')" :s="parId('rockets')!" />
      </div>
      <SmcCouplesCard v-if="!filtre" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { http } from '@/services/http.client'
import SmcParamsCard from './common/SmcParamsCard.vue'
import StraddleParamsCard from './common/StraddleParamsCard.vue'
import RocketsParamsCard from './common/RocketsParamsCard.vue'
import SmcCouplesCard from './common/SmcCouplesCard.vue'

const props = defineProps<{ filtre?: string | null }>()

interface StrategieApi {
  id: string; nom: string; description: string; icone: string; couleur: string
  etat: string; notifications: boolean; capital: number; risque_pct: number
}
const strategies = ref<StrategieApi[]>([])

function parId(id: string): StrategieApi | undefined {
  return strategies.value.find(s => s.id === id)
}

onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    strategies.value = res.data
  } catch {
    strategies.value = []
  }
})
</script>
