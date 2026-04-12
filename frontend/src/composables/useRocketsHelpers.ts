import type { PhaseRocket } from '@/composables/useVeilleRockets'

export function useRocketsHelpers() {
  function icone(phase: PhaseRocket) {
    return phase === 'breakout' ? '🚀' : phase === 'prelancement' ? '⚡' : '🌀'
  }

  function labelPhase(phase: PhaseRocket) {
    return phase === 'breakout' ? 'Breakout' : phase === 'prelancement' ? 'Pré-lancement' : 'Compression'
  }

  function classeScore(s: number) {
    return s >= 70 ? 'text-orange-400' : s >= 50 ? 'text-emerald-400' : 'text-gray-400'
  }

  function classeCoefTrailing(c: number | undefined) {
    return !c ? 'text-gray-500' : c >= 3.5 ? 'text-emerald-400' : c >= 2.0 ? 'text-blue-400' : 'text-gray-400'
  }

  function formatPrix(v: number) {
    return v >= 1000
      ? new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
      : v >= 1 ? v.toFixed(4) : v.toFixed(6)
  }

  function sparklinePath(closes: number[]) {
    const W = 160, H = 44, min = Math.min(...closes), max = Math.max(...closes), range = max - min || 1
    return closes.map((v, i) =>
      `${((i / (closes.length - 1)) * W).toFixed(1)},${(H - ((v - min) / range) * (H - 4) - 2).toFixed(1)}`
    ).join(' ')
  }

  function classeCarteSignal(phase: PhaseRocket | null) {
    if (phase === 'breakout') return 'border-emerald-500/30 bg-emerald-500/[0.04]'
    if (phase === 'prelancement') return 'border-yellow-500/30 bg-yellow-500/[0.04]'
    return 'border-blue-500/25 bg-blue-500/[0.04]'
  }

  function classeBadgePhase(phase: PhaseRocket | null) {
    if (phase === 'breakout') return 'bg-emerald-500/20 text-emerald-300'
    if (phase === 'prelancement') return 'bg-yellow-500/20 text-yellow-300'
    return 'bg-blue-500/20 text-blue-300'
  }

  function classeBadgeVerdict(verdict: 'long' | 'attendre' | 'eviter') {
    if (verdict === 'long') return 'bg-emerald-500/25 text-emerald-300 border border-emerald-500/30'
    if (verdict === 'attendre') return 'bg-yellow-500/20 text-yellow-300 border border-yellow-500/30'
    return 'bg-red-500/20 text-red-300 border border-red-500/30'
  }

  function labelVerdict(verdict: 'long' | 'attendre' | 'eviter') {
    if (verdict === 'long') return '✓ LONG imminent'
    if (verdict === 'attendre') return '⏳ Attendre'
    return '✕ Éviter'
  }

  return {
    icone, labelPhase, classeScore, classeCoefTrailing,
    formatPrix, sparklinePath, classeCarteSignal,
    classeBadgePhase, classeBadgeVerdict, labelVerdict,
  }
}
