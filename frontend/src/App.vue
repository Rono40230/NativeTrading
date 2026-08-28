<template>
  <div id="app" class="flex flex-col h-screen bg-gray-900 text-white">
    <!-- Barre de titre applicative = navigation unique (option 3, 28/08 —
         ancien panneau latéral retiré le même jour, feu vert propriétaire). -->
    <TitleBar />
    <main class="flex-1 min-h-0 overflow-y-auto px-4 py-6 flex flex-col">
      <RouterView />
    </main>
    <ToastAlerte />
    <SignalAlarmeModal />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import TitleBar from './components/common/TitleBar.vue'
import ToastAlerte from './components/common/ToastAlerte.vue'
import SignalAlarmeModal from './components/common/SignalAlarmeModal.vue'
import { useAssetsStore } from '@/stores/assets.store'
import { useSignalAlarmeStore } from '@/stores/signal-alarme.store'
import { useSignalStore } from '@/stores/signal.store'
import { useAlerteStore } from '@/stores/alerte.store'
import { useNotification } from '@/composables/useNotification'
import { WS_BASE_URL } from '@/services/http.client'
import type { Signal } from '@/services/api.types'

const assetsStore = useAssetsStore()
const alarmeStore = useSignalAlarmeStore()
const signalStore = useSignalStore()
const alerteStore = useAlerteStore()
const { jouerSon, signalerSignal } = useNotification()

// Son à chaque nouveau signal (watch ici car App.vue est toujours monté, contrairement à SignalAlarmeModal en v-if)
watch(() => alarmeStore.total, (n, prev) => {
  if (n > prev) jouerSon()
})

onMounted(() => {
  assetsStore.chargerAssets()

    // Exposition dev uniquement — test alarme + son depuis la console Tauri
  if (import.meta.env.DEV) {
    ;(window as any).__testSon = () => jouerSon()
    ;(window as any).__testAlarme = (overrides: Partial<Signal> = {}) => {
      alarmeStore.ajouterSignal({
        id: `test-${Date.now()}`,
        asset: 'BTCUSDT',
        timeframe: 'M5',
        direction: 'LONG',
        score: 85,
        prix_entree: 65000,
        stop_loss: 64000,
        take_profit: [67000, 68500, 70000],
        strategie: 'SMC Directionnel',
        statut: 'Actif',
        verdict: null,
        prix_verdict: null,
        ferme_le: null,
        cree_le: Math.floor(Date.now() / 1000),
        llm_valide: 1,
        llm_conviction: 82,
        llm_raison: 'Signal test — confluence Order Block H1 + RSI survente',
        llm_sl_suggere: null,
        llm_tp1_suggere: null,
        sl_short: null,
        take_profit_short: null,
        sl_long_effectif: null,
        sl_short_effectif: null,
        tps_long_atteints: null,
        tps_short_atteints: null,
        heure_entree: null,
        ...overrides,
      })
    }
  }
})

</script>
