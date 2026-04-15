export function formatDate(ts: number): string {
  const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short', timeZone: tz })
}

export function formatNombre(v: number | undefined): string {
  if (v === undefined || v === null) return '—'
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  if (v >= 1) return v.toFixed(4)
  return v.toFixed(6)
}

export function classeVerdictSignal(verdict: string | null): string {
  const v = verdict?.toLowerCase() ?? ''
  if (v === 'tp3' || v === 'tp2') return 'badge-green'
  if (v === 'tp1') return 'badge-blue'
  if (v === 'be') return 'badge-gray'
  if (v === 'sl') return 'badge-red'
  if (v === 'invalide') return 'badge-orange'
  if (v === 'expire') return 'badge-gray'
  return 'badge-yellow'
}

export function labelVerdictSignal(verdict: string | null): string {
  const v = verdict?.toLowerCase() ?? ''
  if (v === 'tp3') return '✅ TP3'
  if (v === 'tp2') return '✅ TP2 (SL→TP1)'
  if (v === 'tp1') return '✅ TP1 (SL→BE)'
  if (v === 'be')  return '⚪ BE (neutre)'
  if (v === 'sl')  return '❌ SL'
  if (v === 'invalide') return '↩️ Entrée non atteinte'
  if (v === 'expire') return '⏰ Expiré'
  return '⏳ En cours'
}
