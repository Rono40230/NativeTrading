/**
 * Notifie l'utilisateur quand une nouvelle analyse LLM SMC Directionnel est disponible.
 *
 * Comportement :
 * - Poll GET /api/smc/analyse-llm toutes les 10 minutes
 * - Compare cree_le avec localStorage → badge orange + toast si nouvelle analyse
 * - Premier démarrage : silencieux (initialise le marqueur sans toast)
 * - Watch sur la route : marque comme vue dès que l'utilisateur visite /smc/*
 */
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAlerteStore } from '@/stores/alerte.store'
import { apiService } from '@/services/api.service'

const LS_KEY   = 'smc_analyse_vue_le'
const POLL_MS  = 10 * 60 * 1000 // 10 minutes

export function useSmcAnalyseNotif() {
  const nouvelleAnalyse = ref(false)
  const alerteStore     = useAlerteStore()
  const route           = useRoute()
  let timer: ReturnType<typeof setInterval> | null = null

  async function verifier() {
    try {
      const data = await apiService.getDerniereAnalyseLlmSmc()
      const creeLe: string | undefined = data?.cree_le
      if (!creeLe) return

      const vue = localStorage.getItem(LS_KEY)
      if (!vue) {
        // Première visite : mémoriser silencieusement sans toast
        localStorage.setItem(LS_KEY, creeLe)
        return
      }
      if (creeLe > vue) {
        nouvelleAnalyse.value = true
        alerteStore.afficher('🧠 Nouvelle analyse SMC Directionnel disponible', 'info')
      }
    } catch { /* backend indisponible — silencieux */ }
  }

  function marquerVue() {
    localStorage.setItem(LS_KEY, new Date().toISOString())
    nouvelleAnalyse.value = false
  }

  // Marquer comme vue dès la navigation vers une page SMC
  watch(() => route.path, (path) => {
    if (path.startsWith('/smc') && nouvelleAnalyse.value) marquerVue()
  })

  onMounted(() => {
    verifier()
    timer = setInterval(verifier, POLL_MS)
  })

  onUnmounted(() => {
    if (timer) clearInterval(timer)
  })

  return { nouvelleAnalyse }
}
