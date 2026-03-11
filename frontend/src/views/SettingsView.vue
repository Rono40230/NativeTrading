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
        BTC/ETH → Binance (gratuit) · XAUUSD/XAGUSD → Twelvedata
      </p>

      <!-- Binance (lecture seule) -->
      <div class="mb-5">
        <label class="block mb-1 text-sm text-gray-300">Binance API</label>
        <div class="flex items-center gap-2">
          <span class="px-3 py-2 bg-emerald-900/40 border border-emerald-700/40 rounded text-emerald-400 text-sm">
            ✓ Gratuit — aucune clé requise
          </span>
        </div>
      </div>

      <!-- Twelvedata API Key -->
      <div>
        <label class="block mb-1 text-sm text-gray-300">Twelvedata API Key</label>
        <div class="flex gap-3 items-center flex-wrap">
          <input
            v-model="cléTwelvedata"
            :type="afficherCle ? 'text' : 'password'"
            placeholder="ex: 1f192cc6b363..."
            class="bg-gray-700 text-white rounded px-3 py-2 w-72 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
          />
          <button
            class="px-3 py-2 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
            @click="afficherCle = !afficherCle"
          >
            {{ afficherCle ? '🙈 Masquer' : '👁 Afficher' }}
          </button>
          <button
            class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm font-medium transition-colors disabled:opacity-40"
            :disabled="!cléTwelvedata || sauvegardeApiKey"
            @click="sauvegarderApiKey"
          >
            {{ sauvegardeApiKey ? '⏳ Enregistrement...' : '💾 Enregistrer' }}
          </button>
          <button
            class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm font-medium transition-colors disabled:opacity-40"
            :disabled="!cléTwelvedata || testEnCours"
            @click="testerConnexion"
          >
            {{ testEnCours ? '⏳ Test...' : '🔌 Tester' }}
          </button>
        </div>
        <p class="text-xs text-gray-500 mt-1">
          Inscription gratuite sur
          <span class="text-blue-400">twelvedata.com</span> → Dashboard → API Keys
        </p>
        <div v-if="statutTest" class="mt-2 px-3 py-2 rounded text-sm" :class="statutTest.ok ? 'bg-emerald-900/40 text-emerald-400' : 'bg-red-900/40 text-red-400'">
          {{ statutTest.ok ? '✓' : '✗' }} {{ statutTest.message }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useSettingsStore } from '@/stores/settings.store'
import { apiService } from '@/services/api.service'

const settingsStore = useSettingsStore()
const capitalSaisie = ref(settingsStore.capitalDepart)
const sauvegarde = ref(false)
const cléTwelvedata = ref('')
const afficherCle = ref(false)
const sauvegardeApiKey = ref(false)
const testEnCours = ref(false)
const statutTest = ref<{ ok: boolean; message: string } | null>(null)

onMounted(async () => {
  capitalSaisie.value = settingsStore.capitalDepart
  const config = await apiService.obtenirConfig('twelvedata_api_key')
  if (config?.valeur) cléTwelvedata.value = config.valeur
})

function sauvegarder() {
  if (capitalSaisie.value > 0) {
    settingsStore.definirCapital(capitalSaisie.value)
    sauvegarde.value = true
    setTimeout(() => { sauvegarde.value = false }, 2000)
  }
}

async function sauvegarderApiKey() {
  if (!cléTwelvedata.value) return
  sauvegardeApiKey.value = true
  const ok = await apiService.sauvegarderConfig('twelvedata_api_key', cléTwelvedata.value)
  sauvegardeApiKey.value = false
  statutTest.value = ok ? { ok: true, message: 'Clé sauvegardée avec succès' } : { ok: false, message: 'Erreur lors de la sauvegarde' }
  setTimeout(() => { statutTest.value = null }, 3000)
}

async function testerConnexion() {
  if (!cléTwelvedata.value) return
  testEnCours.value = true
  statutTest.value = null
  try {
    // Sauvegarde la clé d'abord pour que le backend l'utilise
    await apiService.sauvegarderConfig('twelvedata_api_key', cléTwelvedata.value)
    const candles = await apiService.getCandles('XAUUSD', 'M15', 1)
    statutTest.value = candles.length > 0
      ? { ok: true, message: `Connexion OK — XAUUSD reçu (${candles.length} bougie)` }
      : { ok: false, message: 'Connexion établie mais aucune donnée reçue' }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : 'Erreur inconnue'
    statutTest.value = { ok: false, message: `Échec: ${msg}` }
  } finally {
    testEnCours.value = false
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
