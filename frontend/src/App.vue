<template>
  <div id="app" class="flex h-screen bg-gray-900 text-white">
    <!-- Zone de déclenchement invisible sur le bord gauche -->
    <div class="sidebar-trigger fixed left-0 top-0 h-full w-3 z-50" />
    <SideBar />
    <main class="flex-1 overflow-y-auto px-4 py-6 flex flex-col">
      <RouterView />
    </main>
    <ToastAlerte />
    <SignalAlarmeModal />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import SideBar from './components/common/SideBar.vue'
import ToastAlerte from './components/common/ToastAlerte.vue'
import SignalAlarmeModal from './components/common/SignalAlarmeModal.vue'
import { useAssetsStore } from '@/stores/assets.store'
import { useSignalAlarmeStore } from '@/stores/signal-alarme.store'
import { useNotification } from '@/composables/useNotification'
import type { Signal } from '@/services/api.types'

const assetsStore = useAssetsStore()
const alarmeStore = useSignalAlarmeStore()
const { jouerSon } = useNotification()

// Son à chaque nouveau signal (watch ici car App.vue est toujours monté, contrairement à SignalAlarmeModal en v-if)
watch(() => alarmeStore.total, (n, prev) => {
  if (n > prev) jouerSon()
})

const WS_SIGNAUX = 'ws://localhost:8080/api/signal-engine/stream'
let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

function connecterWs() {
  if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) return

  ws = new WebSocket(WS_SIGNAUX)

  ws.onmessage = (event) => {
    try {
      const signal = JSON.parse(event.data as string) as Signal
      alarmeStore.ajouterSignal(signal)
    } catch {
      // message non-JSON ignoré
    }
  }

  ws.onclose = () => {
    ws = null
    // Reconnexion automatique après 10s
    reconnectTimer = setTimeout(connecterWs, 10_000)
  }

  ws.onerror = () => {
    ws?.close()
  }
}

onMounted(() => {
  assetsStore.chargerAssets()
  connecterWs()

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

onUnmounted(() => {
  if (reconnectTimer) clearTimeout(reconnectTimer)
  ws?.close()
})
</script>

<style>
/* La sidebar est par défaut hors-écran, slide-in au survol de la zone trigger */
.sidebar-trigger:hover ~ aside,
aside:hover {
  transform: translateX(0) !important;
  opacity: 1 !important;
}
</style>
