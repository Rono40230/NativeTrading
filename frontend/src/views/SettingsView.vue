<template>
  <div class="space-y-4">
    <h1 class="text-3xl font-bold">⚙️ Configuration</h1>

    <!-- Navigation onglets -->
    <div class="flex gap-1 border-b border-white/10">
      <button v-for="t in tabs" :key="t.id" :class="activeTab === t.id
        ? 'border-b-2 border-blue-500 text-white bg-white/5'
        : 'text-gray-400 hover:text-white hover:bg-white/5'"
        class="px-4 py-2 text-sm font-medium rounded-t transition-colors" @click="activeTab = t.id">
        {{ t.label }}
      </button>
    </div>

    <!-- ── Onglet : Connexion/API ───────────────────────────────────────────── -->
    <div v-if="activeTab === 'connexion'" class="space-y-4">

      <!-- Statut IG Markets -->
      <div class="glass-card p-4">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">IG Markets — Statut connexion</h2>
          <div class="flex items-center gap-2">
            <button class="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-xs font-medium transition-colors"
              :disabled="testEnCours" @click="testerConnexion">
              {{ testEnCours ? '…' : '🔌 Tester' }}
            </button>
            <span v-if="statutConnexion === 'ok'"
              class="text-xs px-2 py-0.5 rounded bg-emerald-900/40 text-emerald-300 border border-emerald-700/30">✅
              Connecté</span>
            <span v-else-if="statutConnexion === 'erreur'"
              class="text-xs px-2 py-0.5 rounded bg-red-900/40 text-red-400 border border-red-700/30">❌ {{
              erreurConnexion }}</span>
          </div>
        </div>
        <p class="text-xs text-gray-500">Configurez vos identifiants IG Markets dans le panneau ci-dessous, puis testez
          la connexion.</p>
      </div>

      <!-- Clés API : IG Markets, Anthropic, Telegram, Twelve Data -->
      <ApiKeysPanel />

    </div>

    <!-- ── Onglet 3 : Paramétrages des stratégies ──────────────────────────── -->
    <div v-else-if="activeTab === 'strategies'" class="space-y-4">
      <StrategiesParamsPanel />
    </div>

    <!-- ── Onglet 4 : Gestion du risque ───────────────────────────────────── -->
    <div v-else-if="activeTab === 'risque'" class="space-y-4">
      <div class="glass-card p-4">
        <AssetParamsPanel />
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import GestionAssets from '@/components/common/GestionAssets.vue'
import ApiKeysPanel from '@/components/common/ApiKeysPanel.vue'
import StrategiesParamsPanel from '@/components/StrategiesParamsPanel.vue'
import AssetParamsPanel from '@/components/common/AssetParamsPanel.vue'

const activeTab = ref<'connexion' | 'strategies' | 'risque'>('connexion')
const tabs: { id: typeof activeTab.value; label: string }[] = [
  { id: 'connexion', label: '🔌 Connexion / API' },
  { id: 'strategies', label: '⚙️ Paramétrages des stratégies' },
  { id: 'risque', label: '📊 Gestion du risque' },
]

const statutConnexion = ref<'idle' | 'ok' | 'erreur'>('idle')
const erreurConnexion = ref('')
const testEnCours = ref(false)

const timers: ReturnType<typeof setTimeout>[] = []
onUnmounted(() => timers.forEach(clearTimeout))

async function testerConnexion() {
  testEnCours.value = true
  statutConnexion.value = 'idle'
  erreurConnexion.value = ''
  try {
    await apiService.healthCheck()
  } catch {
    statutConnexion.value = 'erreur'
    erreurConnexion.value = 'Backend non disponible'
    testEnCours.value = false
    return
  }
  const statut = await apiService.igStatus()
  if (statut.connecte) {
    statutConnexion.value = 'ok'
  } else {
    statutConnexion.value = 'erreur'
    erreurConnexion.value = statut.erreur ?? 'IG Markets non disponible'
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
