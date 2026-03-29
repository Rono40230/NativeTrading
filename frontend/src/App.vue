<template>
  <div id="app" class="flex h-screen bg-gray-900 text-white">
    <!-- Zone de déclenchement invisible sur le bord gauche -->
    <div class="sidebar-trigger fixed left-0 top-0 h-full w-3 z-50" />
    <SideBar />
    <main class="flex-1 overflow-y-auto px-4 py-6">
      <RouterView />
    </main>
    <ToastAlerte />
    <SignalAlarmeModal />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { RouterView } from 'vue-router'
import SideBar from './components/common/SideBar.vue'
import ToastAlerte from './components/common/ToastAlerte.vue'
import SignalAlarmeModal from './components/common/SignalAlarmeModal.vue'
import { useAssetsStore } from '@/stores/assets.store'
import { useSignalAlarmeStore } from '@/stores/signal-alarme.store'
import type { Signal } from '@/services/api.types'

const assetsStore = useAssetsStore()
const alarmeStore = useSignalAlarmeStore()

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
