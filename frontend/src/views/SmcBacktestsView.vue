<template>
  <div class="flex flex-col items-center justify-center min-h-[60vh] gap-10 px-8">
    <h1 class="text-2xl font-bold text-white">Backtests SMC</h1>
    <div class="flex flex-col sm:flex-row gap-6 w-full max-w-2xl">

      <RouterLink
        to="/straddle/backtest"
        class="flex-1 flex flex-col items-center justify-center gap-4 rounded-2xl border border-blue-500/30 bg-blue-500/10 hover:bg-blue-500/20 transition-all p-10 text-center group"
      >
        <span class="text-5xl">🧪</span>
        <span class="text-lg font-semibold text-white group-hover:text-blue-300 transition-colors">
          Backtester un créneau horaire
        </span>
        <span class="text-xs text-gray-400">Analyse sur une plage horaire définie</span>
      </RouterLink>

      <RouterLink
        to="/straddle"
        class="flex-1 flex flex-col items-center justify-center gap-4 rounded-2xl border border-yellow-500/30 bg-yellow-500/10 hover:bg-yellow-500/20 transition-all p-10 text-center group"
      >
        <span class="text-5xl">📊</span>
        <span class="text-lg font-semibold text-white group-hover:text-yellow-300 transition-colors">
          Backtester une heure précise
        </span>
        <span class="text-xs text-gray-400">Tableau de volatilité horaire</span>
      </RouterLink>

    </div>
  </div>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>
