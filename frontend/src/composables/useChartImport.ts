import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { ImageAvecTF } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

export interface ContentPart {
  type: 'text' | 'diagram'
  content: string
}

export interface ImageEntry {
  base64: string
  preview: string
  timeframe: string
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

/** Parse les blocs de tableau Markdown (| col | / |---| / | val |) en HTML tabulaire dark-theme. */
function parseTablesInText(text: string): string {
  const lines = text.split('\n')
  const out: string[] = []
  let i = 0
  while (i < lines.length) {
    const line = lines[i]
    if (
      line.trimStart().startsWith('|') &&
      i + 1 < lines.length &&
      /^\|[\s\-:|]+\|$/.test(lines[i + 1].trim())
    ) {
      const rows: string[] = []
      while (i < lines.length && lines[i].includes('|')) {
        rows.push(lines[i])
        i++
      }
      const dataRows = rows.filter(r => !/^\|[\s\-:|]+\|$/.test(r.trim()))
      const cellsOf = (row: string) =>
        row.split('|').filter((_c, ci, arr) => ci > 0 && ci < arr.length - 1).map(c => c.trim())
      const TH = 'padding:7px 12px;border:1px solid #21262d;background:#161b22;color:#b794f4;font-weight:700;text-align:left;white-space:nowrap;font-size:11px;text-transform:uppercase;letter-spacing:.5px'
      const TD = 'padding:6px 12px;border:1px solid #30363d;color:#e6edf3;text-align:left'
      const TR_ODD = 'background:#0d1117'
      const TR_EVEN = 'background:#111827'
      const html =
        '<div style="overflow-x:auto;margin:10px 0"><table style="width:100%;border-collapse:collapse;font-size:12px;border:1px solid #21262d;border-radius:8px;overflow:hidden">' +
        dataRows.map((row, idx) => {
          const tag = idx === 0 ? 'th' : 'td'
          const style = idx === 0 ? TH : TD
          const rowStyle = idx === 0 ? '' : ` style="${idx % 2 === 1 ? TR_ODD : TR_EVEN}"`
          return `<tr${rowStyle}>${cellsOf(row).map(c => `<${tag} style="${style}">${c}</${tag}>`).join('')}</tr>`
        }).join('') +
        '</table></div>'
      out.push(html)
    } else {
      out.push(line)
      i++
    }
  }
  return out.join('\n')
}

/**
 * Convertit le markdown basique en HTML sécurisé (contenu 100 % depuis notre Ollama local).
 * Le texte est échappé en premier, puis les balises sont injectées — pas d'XSS possible.
 */
export function renderMd(text: string): string {
  const escaped = text
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  return parseTablesInText(escaped)
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

  const images = ref<ImageEntry[]>([])
  const notes = ref('')
  const analyseEnCours = ref(false)
  const partsResultat = ref<ContentPart[]>([])
  const dragActif = ref(false)
  const modeleUtilise = ref('')

  function traiterFichiers(files: File[]) {
    const valides = files.filter(f => f.type.startsWith('image/'))
    if (valides.length === 0) {
      alerteStore.afficherErreur('Aucun fichier image valide — PNG, JPG ou WebP attendu')
      return
    }
    valides.forEach(file => {
      const reader = new FileReader()
      reader.onload = (e) => {
        const dataUrl = e.target?.result as string
        images.value.push({
          preview: dataUrl,
          base64: dataUrl.split(',')[1] ?? '',
          timeframe: 'M15',
        })
      }
      reader.readAsDataURL(file)
    })
  }

  function onDrop(e: DragEvent) {
    dragActif.value = false
    traiterFichiers(Array.from(e.dataTransfer?.files ?? []))
  }

  function onInputFile(e: Event) {
    traiterFichiers(Array.from((e.target as HTMLInputElement).files ?? []))
    ;(e.target as HTMLInputElement).value = ''
  }

  function supprimerImage(idx: number) {
    images.value.splice(idx, 1)
  }

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
      const payload: ImageAvecTF[] = images.value.map(img => ({
        base64: img.base64,
        timeframe: img.timeframe,
      }))
      const res = await apiService.analyserChart(asset, payload, notes.value || undefined)
      modeleUtilise.value = res.modele
      partsResultat.value = parseContent(res.analyse)
    } catch (e: unknown) {
      alerteStore.afficherErreur(`Vision IA: ${(e as Error).message}`)
    } finally {
      analyseEnCours.value = false
    }
  }

  function reinitialiser() {
    images.value = []
    notes.value = ''
    partsResultat.value = []
    modeleUtilise.value = ''
  }

  return {
    images,
    notes,
    analyseEnCours,
    partsResultat,
    dragActif,
    modeleUtilise,
    onDrop,
    onInputFile,
    analyserImage,
    supprimerImage,
    mettreAJourTF,
    reinitialiser,
    setDragActif: (v: boolean) => { dragActif.value = v },
  }
}
