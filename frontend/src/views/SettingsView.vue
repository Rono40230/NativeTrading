<template>
  <div class="space-y-6">
    <h1 class="text-3xl font-bold">⚙️ Configuration</h1>

    <!-- Capital de départ -->
    <div class="glass-card p-6">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Compte</h2>
      <div class="space-y-4">
        <div>
          <label class="block mb-2 text-sm text-gray-300">Capital de départ (€)</label>
          <div class="flex gap-3 items-center">
            <input
              v-model.number="capitalSaisie"
              type="number"
              min="1"
              step="100"
              class="bg-gray-700 text-white rounded px-3 py-2 w-48 focus:outline-none focus:ring-2 focus:ring-emerald-500"
              @keyup.enter="sauvegarder"
            />
            <button
              class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded text-sm font-medium transition-colors"
              @click="sauvegarder"
            >
              Enregistrer
            </button>
            <span v-if="sauvegarde" class="text-emerald-400 text-sm">✓ Sauvegardé</span>
          </div>
          <p class="text-xs text-gray-500 mt-1">Utilisé pour le backtesting et le dimensionnement des positions</p>
        </div>
      </div>
    </div>

    <!-- Risque par trade -->
    <div class="glass-card p-6">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Gestion du risque</h2>
      <div>
        <label class="block mb-2 text-sm text-gray-300">Risque par trade (%)</label>
        <input type="number" value="1.0" min="0.1" max="5" step="0.1"
          class="bg-gray-700 text-white rounded px-3 py-2 w-48 focus:outline-none focus:ring-2 focus:ring-emerald-500">
        <p class="text-xs text-gray-500 mt-1">Max 2% recommandé (limite absolue : 2%)</p>
      </div>
    </div>

    <!-- Sources de données -->
    <div class="glass-card p-6">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-1">Sources de données</h2>
      <p class="text-xs text-gray-500 mb-4">
        Tous les actifs → IB Gateway (Interactive Brokers)
      </p>
      <div class="flex items-center gap-3 px-4 py-3 bg-blue-900/20 border border-blue-700/30 rounded-lg">
        <span class="text-2xl">🔌</span>
        <div>
          <p class="text-sm font-medium text-blue-300">IB Gateway — Provider unique</p>
          <p class="text-xs text-gray-400 mt-0.5">
            XAUUSD · XAGUSD · BTC · ETH · Tous timeframes M1→W1
          </p>
        </div>
        <div class="ml-auto">
          <span class="text-xs px-2 py-1 rounded bg-yellow-900/40 text-yellow-400 border border-yellow-700/30">
            ⏳ Intégration en cours
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useSettingsStore } from '@/stores/settings.store'

const settingsStore = useSettingsStore()
const capitalSaisie = ref(settingsStore.capitalDepart)
const sauvegarde = ref(false)

function sauvegarder() {
  if (capitalSaisie.value > 0) {
    settingsStore.definirCapital(capitalSaisie.value)
    sauvegarde.value = true
    setTimeout(() => { sauvegarde.value = false }, 2000)
  }
}
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
  backdrop-filter: blur(12px);
}
</style>
