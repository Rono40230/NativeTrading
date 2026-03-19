import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { ReponseIndicators } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'

interface LigneLiq {
  prix: number
  timestamp: number    // Unix secondes — bord gauche (formation)
  couleur: string
  label: string        // 'EQH' | 'EQL' | ''
}

interface RectAsie {
  timestampDebut: number
  timestampFin: number   // bord droit fixe (fin de session)
  haut: number
  bas: number
  couleurFond: string
  couleurBord: string
}

interface LigneDeviation {
  prix: number
  timestampDebut: number  // même origine que la session
  couleur: string
  label: string           // ex: '+1', '+2', '-1', '-2'
}

/**
 * Canvas dédié aux niveaux de liquidité : lignes horizontales partant de la
 * bougie de formation jusqu'à la dernière bougie (comme OB/IFVG/Imbalance).
 * Label EQH/EQL centré sur la ligne pour les Equal Highs/Lows.
 * z-index 3 — au-dessus du canvas SMC zones (z-index 2).
 */
export function useSmcLiqCanvas() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let lignesRef: LigneLiq[] = []
  let rectsAsieRef: RectAsie[] = []
  let deviationsRef: LigneDeviation[] = []
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null
  let dernierTimestampRef: number | null = null

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
    canvas.width  = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  function redessiner() {
    if (!canvas || !chartRef || !serieRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)

    const timeScale = chartRef.timeScale()

    for (const ligne of lignesRef) {
      const y = serieRef.priceToCoordinate(ligne.prix)
      if (y === null) continue

      const xGaucheRaw = timeScale.timeToCoordinate(ligne.timestamp as any)
      const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0

      const xDroitRaw = dernierTimestampRef !== null
        ? timeScale.timeToCoordinate(dernierTimestampRef as any)
        : null
      const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70

      ctx.strokeStyle = ligne.couleur

      // Ligne horizontale + tick gauche (seulement si la ligne a une largeur)
      if (xDroit > xGauche) {
        ctx.lineWidth = 1.5
        ctx.beginPath()
        ctx.moveTo(xGauche, y)
        ctx.lineTo(xDroit, y)
        ctx.stroke()

        ctx.lineWidth = 2
        ctx.beginPath()
        ctx.moveTo(xGauche, y - 4)
        ctx.lineTo(xGauche, y + 4)
        ctx.stroke()
      }

      // Label EQH / EQL toujours dessiné au bord gauche visible
      if (ligne.label) {
        const xLabel = Math.max(Math.min(xGauche, W - 60), 4)
        ctx.font = 'bold 10px sans-serif'
        ctx.fillStyle = ligne.couleur
        ctx.textAlign = 'left'
        ctx.textBaseline = 'bottom'
        ctx.fillText(ligne.label, xLabel + 3, y - 2)
      }
    }

    // ── Rectangles range Asie ─────────────────────────────────────────────────
    for (const rect of rectsAsieRef) {
      const yHaut = serieRef.priceToCoordinate(rect.haut)
      const yBas  = serieRef.priceToCoordinate(rect.bas)
      if (yHaut === null || yBas === null) continue
      const yTop    = Math.min(yHaut, yBas)
      const yBottom = Math.max(yHaut, yBas)
      const hauteur = yBottom - yTop
      if (hauteur < 1) continue

      const xGaucheRaw = timeScale.timeToCoordinate(rect.timestampDebut as any)
      const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
      const xDroitRaw = timeScale.timeToCoordinate(rect.timestampFin as any)
      const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70
      if (xDroit <= xGauche) continue

      // Fond
      ctx.fillStyle = rect.couleurFond
      ctx.fillRect(xGauche, yTop, xDroit - xGauche, hauteur)
      // Bords haut/bas
      ctx.strokeStyle = rect.couleurBord
      ctx.lineWidth = 1
      ctx.beginPath(); ctx.moveTo(xGauche, yTop);    ctx.lineTo(xDroit, yTop);    ctx.stroke()
      ctx.beginPath(); ctx.moveTo(xGauche, yBottom); ctx.lineTo(xDroit, yBottom); ctx.stroke()
      // Bord gauche
      ctx.lineWidth = 2
      ctx.beginPath(); ctx.moveTo(xGauche, yTop); ctx.lineTo(xGauche, yBottom); ctx.stroke()
    }

    // ── Lignes de déviations Asie ─────────────────────────────────────────────
    for (const dev of deviationsRef) {
      const y = serieRef.priceToCoordinate(dev.prix)
      if (y === null) continue

      const xGaucheRaw = timeScale.timeToCoordinate(dev.timestampDebut as any)
      const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
      const xDroitRaw = dernierTimestampRef !== null
        ? timeScale.timeToCoordinate(dernierTimestampRef as any)
        : null
      const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70

      ctx.strokeStyle = dev.couleur
      ctx.lineWidth = 1
      ctx.setLineDash([4, 4])
      if (xDroit > xGauche) {
        ctx.beginPath(); ctx.moveTo(xGauche, y); ctx.lineTo(xDroit, y); ctx.stroke()
      }
      ctx.setLineDash([])

      if (dev.label) {
        const xLabel = Math.max(Math.min(xGauche, W - 60), 4)
        ctx.font = '9px sans-serif'
        ctx.fillStyle = dev.couleur
        ctx.textAlign = 'left'
        ctx.textBaseline = 'bottom'
        ctx.fillText(dev.label, xLabel + 3, y - 2)
      }
    }

    void H
  }

  function planifierRedessiner() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => { animFrame = null; redessiner() })
  }

  function initialiser(chart: IChartApi, serie: ISeriesApi<'Candlestick'>, container: HTMLElement) {
    detruire()
    chartRef = chart
    serieRef = serie
    canvas   = monterCanvas(container)
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

  /** Met à jour les lignes liquidités depuis les données SMC et les préférences. */
  function mettreAJour(data: ReponseIndicators, prefs: PrefsIndicateurs, dernierTimestamp?: number) {
    lignesRef = []
    rectsAsieRef = []
    deviationsRef = []
    dernierTimestampRef = dernierTimestamp ?? null

    if (!prefs.smcLiquidites) {
      planifierRedessiner()
      return
    }

    // ── Lignes EQH/EQL ───────────────────────────────────────────────────────
    if (data.liquidites?.length) {
      for (const liq of data.liquidites) {
        if (liq.swepe) continue
        if (liq.categorie === 'swing' && !prefs.smcLiqSwingsActif) continue

        let hex: string
        switch (liq.categorie) {
          case 'asie':  hex = prefs.smcLiqCouleurAsie; break
          case 'daily': hex = prefs.smcLiqCouleurDwm;  break
          default:      hex = liq.cote === 'BSL' ? prefs.smcLiqCouleurBsl : prefs.smcLiqCouleurSsl
        }

        lignesRef.push({
          prix:      liq.prix,
          timestamp: liq.timestamp,
          couleur:   hexVersRgba(hex, 0.9),
          label:     liq.cote === 'BSL' ? 'EQH' : 'EQL',
        })
      }
    }

    // ── Rectangles range Asie + déviations ───────────────────────────────────
    if (data.range_asie?.length && prefs.smcLiqAsieRangeActif) {
      const couleurBord = hexVersRgba(prefs.smcLiqAsieCouleur, 0.8)
      const couleurFond = hexVersRgba(prefs.smcLiqAsieCouleur, prefs.smcLiqAsieOpacite)
      for (const r of data.range_asie) {
        rectsAsieRef.push({
          timestampDebut: r.timestamp_debut,
          timestampFin:   r.timestamp_fin,
          haut:           r.haut,
          bas:            r.bas,
          couleurFond,
          couleurBord,
        })
        for (const dev of r.deviations) {
          const label = dev.direction === 'H' ? `+${dev.numero}` : `-${dev.numero}`
          deviationsRef.push({
            prix:           dev.prix,
            timestampDebut: r.timestamp_debut,
            couleur:        hexVersRgba(prefs.smcLiqAsieCouleur, 0.6),
            label,
          })
        }
      }
    }

    planifierRedessiner()
  }

  function effacer() {
    lignesRef = []
    rectsAsieRef = []
    deviationsRef = []
    if (canvas) {
      const ctx = canvas.getContext('2d')
      if (ctx) ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight)
    }
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
    serieRef = null
    containerRef = null
    lignesRef = []
    rectsAsieRef = []
    deviationsRef = []
  }

  return { initialiser, mettreAJour, effacer, detruire }
}
