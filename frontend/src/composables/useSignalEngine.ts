/**
 * useSignalEngine — gestion du Signal Engine (start/stop/status + stream WS).
 *
 * Usage : const { actif, secondesRestantes, signaux24h, demarrer, arreter } = useSignalEngine()
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { useSignalStore } from '@/stores/signal.store'

const WS_URL = 'ws://localhost:8080/api/signal-engine/stream'
const POLL_INTERVAL_MS = 30_000

export function useSignalEngine() {
  const alerteStore = useAlerteStore()
  const signalStore = useSignalStore()

  const actif = ref(false)
  const secondesRestantes = ref(0)
  const signaux24h = ref(0)
  const chargement = ref(false)

  let pollTimer: ReturnType<typeof setInterval> | null = null
  let ws: WebSocket | null = null

  // ── Polling statut ─────────────────────────────────────────────────────────

  async function actualiserStatut() {
    try {
      const statut = await apiService.signalEngineStatut()
      actif.value = statut.actif
      secondesRestantes.value = statut.prochain_cycle_dans_secs
      signaux24h.value = statut.signaux_24h
    } catch {
      // Silencieux — backend peut être indisponible au démarrage
    }
  }

  // ── WebSocket signaux temps réel ───────────────────────────────────────────

  function connecterWs() {
    if (ws?.readyState === WebSocket.OPEN) return

    ws = new WebSocket(WS_URL)

    ws.onmessage = (event) => {
      try {
        const signal = JSON.parse(event.data as string)
        // Injecter en tête de liste dans le store signaux
        signalStore.ajouterSignalTempsReel(signal)
        alerteStore.afficher(`🎯 Signal ${signal.asset}/${signal.timeframe} ${signal.direction}`, 'info')
      } catch {
        // Message non-JSON ignoré
      }
    }

    ws.onerror = () => {
      // Reconnexion au prochain cycle de polling
    }

    ws.onclose = () => {
      ws = null
    }
  }

  function deconnecterWs() {
    ws?.close()
    ws = null
  }

  // ── Commandes ──────────────────────────────────────────────────────────────

  async function demarrer() {
    chargement.value = true
    try {
      await apiService.signalEngineDemarrer()
      await actualiserStatut()
      connecterWs()
      alerteStore.afficherSucces('Signal Engine démarré — analyse toutes les 5 min')
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur réseau'
      alerteStore.afficherErreur(`Démarrage Signal Engine: ${msg}`)
    } finally {
      chargement.value = false
    }
  }

  async function arreter() {
    chargement.value = true
    try {
      await apiService.signalEngineArreter()
      await actualiserStatut()
      deconnecterWs()
      alerteStore.afficher('Signal Engine arrêté', 'info')
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur réseau'
      alerteStore.afficherErreur(`Arrêt Signal Engine: ${msg}`)
    } finally {
      chargement.value = false
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  onMounted(async () => {
    await actualiserStatut()
    if (actif.value) connecterWs()
    pollTimer = setInterval(actualiserStatut, POLL_INTERVAL_MS)
  })

  onUnmounted(() => {
    if (pollTimer) clearInterval(pollTimer)
    deconnecterWs()
  })

  return {
    actif,
    secondesRestantes,
    signaux24h,
    chargement,
    demarrer,
    arreter,
    actualiserStatut,
  }
}
