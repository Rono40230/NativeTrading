import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { ReponseIndicators } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'
import {
  type LigneLiq,
  type RectAsie,
  type LigneDeviation,
  dessinerLignesLiq,
  dessinerRangesAsie,
  dessinerDeviationsAsie,
} from './useSmcLiqCanvasDraw'

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
    const ts = chartRef.timeScale()
    dessinerLignesLiq(ctx, serieRef, ts, lignesRef, W, dernierTimestampRef)
    dessinerRangesAsie(ctx, serieRef, ts, rectsAsieRef, W)
    dessinerDeviationsAsie(ctx, serieRef, ts, deviationsRef, W, dernierTimestampRef)
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

    // ── Lignes EQH/EQL ───────────────────────────────────────────────────────
    if (prefs.smcLiquidites && data.liquidites?.length) {
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
    if (prefs.smcLiquidites && data.range_asie?.length && prefs.smcLiqAsieRangeActif) {
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
        if (prefs.smcLiqAsieDeviationsActif) {
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
    }

    // ── BOS / CHoCH — lignes tiretées au niveau cassé ─────────────────────────
    if (prefs.smcBos && data.bos) {
      const isBosLong = data.bos.direction === 'Long'
      lignesRef.push({
        prix:      data.bos.niveau_casse,
        timestamp: 0,
        couleur:   hexVersRgba(prefs.smcBosCouleur, 0.9),
        label:     isBosLong ? 'BOS ↑' : 'BOS ↓',
        pointille: true,
      })
    }
    if (prefs.smcChoch && data.choch) {
      lignesRef.push({
        prix:      data.choch.niveau_casse,
        timestamp: 0,
        couleur:   hexVersRgba(prefs.smcChochCouleur, 0.9),
        label:     data.choch.direction === 'Long' ? 'CHoCH ↑' : 'CHoCH ↓',
        pointille: true,
      })
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
