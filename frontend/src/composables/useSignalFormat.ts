import { dateHeureParis } from '@/utils/date'

export function formatDate(ts: number): string {
  if (!ts) return '—'
  return dateHeureParis(ts)
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

export function calculerR(signal: { direction: string; prix_entree: number; stop_loss: number; prix_verdict: number | null }): number | null {
  if (!signal.prix_verdict || !signal.prix_entree || !signal.stop_loss) return null;
  const diffPx = signal.prix_entree - signal.stop_loss;
  if (Math.abs(diffPx) < 0.000001) return null; // Évite la division par zéro

  const risk = Math.abs(diffPx);
  const isLong = signal.direction.toUpperCase() === 'LONG';
  const pnl = isLong
    ? (signal.prix_verdict - signal.prix_entree)
    : (signal.prix_entree - signal.prix_verdict);

  return pnl / risk;
}

export function formatR(r: number | null): string {
  if (r === null) return '';
  const sign = r > 0 ? '+' : '';
  return `${sign}${r.toFixed(2)}R`;
}

export function classeR(r: number | null): string {
  if (r === null) return '';
  if (r > 0) return 'text-emerald-400 font-bold';
  if (r < 0) return 'text-red-400 font-bold';
  return 'text-gray-400 font-bold';
}
