<template>
  <div class="space-y-6">
    <h1 class="text-3xl font-bold">⚙️ Configuration</h1>

    <!-- IB Gateway + Compte + Risque sur une ligne -->
    <div class="grid grid-cols-3 gap-4 items-start">

    <!-- IB Gateway -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">IB Gateway — Connexion</h2>
        <div class="flex items-center gap-2 flex-wrap">
          <button
            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
            @click="sauvegarderIB"
          >
            Enregistrer
          </button>
          <button
            class="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-xs font-medium transition-colors"
            :disabled="testEnCours"
            @click="testerConnexion"
          >
            {{ testEnCours ? '…' : '🔌 Tester' }}
          </button>
          <span v-if="ibSauvegarde" class="text-emerald-400 text-xs">✓</span>
          <span v-if="statutConnexion === 'ok'" class="text-xs px-2 py-0.5 rounded bg-emerald-900/40 text-emerald-300 border border-emerald-700/30">✅ Connecté</span>
          <span v-else-if="statutConnexion === 'erreur'" class="text-xs px-2 py-0.5 rounded bg-red-900/40 text-red-400 border border-red-700/30">❌ {{ erreurConnexion }}</span>
        </div>
      </div>
      <div class="grid grid-cols-2 gap-4">
        <!-- Port -->
        <div>
          <label class="block mb-1 text-xs text-gray-400">Port</label>
          <div class="flex gap-2 items-center">
            <input
              v-model.number="ibPort"
              type="number"
              min="1024"
              max="65535"
              placeholder="4002"
              class="bg-gray-700 text-white rounded px-2 py-1.5 w-24 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <span class="text-xs text-gray-500">4002 paper · 4001 live</span>
          </div>
        </div>
        <!-- Client ID -->
        <div>
          <label class="block mb-1 text-xs text-gray-400">Client ID</label>
          <input
            v-model.number="ibClientId"
            type="number"
            min="1"
            max="9999"
            placeholder="100"
            class="bg-gray-700 text-white rounded px-2 py-1.5 w-24 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
      </div>
    </div>

    <!-- Capital de départ -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Compte</h2>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded text-xs font-medium transition-colors"
            @click="sauvegarder"
          >
            Enregistrer
          </button>
          <span v-if="sauvegarde" class="text-emerald-400 text-xs">✓</span>
        </div>
      </div>
      <div>
        <label class="block mb-1 text-xs text-gray-400">Capital de départ (€)</label>
        <div class="flex gap-2 items-center">
          <input
            v-model.number="capitalSaisie"
            type="number"
            min="1"
            step="100"
            class="bg-gray-700 text-white rounded px-2 py-1.5 w-36 text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500"
            @keyup.enter="sauvegarder"
          />
          <span class="text-xs text-gray-500">Utilisé pour le backtesting et le dimensionnement des positions</span>
        </div>
      </div>
    </div>

    <!-- Risque par trade -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Gestion du risque</h2>
      </div>
      <div>
        <label class="block mb-1 text-xs text-gray-400">Risque par trade (%)</label>
        <div class="flex gap-2 items-center">
          <input type="number" value="1.0" min="0.1" max="5" step="0.1"
            class="bg-gray-700 text-white rounded px-2 py-1.5 w-36 text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500">
          <span class="text-xs text-gray-500">Max 2% recommandé (limite absolue : 2%)</span>
        </div>
      </div>
    </div>

    </div> <!-- fin grid 3 colonnes -->

    <!-- Assets -->
    <GestionAssets />

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useSettingsStore } from '@/stores/settings.store'
import { apiService } from '@/services/api.service'
import GestionAssets from '@/components/common/GestionAssets.vue'

const settingsStore = useSettingsStore()
const capitalSaisie = ref(settingsStore.capitalDepart)
const sauvegarde = ref(false)

const ibPort = ref(4002)
const ibClientId = ref(100)
const ibSauvegarde = ref(false)
const statutConnexion = ref<'idle' | 'ok' | 'erreur'>('idle')
const erreurConnexion = ref('')
const testEnCours = ref(false)

// Nettoyage des timers si l'utilisateur navigue avant qu'ils tirent
const timers: ReturnType<typeof setTimeout>[] = []
onUnmounted(() => timers.forEach(clearTimeout))

onMounted(async () => {
  try {
    const port = await apiService.obtenirConfig('ibgateway_port')
    if (port?.valeur != null) ibPort.value = Number(port.valeur)
    const cid = await apiService.obtenirConfig('ibgateway_client_id')
    if (cid?.valeur != null) ibClientId.value = Number(cid.valeur)
  } catch {
    // Backend non disponible — valeurs par défaut utilisées
  }
})

function sauvegarder() {
  if (capitalSaisie.value > 0) {
    settingsStore.definirCapital(capitalSaisie.value)
    sauvegarde.value = true
    timers.push(setTimeout(() => { sauvegarde.value = false }, 2000))
  }
}

async function sauvegarderIB() {
  try {
    await Promise.all([
      apiService.sauvegarderConfig('ibgateway_port', String(ibPort.value)),
      apiService.sauvegarderConfig('ibgateway_client_id', String(ibClientId.value)),
    ])
    ibSauvegarde.value = true
    timers.push(setTimeout(() => { ibSauvegarde.value = false }, 2000))
  } catch {
    // Erreur silencieuse — le backend est peut-être hors-ligne
  }
}

async function testerConnexion() {
  testEnCours.value = true
  statutConnexion.value = 'idle'
  erreurConnexion.value = ''
  try {
    // 1. Vérifier que le backend répond
    await apiService.healthCheck()
  } catch {
    statutConnexion.value = 'erreur'
    erreurConnexion.value = 'Backend non disponible — vérifier que l’app est bien lancée'
    testEnCours.value = false
    return
  }
  // 2. Tester IB Gateway
  const statut = await apiService.ibStatus()
  if (statut.connecte) {
    statutConnexion.value = 'ok'
  } else {
    statutConnexion.value = 'erreur'
    if (statut.erreur?.includes('early eof') || statut.erreur?.includes('eof')) {
      erreurConnexion.value = 'API socket non activée — voir Configurer > API > Paramètres dans IB Gateway'
    } else if (statut.erreur?.includes('Timeout')) {
      erreurConnexion.value = 'IB Gateway ne répond pas (timeout 5s) — est-il ouvert ?'
    } else {
      erreurConnexion.value = statut.erreur ?? 'IB Gateway non disponible'
    }
  }
  testEnCours.value = false
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
