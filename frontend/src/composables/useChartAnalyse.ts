import { ref } from 'vue'
import type { IChartApi } from 'lightweight-charts'
import type { Ref } from 'vue'

export function useChartAnalyse(
  getChart: () => IChartApi | null,
  selectedAsset: Ref<string>,
  selectedTimeframe: Ref<string>,
) {
  const analyseEnCours = ref(false)
  const analyseResultat = ref<string | null>(null)
  const analyseModele = ref('')

  async function analyserAvecLlava() {
    const chart = getChart()
    if (!chart) return

    analyseEnCours.value = true
    analyseResultat.value = null

    try {
      const canvas = chart.takeScreenshot()
      const base64 = canvas.toDataURL('image/png').split(',')[1]

      const response = await fetch('http://localhost:8080/api/ia/chart', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          asset: selectedAsset.value,
          images: [{ base64, timeframe: selectedTimeframe.value }],
          notes: null,
        }),
        signal: AbortSignal.timeout(180_000),
      })

      if (!response.ok) throw new Error(`Ollama HTTP ${response.status}`)

      const data = await response.json() as { analyse: string; modele: string }
      analyseResultat.value = data.analyse
      analyseModele.value = data.modele
    } catch (e: unknown) {
      analyseResultat.value = `Échec analyse: ${e instanceof Error ? e.message : String(e)}`
    } finally {
      analyseEnCours.value = false
    }
  }

  return { analyseEnCours, analyseResultat, analyseModele, analyserAvecLlava }
}
