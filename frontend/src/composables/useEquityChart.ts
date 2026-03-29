import { ref, watch, type Ref } from 'vue'
import { createChart, type IChartApi } from 'lightweight-charts'
import type { BacktestResults } from '@/services/api.service'
import { tickMarkFormatterEquity } from '@/composables/chartTimeScale'

export function useEquityChart(resultats: Ref<BacktestResults | null>) {
  const equityChart = ref<HTMLElement | null>(null)
  let chart: IChartApi | null = null
  let roEquity: ResizeObserver | null = null

  function afficherCourbe() {
    if (!equityChart.value || !resultats.value) return
    chart?.remove()
    chart = createChart(equityChart.value, {
      layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
      grid: { vertLines: { color: '#1f2937' }, horzLines: { color: '#1f2937' } },
      timeScale: { timeVisible: true, secondsVisible: false, tickMarkFormatter: tickMarkFormatterEquity },
      width: equityChart.value.clientWidth, height: 256,
    })
    const series = chart.addAreaSeries({
      lineColor: resultats.value.roi_pct >= 0 ? '#10b981' : '#ef4444',
      topColor: resultats.value.roi_pct >= 0 ? '#10b98133' : '#ef444433',
      bottomColor: 'transparent',
    })
    const capitalInitialSerie = chart.addLineSeries({
      color: '#3b82f6',
      lineWidth: 1,
      lineStyle: 2,
      lastValueVisible: false,
      priceLineVisible: false,
    })
    const pointsReels = resultats.value.equity_curve?.map((point) => ({
      time: point.timestamp as unknown as import('lightweight-charts').Time,
      value: point.capital,
    }))
    const n = Math.max(resultats.value.total_trades, 10)
    const pointsFallback = Array.from({ length: n }, (_, i) => ({
      time: (Math.floor(Date.now() / 1000) - (n - i) * 86400) as unknown as import('lightweight-charts').Time,
      value: resultats.value!.capital_initial + (resultats.value!.profit_net * i) / (n - 1),
    }))
    const pts = pointsReels && pointsReels.length >= 2 ? pointsReels : pointsFallback
    series.setData(pts)
    const debut = pts[0]?.time
    const fin = pts[pts.length - 1]?.time
    if (debut && fin) {
      capitalInitialSerie.setData([
        { time: debut, value: resultats.value.capital_initial },
        { time: fin, value: resultats.value.capital_initial },
      ])
    }
    chart.timeScale().fitContent()
  }

  watch(equityChart, (el) => {
    roEquity?.disconnect()
    if (!el) return
    if (resultats.value) afficherCourbe()
    roEquity = new ResizeObserver(() => chart?.applyOptions({ width: el.clientWidth }))
    roEquity.observe(el)
  })

  function cleanup() {
    roEquity?.disconnect()
  }

  return { equityChart, afficherCourbe, cleanup }
}
