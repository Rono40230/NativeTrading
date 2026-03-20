import { ref } from 'vue'
import type { IChartApi } from 'lightweight-charts'
import type { AnnonceCalendrier } from '@/services/api.types'
import { apiService } from '@/services/api.service'
import { useEcoCalCanvas } from './useEcoCalCanvas'

/**
 * Gestion des annonces économiques sur le graphique :
 * chargement API, canvas overlay, tooltip au survol.
 */
export function useChartEcoCal() {
  const ecoCalCanvas = useEcoCalCanvas()
  const annonces = ref<AnnonceCalendrier[]>([])
  const tooltipAnnonce = ref<AnnonceCalendrier | null>(null)
  const tooltipX = ref(0)
  const tooltipY = ref(0)

  async function chargerAnnonces() {
    try {
      annonces.value = await apiService.obtenirCalendrier(7)
      ecoCalCanvas.mettreAJour(annonces.value)
    } catch { /* dégradation silencieuse */ }
  }

  function initialiser(chart: IChartApi, container: HTMLElement) {
    ecoCalCanvas.initialiser(chart, container)
    ecoCalCanvas.mettreAJour(annonces.value)
    container.addEventListener('mousemove', (e: MouseEvent) => {
      const rect = container.getBoundingClientRect()
      const x = e.clientX - rect.left
      const y = e.clientY - rect.top
      tooltipAnnonce.value = ecoCalCanvas.marqueurSous(x, y)
      tooltipX.value = x
      tooltipY.value = y
    })
    container.addEventListener('mouseleave', () => { tooltipAnnonce.value = null })
  }

  function detruire() {
    ecoCalCanvas.detruire()
  }

  return { initialiser, chargerAnnonces, detruire, tooltipAnnonce, tooltipX, tooltipY }
}
