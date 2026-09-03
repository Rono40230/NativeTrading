<template>
  <div class="space-y-4">
    <!-- Une carte par stratégie : registre + paramètres moteur, un seul
         bouton. filtre = n'en montrer qu'une (bouton ⚙️ de sa page). -->
    <div class="grid grid-cols-1 gap-4" :class="{ 'md:grid-cols-3': !filtre }">
      <SmcParamsCard v-if="parId('SMC') && (!filtre || filtre === 'SMC')" :s="parId('SMC')!" />
      <StraddleParamsCard v-if="parId('straddle') && (!filtre || filtre === 'straddle')" :s="parId('straddle')!" />
      <RocketsParamsCard v-if="parId('rockets') && (!filtre || filtre === 'rockets')" :s="parId('rockets')!" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { http } from '@/services/http.client'
import SmcParamsCard from './common/SmcParamsCard.vue'
import StraddleParamsCard from './common/StraddleParamsCard.vue'
import RocketsParamsCard from './common/RocketsParamsCard.vue'

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
