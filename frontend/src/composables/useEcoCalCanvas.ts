import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { AnnonceCalendrier } from '@/services/api.types'

/** Marqueur dessiné sur le canvas — coordonnées en pixels */
export interface MarqueurEcoCal {
  x: number
  y: number
  annonce: AnnonceCalendrier
}

const DRAPEAUX: Record<string, string> = {
  USD: '🇺🇸', EUR: '🇪🇺', GBP: '🇬🇧', JPY: '🇯🇵',
  CAD: '🇨🇦', AUD: '🇦🇺', CHF: '🇨🇭', NZD: '🇳🇿',
  CNY: '🇨🇳', CNH: '🇨🇳',
}

const Y_MARQUEUR = 14  // px depuis le haut de la zone chart

function emojiDevise(devise: string): string {
  return DRAPEAUX[devise] ?? devise.slice(0, 2)
}

/**
 * Canvas overlay pour afficher les annonces économiques sur le graphique.
 * Même pattern que useSmcLiqCanvas — canvas absolu z-index 4 (au-dessus des autres).
 * Expose `marqueursSous(x, y)` pour le détection hover dans ChartsView.
 */
export function useEcoCalCanvas() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let containerRef: HTMLElement | null = null
  let annoncesRef: AnnonceCalendrier[] = []
  let marqueursCaches: MarqueurEcoCal[] = []
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null

  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top = '0'
    c.style.left = '0'
    c.style.width = '100%'
    c.style.height = '100%'
    c.style.pointerEvents = 'none'
    c.style.zIndex = '4'
    container.appendChild(c)
    return c
  }

  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width = w * ratio
    canvas.height = h * ratio
    // canvas.width = ... remet le ctx à l'identité → scale à appliquer après chaque reset
    const ctx = canvas.getContext('2d')
    if (ctx) { ctx.resetTransform(); ctx.scale(ratio, ratio) }
  }

  function redessiner() {
    if (!canvas || !chartRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    if (W === 0 || H === 0) return
    ctx.clearRect(0, 0, W, H)
    marqueursCaches = []

    const ts = chartRef.timeScale()

    // Bord droit de la zone de tracé (inclut le rightbar, exclut la barre de prix)
    const logicalRange = ts.getVisibleLogicalRange()
    const xLimiteDroite = logicalRange !== null
      ? (ts.logicalToCoordinate(logicalRange.to) ?? W)
      : W

    // Interpolation linéaire pour convertir tout timestamp (y compris futur) en pixel X.
    // timeToCoordinate() retourne null pour les timestamps sans donnée (futur) ;
    // on extrapole depuis la plage visible connue.
    const visibleRange = ts.getVisibleRange()
    if (!visibleRange) return
    const tsFrom = visibleRange.from as number
    const tsTo = visibleRange.to as number
    const xFrom = ts.timeToCoordinate(visibleRange.from as any)
    const xTo = ts.timeToCoordinate(visibleRange.to as any)
    if (xFrom === null || xTo === null || tsTo === tsFrom) return
    const pixelsParSeconde = (xTo - xFrom) / (tsTo - tsFrom)
    const xFromNum = xFrom

    function tsVersX(tsSec: number): number {
      const direct = ts.timeToCoordinate(tsSec as any)
      if (direct !== null) return direct
      // extrapolation : position future dans le rightbar
      return xFromNum + (tsSec - tsFrom) * pixelsParSeconde
    }

    for (const annonce of annoncesRef) {
      const tsSec = Math.floor(new Date(annonce.date_heure).getTime() / 1000)
      const x = Math.round(tsVersX(tsSec))
      if (x < 0 || x > xLimiteDroite) continue

      const y = Y_MARQUEUR
      const estHaut = annonce.impact === 'High'

      // Ligne verticale pointillée
      ctx.save()
      ctx.setLineDash([3, 4])
      ctx.strokeStyle = estHaut ? 'rgba(239,68,68,0.35)' : 'rgba(251,146,60,0.35)'
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.moveTo(x, y + 16)
      ctx.lineTo(x, H)
      ctx.stroke()
      ctx.restore()

      // Fond de la pastille
      ctx.save()
      ctx.fillStyle = estHaut ? 'rgba(239,68,68,0.85)' : 'rgba(251,146,60,0.85)'
      ctx.beginPath()
      ctx.roundRect(x - 10, 0, 20, 16, 4)
      ctx.fill()
      ctx.restore()

      // Emoji drapeau
      ctx.font = '11px sans-serif'
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      ctx.fillText(emojiDevise(annonce.devise), x, y)

      marqueursCaches.push({ x, y, annonce })
    }
  }

  function planifierRedessiner() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => { animFrame = null; redessiner() })
  }

  function initialiser(chart: IChartApi, container: HTMLElement) {
    detruire()
    chartRef = chart
    canvas = monterCanvas(container)
    redimensionner()

    const handler = () => planifierRedessiner()
    chart.timeScale().subscribeVisibleTimeRangeChange(handler)
    chart.timeScale().subscribeVisibleLogicalRangeChange(handler)
    unsubscribe = () => {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(handler)
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(handler)
    }

    const ro = new ResizeObserver(() => { redimensionner(); planifierRedessiner() })
    ro.observe(container)
    ;(canvas as any).__ro = ro

    planifierRedessiner()
  }

  /** Met à jour la liste des annonces et redessine. */
  function mettreAJour(annonces: AnnonceCalendrier[]) {
    annoncesRef = annonces
    planifierRedessiner()
  }

  /**
   * Retourne l'annonce sous le curseur (distance < 12px),
   * ou null si aucun marqueur à proximité.
   */
  function marqueurSous(cursorX: number, cursorY: number): AnnonceCalendrier | null {
    if (cursorY > Y_MARQUEUR + 10) return null
    for (const m of marqueursCaches) {
      if (Math.abs(m.x - cursorX) < 12) return m.annonce
    }
    return null
  }

  function detruire() {
    if (animFrame !== null) { cancelAnimationFrame(animFrame); animFrame = null }
    unsubscribe?.()
    unsubscribe = null
    if (canvas) {
      ;(canvas as any).__ro?.disconnect()
      canvas.parentElement?.removeChild(canvas)
      canvas = null
    }
    chartRef = null
    containerRef = null
    annoncesRef = []
    marqueursCaches = []
  }

  return { initialiser, mettreAJour, marqueurSous, detruire }
}
