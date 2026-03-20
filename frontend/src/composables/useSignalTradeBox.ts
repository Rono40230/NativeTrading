/**
 * useSignalTradeBox — overlay canvas Trade Box sur graphique TradingView.
 *
 * Dessine zones TP/SL (rectangles semi-transparents) pour le dernier signal
 * SMC Directionnel. Se met à jour lors de l'arrivée d'un nouveau signal WS.
 *
 * Pattern identique à useSmcCanvas : canvas absolu sur container chart,
 * redessiné à chaque changement zoom/scroll.
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { Signal } from '@/services/api.service'

// Couleurs palette premium (dark mode)
const COULEUR_SL   = 'rgba(239,68,68,0.15)'
const COULEUR_SL_B = '#ef4444'
const COULEUR_TP1  = 'rgba(16,185,129,0.15)'
const COULEUR_TP1_B = '#10b981'
const COULEUR_TP2  = 'rgba(52,211,153,0.08)'
const COULEUR_TP2_B = '#34d399'
const COULEUR_ENTRY = '#3b82f6'

export function useSignalTradeBox() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let signalRef: Signal | null = null
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null

  // ── Canvas lifecycle ────────────────────────────────────────────────────────

  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top = '0'
    c.style.left = '0'
    c.style.width = '100%'
    c.style.height = '100%'
    c.style.pointerEvents = 'none'
    c.style.zIndex = '3'
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
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  // ── Rendu ───────────────────────────────────────────────────────────────────

  function redessiner() {
    if (!canvas || !chartRef || !serieRef || !signalRef) {
      if (canvas) {
        const ctx = canvas.getContext('2d')
        if (ctx) ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight)
      }
      return
    }

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)

    const signal = signalRef
    const entry = signal.prix_entree
    const sl = signal.stop_loss
    let tps: number[] = []
    try {
      const parsed = JSON.parse(signal.take_profit)
      tps = Array.isArray(parsed) ? parsed.map(Number) : []
    } catch { return }

    if (tps.length === 0) return

    const tp1 = tps[0]
    const tp2 = tps[1] ?? null
    const isBull = signal.direction === 'Long'

    // Coordonnées Y
    const yEntry = serieRef.priceToCoordinate(entry)
    const ySl = serieRef.priceToCoordinate(sl)
    const yTp1 = serieRef.priceToCoordinate(tp1)
    const yTp2 = tp2 !== null ? serieRef.priceToCoordinate(tp2) : null

    if (yEntry == null || ySl == null || yTp1 == null) return

    const xGauche = 0
    const xDroit = W - 70 // laisser place à l'échelle de prix

    // Zone SL (entry → sl)
    const ySLTop = Math.min(yEntry, ySl)
    const ySLBot = Math.max(yEntry, ySl)
    ctx.fillStyle = COULEUR_SL
    ctx.fillRect(xGauche, ySLTop, xDroit - xGauche, ySLBot - ySLTop)
    ctx.strokeStyle = COULEUR_SL_B
    ctx.lineWidth = 1
    ctx.setLineDash([4, 4])
    ctx.strokeRect(xGauche, ySLTop, xDroit - xGauche, ySLBot - ySLTop)
    ctx.setLineDash([])

    // Zone TP1 (entry → tp1)
    const yTP1Top = Math.min(yEntry, yTp1)
    const yTP1Bot = Math.max(yEntry, yTp1)
    ctx.fillStyle = COULEUR_TP1
    ctx.fillRect(xGauche, yTP1Top, xDroit - xGauche, yTP1Bot - yTP1Top)
    ctx.strokeStyle = COULEUR_TP1_B
    ctx.lineWidth = 1
    ctx.setLineDash([4, 4])
    ctx.strokeRect(xGauche, yTP1Top, xDroit - xGauche, yTP1Bot - yTP1Top)
    ctx.setLineDash([])

    // Zone TP2 (tp1 → tp2) si disponible
    if (yTp2 !== null && tp2 !== null) {
      const yTP2Top = Math.min(yTp1, tp2 > tp1 ? yTp2 : yTp1)
      const yTP2Bot = Math.max(yTp1, tp2 > tp1 ? yTp2 : yTp1)
      ctx.fillStyle = COULEUR_TP2
      ctx.fillRect(xGauche, yTP2Top, xDroit - xGauche, yTP2Bot - yTP2Top)
      ctx.strokeStyle = COULEUR_TP2_B
      ctx.lineWidth = 1
      ctx.setLineDash([2, 6])
      ctx.strokeRect(xGauche, yTP2Top, xDroit - xGauche, yTP2Bot - yTP2Top)
      ctx.setLineDash([])
    }

    // Ligne Entry
    ctx.strokeStyle = COULEUR_ENTRY
    ctx.lineWidth = 1.5
    ctx.setLineDash([])
    ctx.beginPath()
    ctx.moveTo(xGauche, yEntry)
    ctx.lineTo(xDroit, yEntry)
    ctx.stroke()

    // Labels
    ctx.font = 'bold 11px monospace'
    ctx.textBaseline = 'middle'

    // Label SL
    const midSL = (ySLTop + ySLBot) / 2
    ctx.fillStyle = COULEUR_SL_B
    ctx.fillText(`SL ${sl.toFixed(4)}`, xGauche + 6, midSL)

    // Label Entry
    ctx.fillStyle = COULEUR_ENTRY
    ctx.fillText(`Entry ${entry.toFixed(4)}`, xGauche + 6, yEntry - 8)

    // Label TP1 + R:R
    const midTP1 = (yTP1Top + yTP1Bot) / 2
    const risk = Math.abs(entry - sl)
    const reward1 = Math.abs(tp1 - entry)
    const rr1 = risk > 0 ? (reward1 / risk).toFixed(1) : '?'
    ctx.fillStyle = COULEUR_TP1_B
    ctx.fillText(`TP1 ${tp1.toFixed(4)}  R:R ${rr1}`, xGauche + 6, midTP1)

    // Label TP2
    if (yTp2 !== null && tp2 !== null) {
      const reward2 = Math.abs(tp2 - entry)
      const rr2 = risk > 0 ? (reward2 / risk).toFixed(1) : '?'
      const midTP2 = yTp2 + (isBull ? -12 : 12)
      ctx.fillStyle = COULEUR_TP2_B
      ctx.fillText(`TP2 ${tp2.toFixed(4)}  R:R ${rr2}`, xGauche + 6, midTP2)
    }

    // Badge asset/direction en haut à droite
    const badge = `${signal.asset} ${signal.timeframe} ${signal.direction.toUpperCase()}`
    ctx.font = 'bold 12px sans-serif'
    ctx.fillStyle = isBull ? COULEUR_TP1_B : COULEUR_SL_B
    ctx.textAlign = 'right'
    ctx.fillText(badge, xDroit - 4, 18)
    ctx.textAlign = 'left'
  }

  function planifierRedessinage() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(redessiner)
  }

  // ── API publique ────────────────────────────────────────────────────────────

  /**
   * Initialise le canvas sur le container et s'abonne aux événements chart.
   * Appeler une seule fois quand le chart est monté.
   */
  function initialiser(
    container: HTMLElement,
    chart: IChartApi,
    serie: ISeriesApi<'Candlestick'>,
  ) {
    chartRef = chart
    serieRef = serie

    if (!canvas) {
      canvas = monterCanvas(container)
    }

    redimensionner()
    const observer = new ResizeObserver(() => { redimensionner(); planifierRedessinage() })
    observer.observe(container)

    const onZoom = () => planifierRedessinage()
    chart.timeScale().subscribeVisibleTimeRangeChange(onZoom)
    unsubscribe = () => {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(onZoom)
      observer.disconnect()
    }
  }

  /**
   * Met à jour le signal affiché et redessine immédiatement.
   * Passer `null` pour effacer le Trade Box.
   */
  function mettreAJourSignal(signal: Signal | null) {
    signalRef = signal
    planifierRedessinage()
  }

  /**
   * Nettoie le canvas et les abonnements. Appeler dans onUnmounted.
   */
  function detruire() {
    unsubscribe?.()
    unsubscribe = null
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    canvas?.remove()
    canvas = null
    chartRef = null
    serieRef = null
    signalRef = null
  }

  return { initialiser, mettreAJourSignal, detruire }
}
