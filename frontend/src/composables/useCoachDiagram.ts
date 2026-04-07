import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

type DiagramPart = { type: 'diagram'; content: string }
type ContentPart = { type: 'text' | 'diagram' | 'suggestion'; content: string }

export const diagrammesGeneres = ref<DiagramPart[]>([])
export const diagramChargement = ref(false)

export function parseContent(text: string): ContentPart[] {
  const parts: ContentPart[] = []
  const regex = /<htmldiagram>([\s\S]*?)<\/htmldiagram>|```html\s*([\s\S]*?)```|<suggest_diagram>([\s\S]*?)<\/suggest_diagram>/g
  let last = 0
  let match
  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) parts.push({ type: 'text', content: text.slice(last, match.index) })
    if (match[1] != null || match[2] != null)
      parts.push({ type: 'diagram', content: match[1] ?? match[2] })
    else if (match[3] != null)
      parts.push({ type: 'suggestion', content: match[3].trim() })
    last = regex.lastIndex
  }
  if (last < text.length) parts.push({ type: 'text', content: text.slice(last) })
  return parts
}

export function buildSrcdoc(html: string, id: string): string {
  const darkCss = `<style>html,body{background:#0d1117!important;color:#e6edf3!important;font-family:'Inter',-apple-system,sans-serif!important;margin:0!important;padding:8px!important;box-sizing:border-box!important}</style>`
  const resizeJs = `<scr` + `ipt>var _id='${id}';function _s(){var h=Math.max(document.body.scrollHeight,document.documentElement.scrollHeight);window.parent.postMessage({type:'resize',id:_id,height:h},'*')}window.addEventListener('load',function(){_s();setTimeout(_s,200);setTimeout(_s,600);setTimeout(_s,1200)});try{new ResizeObserver(_s).observe(document.body)}catch(e){}<` + `/script>`
  const trimmed = html.trim()
  if (/^<!doctype/i.test(trimmed) || /^<html/i.test(trimmed)) {
    let doc = trimmed.includes('</body>') ? trimmed : trimmed + '</body></html>'
    doc = doc.includes('</head>') ? doc.replace('</head>', darkCss + '</head>') : darkCss + doc
    return doc.replace(/(<\/body>)/i, resizeJs + '$1')
  }
  return `<!DOCTYPE html><html><head><meta charset="utf-8">${darkCss}</head><body>${html}${resizeJs}</body></html>`
}

export async function genererDiagram(sujet: string): Promise<void> {
  const alerteStore = useAlerteStore()
  diagramChargement.value = true
  try {
    const res = await apiService.genererDiagramme(sujet)
    const parts = parseContent(res.reponse)
    const html = parts.find(p => p.type === 'diagram')
    diagrammesGeneres.value.push({ type: 'diagram', content: html?.content ?? res.reponse })
  } catch (e: unknown) {
    alerteStore.afficherErreur((e as Error).message)
  } finally {
    diagramChargement.value = false
  }
}
