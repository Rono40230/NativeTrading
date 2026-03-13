import { type Ref, type ComputedRef } from 'vue'
import {
  createChart,
  type IChartApi,
  type ISeriesApi,
  type CandlestickSeriesOptions,
  type Time,
} from 'lightweight-charts'
import type { Candle } from '@/services/api.service'

/**
 * Gestion du cycle de vie du graphique TradingView Lightweight Charts.
 * Extrait de ChartsView pour respecter la limite de 300 lignes.
 */
export function useChartTradingView(
  chartContainer: Ref<HTMLElement | null>,
  bougies: ComputedRef<Candle[]>,
) {
  let chart: IChartApi | null = null
  let candleSeries: ISeriesApi<'Candlestick'> | null = null
  let resizeObserver: ResizeObserver | null = null

  function initChart() {
    if (!chartContainer.value) return
    chart = createChart(chartContainer.value, {
      layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      crosshair: { mode: 1 },
      rightPriceScale: { borderColor: 'rgba(255,255,255,0.1)' },
      timeScale: {
        borderColor: 'rgba(255,255,255,0.1)',
        timeVisible: true,
        secondsVisible: false,
      },
      width: chartContainer.value.clientWidth,
      height: chartContainer.value.clientHeight,
    })

    const opts: Partial<CandlestickSeriesOptions> = {
      upColor: '#10b981',
      downColor: '#ef4444',
      borderUpColor: '#10b981',
      borderDownColor: '#ef4444',
      wickUpColor: '#10b981',
      wickDownColor: '#ef4444',
    }
    candleSeries = chart.addCandlestickSeries(opts)
    mettreAJourSerie(true)
  }

  function mettreAJourSerie(scrollToEnd = false) {
    if (!candleSeries) return
    const data = bougies.value.map((b) => ({
      time: (new Date(b.timestamp).getTime() / 1000) as unknown as Time,
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
    }))
    if (data.length > 0) {
      candleSeries.setData(data)
      if (scrollToEnd) chart?.timeScale().scrollToRealTime()
    }
  }

  function mettreAJourEnDirect(bougie: Candle) {
    if (!candleSeries) return
    candleSeries.update({
      time: (new Date(bougie.timestamp).getTime() / 1000) as unknown as Time,
      open: bougie.open,
      high: bougie.high,
      low: bougie.low,
      close: bougie.close,
    })
  }

  function detruireChart() {
    chart?.remove()
    chart = null
    candleSeries = null
  }

  function configurerRedimensionnement() {
    if (!chartContainer.value) return
    resizeObserver = new ResizeObserver(() => {
      if (chart && chartContainer.value) {
        chart.applyOptions({
          width: chartContainer.value.clientWidth,
          height: chartContainer.value.clientHeight,
        })
      }
    })
    resizeObserver.observe(chartContainer.value)
  }

  function arreterRedimensionnement() {
    resizeObserver?.disconnect()
  }

  function getChart() {
    return chart
  }

  return {
    initChart,
    mettreAJourSerie,
    mettreAJourEnDirect,
    detruireChart,
    configurerRedimensionnement,
    arreterRedimensionnement,
    getChart,
  }
}
