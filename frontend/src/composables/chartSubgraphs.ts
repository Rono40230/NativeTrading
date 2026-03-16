import { createChart } from 'lightweight-charts'
import type { IChartApi, LineSeriesOptions } from 'lightweight-charts'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { creerOptionsSousGraphique } from './chartIndicatorsConfig'

/** Contexte de synchronisation partagé entre le graphique principal et les sous-graphiques.
 *  Utiliser un objet (pas des primitives) pour que les mutations soient visibles par toutes les callbacks. */
export interface SyncCtx {
  syncing: boolean
  initialized: boolean
}

type Point = { time: number; value: number }

// ─── RSI ──────────────────────────────────────────────────────────────────────

export function creerSousGraphiqueRsi(
  container: HTMLElement,
  rsiData: Point[],
  prefs: PrefsIndicateurs,
  mainChart: IChartApi,
  ctx: SyncCtx,
): { chart: IChartApi; syncFromMain: (range: any) => void } {
  const chart = createChart(container, creerOptionsSousGraphique(container))
  const serie = chart.addLineSeries({
    color: prefs.rsiCouleur || '#a855f7',
    lineWidth: 1 as 1,
    crosshairMarkerVisible: false,
    lastValueVisible: true,
    priceLineVisible: false,
  } as Partial<LineSeriesOptions>)
  serie.setData(rsiData.map((p) => ({ time: p.time as any, value: p.value })))
  const ob = prefs.rsiSurachat ?? 70
  const os = prefs.rsiSurvente ?? 30
  serie.createPriceLine({ price: ob, color: 'rgba(239,68,68,0.7)',  lineWidth: 1, lineStyle: 0, axisLabelVisible: true, title: `OB ${ob}` })
  serie.createPriceLine({ price: os, color: 'rgba(16,185,129,0.7)', lineWidth: 1, lineStyle: 0, axisLabelVisible: true, title: `OS ${os}` })

  chart.timeScale().subscribeVisibleTimeRangeChange((range) => {
    if (ctx.syncing || !ctx.initialized || range === null) return
    ctx.syncing = true
    mainChart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  })
  const syncFromMain = (range: any) => {
    if (ctx.syncing || range === null) return
    ctx.syncing = true
    chart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  }
  return { chart, syncFromMain }
}

// ─── MACD ─────────────────────────────────────────────────────────────────────

interface MacdData {
  histogramme: Point[]
  macd: Point[]
  signal: Point[]
}

export function creerSousGraphiqueMacd(
  container: HTMLElement,
  macdData: MacdData,
  mainChart: IChartApi,
  ctx: SyncCtx,
): { chart: IChartApi; syncFromMain: (range: any) => void } {
  const chart = createChart(container, creerOptionsSousGraphique(container))

  const histoSerie = chart.addHistogramSeries({
    color: 'rgba(100,116,139,0.6)',
    priceLineVisible: false,
    lastValueVisible: false,
  } as any)
  histoSerie.setData(macdData.histogramme.map((p) => ({
    time: p.time as any,
    value: p.value,
    color: p.value >= 0 ? 'rgba(16,185,129,0.55)' : 'rgba(239,68,68,0.55)',
  })))

  const macdLigne = chart.addLineSeries({
    color: '#3b82f6',
    lineWidth: 1 as 1,
    crosshairMarkerVisible: false,
    lastValueVisible: true,
    priceLineVisible: false,
  } as Partial<LineSeriesOptions>)
  macdLigne.setData(macdData.macd.map((p) => ({ time: p.time as any, value: p.value })))

  const signalLigne = chart.addLineSeries({
    color: '#f59e0b',
    lineWidth: 1 as 1,
    crosshairMarkerVisible: false,
    lastValueVisible: true,
    priceLineVisible: false,
  } as Partial<LineSeriesOptions>)
  signalLigne.setData(macdData.signal.map((p) => ({ time: p.time as any, value: p.value })))
  macdLigne.createPriceLine({ price: 0, color: 'rgba(148,163,184,0.35)', lineWidth: 1, lineStyle: 0, axisLabelVisible: false, title: '0' })

  chart.timeScale().subscribeVisibleTimeRangeChange((range) => {
    if (ctx.syncing || !ctx.initialized || range === null) return
    ctx.syncing = true
    mainChart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  })
  const syncFromMain = (range: any) => {
    if (ctx.syncing || range === null) return
    ctx.syncing = true
    chart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  }
  return { chart, syncFromMain }
}

// ─── ATR ──────────────────────────────────────────────────────────────────────

export function creerSousGraphiqueAtr(
  container: HTMLElement,
  atrData: Point[],
  prefs: PrefsIndicateurs,
  mainChart: IChartApi,
  ctx: SyncCtx,
): { chart: IChartApi; syncFromMain: (range: any) => void } {
  const chart = createChart(container, creerOptionsSousGraphique(container))
  const serie = chart.addLineSeries({
    color: prefs.atrCouleur || '#f43f5e',
    lineWidth: 1 as 1,
    crosshairMarkerVisible: false,
    lastValueVisible: true,
    priceLineVisible: false,
  } as Partial<LineSeriesOptions>)
  serie.setData(atrData.map((p) => ({ time: p.time as any, value: p.value })))
  serie.createPriceLine({ price: 0, color: 'rgba(148,163,184,0.3)', lineWidth: 1, lineStyle: 0, axisLabelVisible: false, title: '' })

  chart.timeScale().subscribeVisibleTimeRangeChange((range) => {
    if (ctx.syncing || !ctx.initialized || range === null) return
    ctx.syncing = true
    mainChart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  })
  const syncFromMain = (range: any) => {
    if (ctx.syncing || range === null) return
    ctx.syncing = true
    chart.timeScale().setVisibleRange(range)
    ctx.syncing = false
  }
  return { chart, syncFromMain }
}
