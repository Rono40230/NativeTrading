<template>
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6 text-sm mb-4">
    <StraddleParamsCard />
    <SmcParamsCard />
    <RocketsParamsCard />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'
import StraddleParamsCard from './common/StraddleParamsCard.vue'
import SmcParamsCard from './common/SmcParamsCard.vue'
import RocketsParamsCard from './common/RocketsParamsCard.vue'

const store = useStrategyParamsStore()

onMounted(() => store.charger())
</script>

<style>
/* Global styles for number inputs in settings */
input[type='number']::-webkit-outer-spin-button,
input[type='number']::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
input[type='number'] {
  -moz-appearance: textfield;
}

.animate-fade-in {
  animation: fadeIn 0.2s ease-in-out forwards;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
