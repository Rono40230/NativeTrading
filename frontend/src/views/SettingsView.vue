<template>
  <div class="space-y-4">
    <h1 class="text-3xl font-bold">⚙️ Paramètres</h1>

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

      <!-- Clés API : Anthropic, Telegram -->
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
import { ref } from 'vue'
import ApiKeysPanel from '@/components/common/ApiKeysPanel.vue'
import StrategiesParamsPanel from '@/components/StrategiesParamsPanel.vue'
import AssetParamsPanel from '@/components/common/AssetParamsPanel.vue'

const activeTab = ref<'connexion' | 'strategies' | 'risque'>('connexion')
const tabs: { id: typeof activeTab.value; label: string }[] = [
  { id: 'connexion', label: '🔌 Connexion / API' },
  { id: 'strategies', label: '⚙️ Paramétrages des stratégies' },
  { id: 'risque', label: '📊 Gestion du risque' },
]
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
  backdrop-filter: blur(12px);
}
</style>
