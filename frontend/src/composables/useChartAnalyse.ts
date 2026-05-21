import { ref } from 'vue'
import type { IChartApi } from 'lightweight-charts'
import type { Ref } from 'vue'

export const MODELES_VISION = [
  { label: 'qwen2.5vl 7b — Rapide', value: 'qwen2.5vl:7b' },
  { label: 'qwen2.5vl 32b — Test (lourd)', value: 'qwen2.5vl:32b' },
  { label: 'llama3.2-vision 11b — Meta', value: 'llama3.2-vision:11b' },
]

export function useChartAnalyse(
  getChart: () => IChartApi | null,
  selectedAsset: Ref<string>,
  selectedTimeframe: Ref<string>,
) {
  const analyseEnCours = ref(false)
  const analyseResultat = ref<string | null>(null)
  const analyseModele = ref('')
  const modeleSelectionne = ref(MODELES_VISION[0].value)

  async function analyserAvecLlava() {
    const chart = getChart()
    if (!chart) return

    analyseEnCours.value = true
    analyseResultat.value = null

    try {
      const canvas = chart.takeScreenshot()
      const base64 = canvas.toDataURL('image/png').split(',')[1]

      const response = await fetch('http://localhost:8080/api/ia/chart/local', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          asset: selectedAsset.value,
          images: [{ base64, timeframe: selectedTimeframe.value }],
          notes: null,
          model: modeleSelectionne.value,
        }),
        signal: AbortSignal.timeout(300_000),
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

  return { analyseEnCours, analyseResultat, analyseModele, modeleSelectionne, analyserAvecLlava }
}
