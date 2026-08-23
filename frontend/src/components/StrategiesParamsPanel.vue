<template>
  <div class="space-y-4">
    <!-- Réglages par stratégie (registre : état, son TG, capital, risque) -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <StrategieReglagesCard v-for="s in strategies" :key="s.id" :s="s" />
    </div>

    <!-- Paramètres numériques propres au Straddle (moteur branché dessus) -->
    <StraddleParamsCard />

    <!-- Rockets : en construction (moteur VCP à venir) -->
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { http } from '@/services/http.client'
import StrategieReglagesCard from './common/StrategieReglagesCard.vue'
import StraddleParamsCard from './common/StraddleParamsCard.vue'

interface StrategieApi {
  id: string; nom: string; description: string; icone: string; couleur: string
  etat: string; notifications: boolean; capital: number; risque_pct: number
}
const strategies = ref<StrategieApi[]>([])

onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    strategies.value = res.data
  } catch {
    strategies.value = []
  }
})
</script>
