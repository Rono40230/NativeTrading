import type { IChartApi, ISeriesApi, LineSeriesOptions } from 'lightweight-charts'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { ReponseIndicators } from '@/services/api.service'
import { COULEURS, hexVersRgba } from './chartIndicatorsConfig'

type Point = { time: number; value: number }
type AjouterLigneFn = (chart: IChartApi, data: Point[], couleur: string, largeur?: number) => ISeriesApi<'Line'>
type AjouterFantomeFn = (chart: IChartApi) => ISeriesApi<'Line'>
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

// ─── Overlays SMC (Order Blocks, FVG, IFVG, Fibonacci, BSL/SSL) ───────────────

export function appliquerSmcOverlays(
  chart: IChartApi,
  data: ReponseIndicators,
  ajouterFantome: AjouterFantomeFn,
): void {
  if (data.order_blocks?.length) {
    for (const ob of data.order_blocks) {
      const couleur = ob.direction === 'Long' ? COULEURS.ob_long : COULEURS.ob_short
      const f = ajouterFantome(chart)
      f.createPriceLine({ price: ob.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: `OB ${ob.direction}` })
      f.createPriceLine({ price: ob.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
    }
  }

  if (data.imbalances?.length) {
    for (const fvg of data.imbalances) {
      if (fvg.comble) continue
      const couleur = fvg.direction === 'Long' ? COULEURS.fvg_long : COULEURS.fvg_short
      const f = ajouterFantome(chart)
      f.createPriceLine({ price: fvg.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: 'FVG' })
      f.createPriceLine({ price: fvg.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
    }
  }

  if (data.ifvg?.length) {
    for (const i of data.ifvg) {
      const couleur = i.direction === 'Long' ? COULEURS.ifvg_long : COULEURS.ifvg_short
      const f = ajouterFantome(chart)
      f.createPriceLine({ price: i.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: 'IFVG' })
      f.createPriceLine({ price: i.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
    }
  }

  if (data.fibonacci) {
    const fib = data.fibonacci
    const f = ajouterFantome(chart)
    for (const [niveau, label] of [
      [fib.niveau_236, 'Fib 23.6%'],
      [fib.niveau_382, 'Fib 38.2%'],
      [fib.niveau_500, 'Fib 50%'],
      [fib.niveau_618, 'Fib 61.8%'],
      [fib.niveau_786, 'Fib 78.6%'],
    ] as [number, string][]) {
      f.createPriceLine({ price: niveau, color: COULEURS.fib, lineWidth: 1, lineStyle: 1, axisLabelVisible: true, title: label })
    }
  }

  if (data.liquidites?.length) {
    for (const liq of data.liquidites) {
      if (liq['sweepé']) continue
      const couleur = liq.cote === 'BSL' ? COULEURS.bsl : COULEURS.ssl
      const f = ajouterFantome(chart)
      f.createPriceLine({ price: liq.prix, color: couleur, lineWidth: 2, lineStyle: 0, axisLabelVisible: true, title: `${liq.cote}${liq.equal ? ' (EQ)' : ''}` })
    }
  }
}
