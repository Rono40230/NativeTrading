import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

export interface ContentPart {
  type: 'text' | 'diagram'
  content: string
}

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

/**
 * Convertit le markdown basique en HTML sécurisé (contenu 100 % depuis notre Ollama local).
 * Le texte est échappé en premier, puis les balises sont injectées — pas d'XSS possible.
 */
export function renderMd(text: string): string {
  return text
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/```([\s\S]*?)```/g, (_m, code) =>
      `<pre style="background:#161b22;border:1px solid #21262d;border-radius:8px;padding:12px;overflow-x:auto;font-family:monospace;font-size:12px;color:#7ee787;margin:6px 0">${code.trim()}</pre>`)
    .replace(/`([^`]+)`/g, '<code style="background:#161b22;padding:2px 6px;border-radius:4px;font-size:12px;color:#7ee787">$1</code>')
    .replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>')
    .replace(/\*\*(.+?)\*\*/g, '<strong style="color:#e6edf3">$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/^### (.+)$/gm, '<h3 style="font-size:14px;color:#b794f4;margin:8px 0 2px">$1</h3>')
    .replace(/^## (.+)$/gm, '<h2 style="font-size:15px;color:#63b3ed;margin:10px 0 2px">$1</h2>')
    .replace(/^# (.+)$/gm, '<h1 style="font-size:17px;color:#e6edf3;margin:12px 0 4px">$1</h1>')
    .replace(/^---$/gm, '<hr style="border:none;border-top:1px solid #21262d;margin:8px 0"/>')
    .replace(/^- (.+)$/gm, '<div style="display:flex;gap:8px;margin:1px 0"><span style="color:#63b3ed">•</span><span>$1</span></div>')
    .replace(/\n\n/g, '<br/>')
    .replace(/\n/g, '<br/>')
}

export function useChartImport() {
  const alerteStore = useAlerteStore()

  const imageBase64 = ref('')
  const imagePreview = ref('')
  const notes = ref('')
  const analyseEnCours = ref(false)
  const partsResultat = ref<ContentPart[]>([])
  const dragActif = ref(false)
  const modeleUtilise = ref('')

  function traiterFichier(file: File) {
    if (!file.type.startsWith('image/')) {
      alerteStore.afficherErreur('Fichier non supporté — glissez une image (PNG, JPG, WebP)')
      return
    }
    const reader = new FileReader()
    reader.onload = (e) => {
      const dataUrl = e.target?.result as string
      imagePreview.value = dataUrl
      imageBase64.value = dataUrl.split(',')[1] ?? ''
    }
    reader.readAsDataURL(file)
  }

  function onDrop(e: DragEvent) {
    dragActif.value = false
    const file = e.dataTransfer?.files[0]
    if (file) traiterFichier(file)
  }

  function onInputFile(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (file) traiterFichier(file)
  }

  async function analyserImage(asset: string, timeframe: string) {
    if (!imageBase64.value) {
      alerteStore.afficherErreur("Importez d'abord une image de chart")
      return
    }
    analyseEnCours.value = true
    partsResultat.value = []
    try {
      const res = await apiService.analyserChart(
        asset,
        timeframe,
        imageBase64.value,
        notes.value || undefined,
      )
      modeleUtilise.value = res.modele
      partsResultat.value = parseContent(res.analyse)
    } catch (e: unknown) {
      alerteStore.afficherErreur(`Vision IA: ${(e as Error).message}`)
    } finally {
      analyseEnCours.value = false
    }
  }

  function reinitialiser() {
    imageBase64.value = ''
    imagePreview.value = ''
    notes.value = ''
    partsResultat.value = []
    modeleUtilise.value = ''
  }

  return {
    imageBase64,
    imagePreview,
    notes,
    analyseEnCours,
    partsResultat,
    dragActif,
    modeleUtilise,
    onDrop,
    onInputFile,
    analyserImage,
    reinitialiser,
    setDragActif: (v: boolean) => { dragActif.value = v },
  }
}
