import type { IChartApi, ISeriesApi, IPriceLine, LineSeriesOptions } from 'lightweight-charts'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { ReponseIndicators } from '@/services/api.service'
import { COULEURS, hexVersRgba } from './chartIndicatorsConfig'

type Point = { time: number; value: number }
type AjouterLigneFn = (chart: IChartApi, data: Point[], couleur: string, largeur?: number) => ISeriesApi<'Line'>
type PushSeriesFn = (s: ISeriesApi<any>) => void

// ─── Bollinger Bands (overlay sur graphique principal) ────────────────────────

export function appliquerBollinger(
  chart: IChartApi,
  bollinger: NonNullable<ReponseIndicators['bollinger']>,
  prefs: PrefsIndicateurs,
  ajouterLigne: AjouterLigneFn,
  pushSeries: PushSeriesFn,
): void {
  const couleurHaute  = prefs.bollingerCouleurHaute  ?? COULEURS.bollingerHaute
  const couleurMilieu = prefs.bollingerCouleurMilieu ?? COULEURS.bollingerMilieu
  const couleurBasse  = prefs.bollingerCouleurBasse  ?? COULEURS.bollingerBasse

  const upperArea = chart.addAreaSeries({
    lineColor: couleurHaute,
    topColor: hexVersRgba(couleurHaute, 0.18),
    bottomColor: 'rgba(0,0,0,0)',
    lineWidth: 1 as 1,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
  } as any)
  upperArea.setData(bollinger.haute.map((p) => ({ time: p.time as any, value: p.value })))
  pushSeries(upperArea)

  ajouterLigne(chart, bollinger.basse, couleurBasse, 1)

  const milieu = chart.addLineSeries({
    color: couleurMilieu,
    lineWidth: 1 as 1,
    lineStyle: 1,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
  } as Partial<LineSeriesOptions>)
  milieu.setData(bollinger.milieu.map((p) => ({ time: p.time as any, value: p.value })))
  pushSeries(milieu)
}

// ─── Overlays SMC (Fibonacci + BSL/SSL comme price lines ; OB/FVG/IFVG via canvas) ──

/**
 * Applique les overlays SMC sur la candleSerie sous forme de price lines.
 * Seuls Fibonacci et BSL/SSL utilisent des lignes (ce sont des niveaux horizontaux par nature).
 * OB / FVG / IFVG sont désormais dessinés comme rectangles via useSmcCanvas.
 * Retourne la liste des IPriceLine créées pour permettre leur suppression propre.
 */
export function appliquerSmcOverlays(
  candleSerie: ISeriesApi<'Candlestick'>,
  data: ReponseIndicators,
  prefs: PrefsIndicateurs,
): IPriceLine[] {
  const lignes: IPriceLine[] = []

  if (data.fibonacci) {
    const fib     = data.fibonacci
    const couleur = hexVersRgba(prefs.smcFibCouleur, 0.85)
    const niveaux: [number, string, boolean][] = [
      [fib.niveau_236, 'Fib 23.6%', prefs.smcFibAfficher236],
      [fib.niveau_382, 'Fib 38.2%', true],
      [fib.niveau_500, 'Fib 50%',   true],
      [fib.niveau_618, 'Fib 61.8%', true],
      [fib.niveau_786, 'Fib 78.6%', prefs.smcFibAfficher786],
    ]
    for (const [niveau, label, visible] of niveaux) {
      if (!visible) continue
      lignes.push(candleSerie.createPriceLine({ price: niveau, color: couleur, lineWidth: 1, lineStyle: 1, axisLabelVisible: true, title: label }))
    }
  }

  if (data.liquidites?.length) {
    for (const liq of data.liquidites) {
      if (liq.swepe) continue
      if (liq.categorie === 'swing' && !prefs.smcLiqSwingsActif) continue
      let hex: string
      switch (liq.categorie) {
        case 'asie':   hex = prefs.smcLiqCouleurAsie; break
        case 'london': hex = prefs.smcLiqCouleurLondon; break
        case 'ny':     hex = prefs.smcLiqCouleurNY; break
        case 'lc':     hex = prefs.smcLiqCouleurLC; break
        case 'daily':  hex = prefs.smcLiqCouleurDwm; break
        default:       hex = liq.cote === 'BSL' ? prefs.smcLiqCouleurBsl : prefs.smcLiqCouleurSsl
      }
      const couleur = hexVersRgba(hex, 0.9)
      const side = liq.cote === 'BSL' ? 'H' : 'L'
      const labelSuffix = liq.equal ? ' (EQ)' : ''
      const labelCat = liq.categorie === 'swing'
        ? (liq.cote === 'BSL' ? 'High' : 'Low')
        : liq.categorie === 'daily' ? `D ${side}`
        : `${liq.categorie.charAt(0).toUpperCase() + liq.categorie.slice(1)} ${side}`
      lignes.push(candleSerie.createPriceLine({
        price: liq.prix, color: couleur, lineWidth: 2, lineStyle: 0,
        axisLabelVisible: true, title: `${labelCat}${labelSuffix}`,
      }))
    }
  }

  return lignes
}
