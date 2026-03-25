export function formatDate(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' })
}

export function formatNombre(v: number | undefined): string {
  if (v === undefined || v === null) return '—'
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  if (v >= 1) return v.toFixed(4)
  return v.toFixed(6)
}

export function classeVerdictSignal(verdict: string | null): string {
  if (verdict === 'TP3' || verdict === 'TP2') return 'badge-green'
  if (verdict === 'TP1') return 'badge-blue'
  if (verdict === 'SL') return 'badge-red'
  if (verdict === 'expire') return 'badge-gray'
  return 'badge-yellow'
}

export function labelVerdictSignal(verdict: string | null): string {
  if (verdict === 'TP3') return '✅ TP3'
  if (verdict === 'TP2') return '✅ TP2'
  if (verdict === 'TP1') return '🟡 TP1'
  if (verdict === 'SL') return '❌ SL'
  if (verdict === 'expire') return '⏰ Expiré'
  return '⏳ En cours'
}
