// ─── Couleurs des overlays ────────────────────────────────────────────────────
export const COULEURS = {
  ema: '#f59e0b',
  bollingerHaute: '#6366f1',
  bollingerMilieu: '#818cf8',
  bollingerBasse: '#6366f1',
  ob_long: 'rgba(16, 185, 129, 0.25)',
  ob_short: 'rgba(239, 68, 68, 0.25)',
  fvg_long: 'rgba(59, 130, 246, 0.20)',
  fvg_short: 'rgba(245, 158, 11, 0.20)',
  ifvg_long: 'rgba(99, 102, 241, 0.25)',
  ifvg_short: 'rgba(236, 72, 153, 0.25)',
  fib: 'rgba(148, 163, 184, 0.6)',
  bsl: 'rgba(16, 185, 129, 0.8)',
  ssl: 'rgba(239, 68, 68, 0.8)',
}

/** Convertit un hex #rrggbb en rgba(r,g,b,alpha) */
export function hexVersRgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return `rgba(${r},${g},${b},${alpha})`
}

import { TickMarkType } from 'lightweight-charts'

function tickMarkFormatterParis(ts: number, markType: TickMarkType): string {
  const d = new Date(ts * 1000)
  if (markType <= TickMarkType.DayOfMonth) {
    return new Intl.DateTimeFormat('fr-FR', {
      timeZone: 'Europe/Paris', day: '2-digit', month: '2-digit',
    }).format(d)
  }
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'Europe/Paris', hour: '2-digit', minute: '2-digit',
  }).format(d)
}

/** Options communes à tous les sous-graphiques lightweight-charts */
export function creerOptionsSousGraphique(container: HTMLElement) {
  return {
    layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
    grid: {
      vertLines: { color: 'rgba(255,255,255,0.05)' },
      horzLines: { visible: false },
    },
    crosshair: { mode: 1, horzLine: { visible: false } },
    rightPriceScale: { borderColor: 'rgba(255,255,255,0.1)', minimumWidth: 80 },
    timeScale: {
      borderColor: 'rgba(255,255,255,0.1)',
      timeVisible: true,
      secondsVisible: false,
      tickMarkFormatter: tickMarkFormatterParis,
    },
    width: container.clientWidth,
    height: container.clientHeight,
  }
}
