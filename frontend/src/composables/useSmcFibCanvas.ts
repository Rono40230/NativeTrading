import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { NiveauxFibonacci } from '@/services/api.types'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'

/**
 * Canvas dédié à l'affichage des segments Fibonacci.
 * Les niveaux sont dessinés comme des segments ancrés au timestamp du swing
 * (pas des lignes infinies), avec golden zone rectangulaire.
 */
export function useSmcFibCanvas() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null
  let fibRef: NiveauxFibonacci | null = null
  let prefsRef: PrefsIndicateurs | null = null
  let dernierTimestamp: number | null = null

  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.cssText = 'position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:4;'
    container.appendChild(c)
    return c
  }

  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width  = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  function redessiner() {
    if (!canvas || !chartRef || !serieRef || !fibRef || !prefsRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)

    const fib   = fibRef
    const prefs = prefsRef
    const ts    = chartRef.timeScale()

    // En mode haussier : 0% = bas, 100% = haut. En mode baissier : inversé.
    const haussier  = prefs.smcFibSensHaussier
    const prixZero  = haussier ? fib.swing_bas  : fib.swing_haut
    const prixUn    = haussier ? fib.swing_haut : fib.swing_bas
    // L'ancrage gauche est toujours le début réel du swing (timestamp le plus ancien),
    // indépendamment de la préférence d'affichage haussier/baissier.
    const tsAncrage = Math.min(fib.timestamp_haut, fib.timestamp_bas)

    // Bord droit = dernière bougie visible
    const xDroitRaw = dernierTimestamp !== null
      ? ts.timeToCoordinate(dernierTimestamp as any)
      : null
    const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70

    const xGaucheRaw = ts.timeToCoordinate(tsAncrage as any)
    const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0

    if (xDroit <= xGauche) return

    // Définition des 5 niveaux : [ratio, prix, couleur, label]
    const niveaux: [number, number, string, string][] = [
      [0,     prixZero,       prefs.smcFibCouleur0,   '0%'],
      [0.5,   fib.niveau_500, prefs.smcFibCouleur500, '50%'],
      [0.618, fib.niveau_618, prefs.smcFibCouleur618, '61.8%'],
      [0.786, fib.niveau_786, prefs.smcFibCouleur786, '78.6%'],
      [1,     prixUn,         prefs.smcFibCouleur1,   '100%'],
    ]

    // ── Golden Zone (rectangle 50%→61.8%) ────────────────────────────────
    if (prefs.smcFibGoldenZone) {
      const y500 = serieRef.priceToCoordinate(fib.niveau_500)
      const y618 = serieRef.priceToCoordinate(fib.niveau_618)
      if (y500 !== null && y618 !== null) {
        const yTop    = Math.min(y500, y618)
        const yBottom = Math.max(y500, y618)
        ctx.fillStyle = hexVersRgba(prefs.smcFibGoldenCouleur, prefs.smcFibGoldenOpacite)
        ctx.fillRect(xGauche, yTop, xDroit - xGauche, yBottom - yTop)
      }
    }

    // ── Segments de niveaux ───────────────────────────────────────────────
    for (const [, prix, couleur, label] of niveaux) {
      const y = serieRef.priceToCoordinate(prix)
      if (y === null || y < 0 || y > H) continue

      ctx.strokeStyle = couleur
      ctx.lineWidth   = 1
      ctx.setLineDash([])
      ctx.beginPath()
      ctx.moveTo(xGauche, y)
      ctx.lineTo(xDroit, y)
      ctx.stroke()

      // Label à droite du segment
      ctx.fillStyle = couleur
      ctx.font      = '10px monospace'
      ctx.textAlign = 'left'
      ctx.fillText(label, xDroit + 4, y + 3)
    }

    // ── Trait vertical d'ancrage au swing ─────────────────────────────────
    const yZero = serieRef.priceToCoordinate(prixZero)
    const yUn   = serieRef.priceToCoordinate(prixUn)
    if (yZero !== null && yUn !== null) {
      ctx.strokeStyle = prefs.smcFibCouleur0
      ctx.lineWidth   = 1.5
      ctx.setLineDash([])
      ctx.beginPath()
      ctx.moveTo(xGauche, Math.min(yZero, yUn))
      ctx.lineTo(xGauche, Math.max(yZero, yUn))
      ctx.stroke()
    }

    void H
  }

  function planifierRedessiner() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => { animFrame = null; redessiner() })
  }

  function initialiser(chart: IChartApi, serie: ISeriesApi<'Candlestick'>, container: HTMLElement) {
    detruire()
    chartRef    = chart
    serieRef    = serie
    canvas      = monterCanvas(container)
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

  function mettreAJour(fib: NiveauxFibonacci | undefined, prefs: PrefsIndicateurs, dernierTs?: number) {
    fibRef          = fib ?? null
    prefsRef        = prefs
    dernierTimestamp = dernierTs ?? null
    planifierRedessiner()
  }

  function effacer() {
    fibRef = null
    if (canvas) {
      const ctx = canvas.getContext('2d')
      if (ctx) ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight)
    }
  }

  function detruire() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    unsubscribe?.()
    if (canvas) {
      ;(canvas as any).__ro?.disconnect()
      canvas.remove()
    }
    canvas = null; chartRef = null; serieRef = null; containerRef = null
    unsubscribe = null; animFrame = null; fibRef = null; prefsRef = null
  }

  return { initialiser, mettreAJour, effacer, detruire }
}
