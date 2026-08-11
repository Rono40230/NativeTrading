/**
 * useSignalEngine — contrôle du Signal Engine (start/stop/status + polling).
 *
 * Le flux temps réel (WS signal-engine/stream) est désormais détenu en un seul
 * exemplaire par App.vue (racine toujours montée), qui nourrit le signalStore,
 * l'alarme modale, le toast et la notification OS. Ce composable ne gère plus que
 * le contrôle (démarrage/arrêt) et le polling du statut — il n'ouvre plus de WS,
 * ce qui met fin aux notifications dupliquées sur le Dashboard.
 *
 * Usage : const { actif, secondesRestantes, signaux24h, demarrer, arreter } = useSignalEngine()
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

const POLL_INTERVAL_MS = 30_000

export function useSignalEngine() {
  const alerteStore = useAlerteStore()

  const actif = ref(false)
  const secondesRestantes = ref(0)
  const signaux24h = ref(0)
  const chargement = ref(false)

  let pollTimer: ReturnType<typeof setInterval> | null = null

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

  // ── Commandes ──────────────────────────────────────────────────────────────

  async function demarrer() {
    chargement.value = true
    try {
      await apiService.signalEngineDemarrer()
      await actualiserStatut()
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
    pollTimer = setInterval(actualiserStatut, POLL_INTERVAL_MS)
  })

  onUnmounted(() => {
    if (pollTimer) clearInterval(pollTimer)
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
