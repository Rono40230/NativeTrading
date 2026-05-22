import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { ImageAvecTF } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

export type StatutAnthropic = 'non-configure' | 'ok' | 'credits-insuffisants'
export const anthropicStatutChart = ref<StatutAnthropic>('non-configure')
export const anthropicActifChart = ref<boolean>(localStorage.getItem('anthropic_actif_chart') !== 'false')

export function toggleAnthropicChart() {
  anthropicActifChart.value = !anthropicActifChart.value
  localStorage.setItem('anthropic_actif_chart', String(anthropicActifChart.value))
}

export interface ContentPart {
  type: 'text' | 'diagram'
  content: string
}

export interface ImageEntry {
  base64: string
  preview: string
  timeframe: string
}

// Ré-exports pour les composants consommateurs
export { renderMd, buildSections } from './chartAnalyseRenderer'
export type { AnalyseSection } from './chartAnalyseRenderer'

function parseContent(text: string): ContentPart[] {
  const parts: ContentPart[] = []
  const regex = /<htmldiagram>([\s\S]*?)<\/htmldiagram>/gi
  let last = 0
  let match: RegExpExecArray | null
  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) {
      parts.push({ type: 'text', content: text.slice(last, match.index) })
    }
    parts.push({ type: 'diagram', content: match[1].trim() })
    last = regex.lastIndex
  }
  if (last < text.length) {
    parts.push({ type: 'text', content: text.slice(last) })
  }
  return parts
}

export function useChartImport() {
  const alerteStore = useAlerteStore()

  const images = ref<ImageEntry[]>([])
  const notes = ref('')
  const analyseEnCours = ref(false)
  const partsResultat = ref<ContentPart[]>([])
  const dragActif = ref(false)
  const modeleUtilise = ref('')

  const analyseLocalEnCours = ref(false)
  const partsResultatLocal = ref<ContentPart[]>([])
  const modeleLocalUtilise = ref('')

  function traiterFichiers(files: File[]) {
    const valides = files.filter(f => f.type.startsWith('image/'))
    if (valides.length === 0) {
      alerteStore.afficherErreur('Aucun fichier image valide — PNG, JPG ou WebP attendu')
      return
    }
    const file = valides[0]
    const reader = new FileReader()
    reader.onload = (e) => {
      const dataUrl = e.target?.result as string
      images.value = [{ preview: dataUrl, base64: dataUrl, timeframe: 'M15' }]
    }
    reader.readAsDataURL(file)
  }

  function onDrop(e: DragEvent) {
    dragActif.value = false
    traiterFichiers(Array.from(e.dataTransfer?.files ?? []))
  }

  function onInputFile(e: Event) {
    traiterFichiers(Array.from((e.target as HTMLInputElement).files ?? []))
    ;(e.target as HTMLInputElement).value = ''
  }

  function supprimerImage(idx: number) { images.value.splice(idx, 1) }
  function mettreAJourTF(idx: number, tf: string) {
    if (images.value[idx]) images.value[idx].timeframe = tf
  }

  async function analyserImage(asset: string) {
    if (images.value.length === 0) {
      alerteStore.afficherErreur("Importez d'abord au moins un chart")
      return
    }
    analyseEnCours.value = true
    partsResultat.value = []
    try {
      const payload: ImageAvecTF[] = images.value.map(img => ({ base64: img.base64, timeframe: img.timeframe }))
      const res = await apiService.analyserChart(asset, payload, notes.value || undefined)
      modeleUtilise.value = res.modele
      anthropicStatutChart.value = 'ok'
      partsResultat.value = parseContent(res.analyse)
    } catch (e: unknown) {
      const axiosErr = e as any
      const detail: string = axiosErr?.response?.data?.error ?? (e as Error).message
      if (detail.toLowerCase().includes('crédit') || detail.toLowerCase().includes('credit')) {
        anthropicStatutChart.value = 'credits-insuffisants'
      }
      alerteStore.afficherErreur(`Vision IA: ${detail}`)
    } finally {
      analyseEnCours.value = false
    }
  }

  function reinitialiser() {
    images.value = []
    notes.value = ''
    partsResultat.value = []
    modeleUtilise.value = ''
    partsResultatLocal.value = []
    modeleLocalUtilise.value = ''
  }

  return {
    images, notes, analyseEnCours, partsResultat, dragActif, modeleUtilise,
    analyseLocalEnCours, partsResultatLocal, modeleLocalUtilise,
    onDrop, onInputFile, analyserImage,
    supprimerImage, mettreAJourTF, reinitialiser,
    setDragActif: (v: boolean) => { dragActif.value = v },
  }
}
