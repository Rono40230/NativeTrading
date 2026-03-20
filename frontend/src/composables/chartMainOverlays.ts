import type { IChartApi, ISeriesApi, LineSeriesOptions } from 'lightweight-charts'
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
