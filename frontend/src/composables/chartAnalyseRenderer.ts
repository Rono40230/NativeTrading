import type { ContentPart } from './useChartImport'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface AnalyseSection {
  type: 'section' | 'table' | 'diagram'
  title?: string
  icon?: string
  html: string
  colorClass: string
  headerClass: string
}

// ── Thème couleur par titre ───────────────────────────────────────────────────

function sectionTheme(title: string): { colorClass: string; headerClass: string; icon: string } {
  const t = title.toLowerCase()
  if (t.includes('biais') || t.includes('structure') || t.includes('tendance'))
    return { colorClass: 'blue', headerClass: 'blue', icon: '📈' }
  if (t.includes('liquidit') || t.includes('sweep') || t.includes('inducement'))
    return { colorClass: 'yellow', headerClass: 'yellow', icon: '💧' }
  if (t.includes('poi') || t.includes('order block') || t.includes('fvg') || t.includes('ifvg') || t.includes('fib'))
    return { colorClass: 'orange', headerClass: 'orange', icon: '🎯' }
  if (t.includes('piège') || t.includes('risque') || t.includes('attention') || t.includes('warning'))
    return { colorClass: 'red', headerClass: 'red', icon: '⚠️' }
  if (t.includes('score') || t.includes('confluence') || t.includes('signal'))
    return { colorClass: 'green', headerClass: 'green', icon: '⭐' }
  if (t.includes('tp') || t.includes('sl') || t.includes('entrée') || t.includes('niveau') || t.includes('prix'))
    return { colorClass: 'purple', headerClass: 'purple', icon: '△' }
  if (t.includes('règle') || t.includes('direction') || t.includes('note'))
    return { colorClass: 'gray', headerClass: 'gray', icon: '📝' }
  return { colorClass: 'gray', headerClass: 'gray', icon: '📄' }
}

// ── buildSections ─────────────────────────────────────────────────────────────

export function buildSections(parts: ContentPart[]): AnalyseSection[] {
  const sections: AnalyseSection[] = []

  for (const part of parts) {
    if (part.type === 'diagram') {
      sections.push({ type: 'diagram', html: part.content, colorClass: 'blue', headerClass: 'blue' })
      continue
    }

    const text = part.content.trim()
    if (!text) continue

    const lines = text.split('\n')
    let currentTitle = ''
    let currentLines: string[] = []

    const flush = () => {
      const body = currentLines.join('\n').trim()
      if (!body && !currentTitle) return
      const hasTable = body.includes('|') && body.match(/^\s*\|/m)
      if (hasTable && !currentTitle) {
        sections.push({ type: 'table', html: renderMd(body), colorClass: 'gray', headerClass: 'gray' })
      } else {
        const theme = sectionTheme(currentTitle)
        sections.push({
          type: 'section',
          title: currentTitle || undefined,
          icon: currentTitle ? theme.icon : undefined,
          html: renderMd(body),
          colorClass: theme.colorClass,
          headerClass: theme.headerClass,
        })
      }
      currentTitle = ''
      currentLines = []
    }

    for (const line of lines) {
      const heading = line.match(/^#{1,3}\s+(.+)$/)
      if (heading) {
        flush()
        currentTitle = heading[1].replace(/\*+/g, '').trim()
      } else {
        currentLines.push(line)
      }
    }
    flush()
  }

  return sections
}

// ── renderMd ──────────────────────────────────────────────────────────────────

function extractTables(rawText: string): { withPlaceholders: string; tables: string[] } {
  const tables: string[] = []
  const lines = rawText.split('\n')
  const out: string[] = []
  let i = 0

  const isSepLine = (l: string) => /^\s*\|[\s\-:|]+\|\s*$/.test(l)
  const escCell = (s: string) =>
    s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  const cellsOf = (row: string) =>
    row.split('|').filter((_c, ci, arr) => ci > 0 && ci < arr.length - 1).map(c => escCell(c.trim()))

  const TH = 'padding:7px 12px;border:1px solid #21262d;background:#161b22;color:#b794f4;font-weight:700;text-align:left;white-space:nowrap;font-size:11px;text-transform:uppercase;letter-spacing:.5px'
  const TD = 'padding:6px 12px;border:1px solid #30363d;color:#e6edf3;text-align:left;vertical-align:middle'
  const TR_ODD = 'background:#0d1117'
  const TR_EVEN = 'background:#111827'

  while (i < lines.length) {
    const line = lines[i]
    if (line.trimStart().startsWith('|') && i + 1 < lines.length && isSepLine(lines[i + 1])) {
      const rows: string[] = []
      while (i < lines.length && lines[i].trimStart().startsWith('|')) {
        rows.push(lines[i++])
      }
      const dataRows = rows.filter(r => !isSepLine(r))
      const html =
        '<div style="overflow-x:auto;margin:8px 0">' +
        '<table style="width:100%;border-collapse:collapse;font-size:12px;border:1px solid #21262d;border-radius:8px;overflow:hidden">' +
        dataRows.map((row, idx) => {
          const tag = idx === 0 ? 'th' : 'td'
          const style = idx === 0 ? TH : TD
          const trStyle = idx === 0 ? '' : ` style="${idx % 2 === 1 ? TR_ODD : TR_EVEN}"`
          return `<tr${trStyle}>${cellsOf(row).map(c => `<${tag} style="${style}">${c}</${tag}>`).join('')}</tr>`
        }).join('') +
        '</table></div>'
      tables.push(html)
      out.push(`\x01TBL${tables.length - 1}\x01`)
    } else {
      out.push(lines[i++])
    }
  }

  return { withPlaceholders: out.join('\n'), tables }
}

export function renderMd(text: string): string {
  const { withPlaceholders, tables } = extractTables(text)

  const rendered = withPlaceholders
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/```([\s\S]*?)```/g, (_m, code) =>
      `<pre style="background:#161b22;border:1px solid #21262d;border-radius:8px;padding:12px;overflow-x:auto;font-family:monospace;font-size:12px;color:#7ee787;margin:6px 0">${code.trim()}</pre>`)
    .replace(/`([^`]+)`/g, '<code style="background:#161b22;padding:2px 6px;border-radius:4px;font-size:12px;color:#7ee787">$1</code>')
    .replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>')
    .replace(/\*\*(.+?)\*\*/g, '<strong style="color:#e6edf3">$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/^### (.+)$/gm, '<h3 style="font-size:13px;color:#b794f4;font-weight:700;margin:4px 0 1px">$1</h3>')
    .replace(/^## (.+)$/gm, '<h2 style="font-size:14px;color:#63b3ed;font-weight:700;margin:6px 0 1px">$1</h2>')
    .replace(/^# (.+)$/gm, '<h1 style="font-size:16px;color:#e6edf3;font-weight:700;margin:8px 0 2px">$1</h1>')
    .replace(/^---$/gm, '<hr style="border:none;border-top:1px solid #21262d;margin:5px 0"/>')
    .replace(/^- (.+)$/gm, '<div style="display:flex;gap:6px;margin:1px 0"><span style="color:#63b3ed;flex-shrink:0">•</span><span>$1</span></div>')
    .replace(/\n\n/g, '<br/>')
    .replace(/\n/g, ' ')

  return rendered.replace(/\x01TBL(\d+)\x01/g, (_, idx) => tables[Number(idx)])
}
