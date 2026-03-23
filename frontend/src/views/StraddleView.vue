<template>
  <div class="flex flex-col gap-6 max-w-3xl mx-auto">
    <!-- En-tête -->
    <div>
      <h1 class="text-xl font-bold text-white">⚡ Straddle — Créneaux de volatilité</h1>
      <p class="text-sm text-gray-400 mt-1">
        Identifie les récurrences de forte volatilité bidirectionnelle sur Forex, métaux et BTC/ETH.
        Le LLM analyse l'historique OHLCV et propose des créneaux hora­ires à valider en backtest.
      </p>
    </div>

    <!-- Sélecteurs -->
    <div class="glass-card p-4 flex flex-wrap gap-4 items-end">
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Asset</label>
        <select v-model="asset" class="bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white">
          <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Période d'analyse</label>
        <select v-model="periode" class="bg-gray-800 border border-white/10 rounded-lg px-3 py-2 text-sm text-white">
          <option value="3m">3 mois</option>
          <option value="6m">6 mois</option>
          <option value="1a">1 an</option>
          <option value="2a">2 ans</option>
        </select>
      </div>

      <button
        class="px-5 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-semibold transition-all disabled:opacity-50"
        :disabled="chargement"
        @click="analyser"
      >
        {{ chargement ? '⏳ Analyse en cours…' : '🔍 Analyser avec le LLM' }}
      </button>
    </div>

    <!-- Placeholder — résultats à venir -->
    <div class="glass-card p-8 flex flex-col items-center justify-center gap-3 text-center">
      <span class="text-4xl">⚡</span>
      <p class="text-white font-semibold">Fonctionnalité en cours de développement</p>
      <p class="text-sm text-gray-400 max-w-md">
        Cette vue permettra au LLM d'analyser les récurrences de volatilité sur l'historique OHLCV
        et de proposer des créneaux horaires (ex : "Mardi 14h–16h UTC, XAUUSD, ATR +0.8%").
        Les créneaux seront ensuite testés en backtest via l'onglet dédié.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const asset = ref('XAUUSD')
const periode = ref('6m')
const chargement = ref(false)

const assets = ['XAUUSD', 'XAGUSD', 'EURUSD', 'GBPUSD', 'USDJPY', 'BTCUSDT', 'ETHUSDT']

async function analyser() {
  chargement.value = true
  // TODO: appel POST /api/straddle/analyser
  await new Promise(r => setTimeout(r, 1000))
  chargement.value = false
}
</script>
