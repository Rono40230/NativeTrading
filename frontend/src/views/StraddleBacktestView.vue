<template>
  <div class="flex flex-col gap-6 max-w-3xl mx-auto">
    <!-- En-tête -->
    <div>
      <h1 class="text-xl font-bold text-white">⚡ Straddle — Backtest des créneaux</h1>
      <p class="text-sm text-gray-400 mt-1">
        Teste statistiquement les créneaux de volatilité identifiés par le LLM.
        Résultats : winrate, profit factor, Sharpe ratio par créneau.
      </p>
    </div>

    <!-- Placeholder -->
    <div class="glass-card p-8 flex flex-col items-center justify-center gap-3 text-center">
      <span class="text-4xl">🧪</span>
      <p class="text-white font-semibold">Fonctionnalité en cours de développement</p>
      <p class="text-sm text-gray-400 max-w-md">
        Cette vue affichera les créneaux identifiés et permettra de lancer un backtest
        sur chaque créneau via le moteur backtest existant.
        Les résultats (winrate, Sharpe, profit factor) seront sauvegardés et comparables.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
// Fonctionnalité à implémenter — Sprint 2 du plan de travail SMC/Straddle
</script>
