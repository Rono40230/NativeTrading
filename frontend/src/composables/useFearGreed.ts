import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { FearGreedData } from '@/services/api.types'

/**
 * Composable Fear & Greed Index Bitcoin (alternative.me).
 * Rafraîchit automatiquement toutes les heures.
 */
export function useFearGreed() {
  const data = ref<FearGreedData | null>(null)
  let intervalle: ReturnType<typeof setInterval> | null = null

  async function charger() {
    const result = await apiService.obtenirFearGreed()
    if (result !== null) data.value = result
  }

  onMounted(() => {
    charger()
    intervalle = setInterval(charger, 3_600_000) // 1h
  })

  onUnmounted(() => {
    if (intervalle !== null) clearInterval(intervalle)
  })

  return { data }
}
