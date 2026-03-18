import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { ReponseIndicators } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'

interface ZoneRect {
  prixHaut: number
  prixBas: number
  timestamp: number        // Unix secondes — bord gauche
  timestampDroit?: number  // si défini : bord droit fixe (sinon = dernière bougie)
  couleurFond: string
  couleurBord: string
  label: string
}

/**
 * Gestion du canvas HTML superposé pour afficher les zones SMC (OB, FVG, IFVG)
 * sous forme de vrais rectangles semi-transparents, comme TradingView.
 *
 * Technique : canvas absolu sur le container chart, redessiné à chaque
 * changement de zoom/scroll via l'API LightWeight Charts.
 */
export function useSmcCanvas() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let zonesRef: ZoneRect[] = []
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null
  // Timestamp Unix (secondes) de la dernière bougie — bord droit des rectangles
  let dernierTimestampRef: number | null = null

  /** Injecte le canvas dans le container du chart (position absolute, couvre tout). */
  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top = '0'
    c.style.left = '0'
    c.style.width = '100%'
    c.style.height = '100%'
    c.style.pointerEvents = 'none'   // transparent aux clics — le chart reste interactif
    c.style.zIndex = '2'
    container.appendChild(c)
    return c
  }

  /** Synchronise les dimensions du canvas avec son container CSS. */
  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    // Lire les dimensions du container (plus fiable que canvas.offsetWidth avant le premier layout)
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width  = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  /** Redessine toutes les zones sur le canvas selon l'état courant du chart. */
  function redessiner() {
    if (!canvas || !chartRef || !serieRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)

    const timeScale = chartRef.timeScale()

    // Largeur de la zone de prix (échelle droite) — on dessine jusqu'au bord de la prixScale
    const scrollPos = timeScale.scrollPosition()

    for (const zone of zonesRef) {
      const yHaut = serieRef.priceToCoordinate(zone.prixHaut)
      const yBas  = serieRef.priceToCoordinate(zone.prixBas)

      // Les coordonnées peuvent être null si hors écran
      if (yHaut === null || yBas === null) continue

      const yTop    = Math.min(yHaut, yBas)
      const yBottom = Math.max(yHaut, yBas)
      const hauteur = yBottom - yTop

      if (hauteur < 1) continue

      // Bord gauche = timestamp formation (null si avant la plage visible → 0)
      const xGaucheRaw = timeScale.timeToCoordinate(zone.timestamp as any)
      const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
      // Bord droit = timestampDroit fixe si défini, sinon dernière bougie
      const xDroitSrc = zone.timestampDroit ?? dernierTimestampRef
      const xDroitRaw = xDroitSrc !== null && xDroitSrc !== undefined
        ? timeScale.timeToCoordinate(xDroitSrc as any)
        : null
      const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70
      if (xDroit <= xGauche) continue
      const largeur = xDroit - xGauche

      // Dessiner le rectangle depuis la bougie de formation jusqu'au bord droit
      ctx.fillStyle   = zone.couleurFond
      ctx.fillRect(xGauche, yTop, largeur, hauteur)

      // Bordure haute pleine
      ctx.strokeStyle = zone.couleurBord
      ctx.lineWidth   = 1.5
      ctx.beginPath()
      ctx.moveTo(xGauche, yTop)
      ctx.lineTo(xDroit, yTop)
      ctx.stroke()

      // Bordure basse pleine
      ctx.beginPath()
      ctx.moveTo(xGauche, yBottom)
      ctx.lineTo(xDroit, yBottom)
      ctx.stroke()

      // Bordure gauche verticale (marque l'origine de la zone)
      ctx.lineWidth = 2
      ctx.beginPath()
      ctx.moveTo(xGauche, yTop)
      ctx.lineTo(xGauche, yBottom)
      ctx.stroke()
    }

    // Eviter warning TS sur scrollPos inutilisé
    void scrollPos
  }

  /** Planifie un redessin via requestAnimationFrame (évite les surdessin). */
  function planifierRedessiner() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => {
      animFrame = null
      redessiner()
    })
  }

  /**
   * Initialise le canvas sur le container donné et s'abonne aux événements
   * de scroll/zoom du chart pour maintenir les zones à jour.
   */
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

    // ResizeObserver pour recalibrer en cas de redimensionnement du container
    const ro = new ResizeObserver(() => {
      redimensionner()
      planifierRedessiner()
    })
    ro.observe(container)
    // Stocker pour nettoyage
    ;(canvas as any).__ro = ro

    // Premier dessin planifié après layout (rAF garantit dimensions non-nulles)
    planifierRedessiner()
  }

  /**
   * Met à jour les zones à dessiner depuis les données SMC et les préférences.
   * À appeler après chaque rechargement d'indicateurs.
   */
  function mettreAJourZones(data: ReponseIndicators, prefs: PrefsIndicateurs, dernierTimestamp?: number) {
    zonesRef = []
    dernierTimestampRef = dernierTimestamp ?? null

    if (data.order_blocks?.length) {
      for (const ob of data.order_blocks) {
        const hex = ob.direction === 'Long' ? prefs.smcObCouleurLong : prefs.smcObCouleurShort
        zonesRef.push({
          prixHaut:    ob.prix_haut,
          prixBas:     ob.prix_bas,
          timestamp:   ob.timestamp,
          couleurFond: hexVersRgba(hex, prefs.smcObOpacite),
          couleurBord: hexVersRgba(hex, Math.min(prefs.smcObOpacite * 4, 1)),
          label:       ob.direction === 'Long' ? 'OB Haussier' : 'OB Baissier',
        })
      }
    }

    if (data.ifvg?.length) {
      for (const ifvg of data.ifvg) {
        // Couleurs : gauche = origine du FVG, droite = après inversion
        // Long (ex-bear inversé) : gauche rouge (bear), droite vert (haussier)
        // Short (ex-bull inversé) : gauche vert (bull), droite rouge (baissier)
        const hexGauche = ifvg.direction === 'Long' ? prefs.smcIfvgCouleurShort : prefs.smcIfvgCouleurLong
        const hexDroite = ifvg.direction === 'Long' ? prefs.smcIfvgCouleurLong : prefs.smcIfvgCouleurShort
        // Rect gauche : formation → inversion (couleur d'origine)
        zonesRef.push({
          prixHaut:       ifvg.prix_haut,
          prixBas:        ifvg.prix_bas,
          timestamp:      ifvg.timestamp,
          timestampDroit: ifvg.timestamp_inversion,
          couleurFond:    hexVersRgba(hexGauche, prefs.smcIfvgOpacite),
          couleurBord:    'transparent',
          label:          '',
        })
        // Rect droite : inversion → maintenant (couleur inversée)
        zonesRef.push({
          prixHaut:    ifvg.prix_haut,
          prixBas:     ifvg.prix_bas,
          timestamp:   ifvg.timestamp_inversion,
          couleurFond: hexVersRgba(hexDroite, prefs.smcIfvgOpacite),
          couleurBord: 'transparent',
          label:       ifvg.direction === 'Long' ? 'IFVG ↑' : 'IFVG ↓',
        })
      }
    }

    if (data.imbalance?.length) {
      for (const z of data.imbalance) {
        if (z.remplie) continue
        const isBull  = z.type_zone === 'FvgBull' || z.type_zone === 'OgBull'
        const isOg    = z.type_zone === 'OgBull'  || z.type_zone === 'OgBear'
        const hex     = isBull ? prefs.smcImbCouleurBull : prefs.smcImbCouleurBear
        const opacite = isOg ? Math.min(prefs.smcImbOpacite * 2.5, 1) : prefs.smcImbOpacite
        zonesRef.push({
          prixHaut:    z.haut,
          prixBas:     z.bas,
          timestamp:   z.timestamp,
          couleurFond: hexVersRgba(hex, opacite),
          couleurBord: 'transparent',
          label:       isOg ? 'OG' : '',
        })
      }
    }

    if (data.bpr?.length) {
      for (const z of data.bpr) {
        // BPR : FVG bullish (bleu) + FVG bearish (rouge) superposés → intersection visible
        zonesRef.push({
          prixHaut:    z.bull_haut,
          prixBas:     z.bull_bas,
          timestamp:   z.timestamp,
          couleurFond: hexVersRgba(prefs.smcBprCouleurBull, prefs.smcBprOpacite),
          couleurBord: hexVersRgba(prefs.smcBprCouleurBull, Math.min(prefs.smcBprOpacite * 3, 1)),
          label:       'BPR',
        })
        zonesRef.push({
          prixHaut:    z.bear_haut,
          prixBas:     z.bear_bas,
          timestamp:   z.timestamp,
          couleurFond: hexVersRgba(prefs.smcBprCouleurBear, prefs.smcBprOpacite),
          couleurBord: hexVersRgba(prefs.smcBprCouleurBear, Math.min(prefs.smcBprOpacite * 3, 1)),
          label:       '',
        })
      }
    }

    planifierRedessiner()
  }

  /** Efface toutes les zones sans détruire le canvas. */
  function effacerZones() {
    zonesRef = []
    if (canvas) {
      const ctx = canvas.getContext('2d')
      if (ctx) ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight)
    }
  }

  /** Détruit le canvas et désenregistre tous les abonnements. */
  function detruire() {
    if (animFrame !== null) {
      cancelAnimationFrame(animFrame)
      animFrame = null
    }
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
    zonesRef = []
  }

  return { initialiser, mettreAJourZones, effacerZones, detruire, redessiner: planifierRedessiner }
}
