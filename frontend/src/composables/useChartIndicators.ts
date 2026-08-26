import { ref } from 'vue'
import type { IChartApi, ISeriesApi, SeriesType, LineSeriesOptions } from 'lightweight-charts'
import { LineStyle } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { ReponseIndicators } from '@/services/api.service'
import { COULEURS, buildIndicatorsParams } from './chartIndicatorsConfig'
import { creerSousGraphiqueRsi, creerSousGraphiqueMacd, creerSousGraphiqueAtr, type SyncCtx } from './chartSubgraphs'
import { appliquerBollinger } from './chartMainOverlays'

export function useChartIndicators() {
  const enChargement = ref(false)
  const erreur = ref<string | null>(null)

  // Tableau plain non-reactif
  let seriesActives: ISeriesApi<SeriesType>[] = []
  let candleSerieSmcRef: ISeriesApi<'Candlestick'> | null = null
  let rsiChart: IChartApi | null = null
  let syncMainToRsi: ((range: any) => void) | null = null
  let macdChart: IChartApi | null = null
  let syncMainToMacd: ((range: any) => void) | null = null
  let atrChart: IChartApi | null = null
  let syncMainToAtr: ((range: any) => void) | null = null
  // Compteur d'annulation : si un nouvel appel demarre, le precedent est ignore
  let appelEnCours = 0

  function supprimerOverlays(
    chart: IChartApi,
    rsiCont: HTMLElement | null = null,
    macdCont: HTMLElement | null = null,
    atrCont: HTMLElement | null = null,
  ) {
    candleSerieSmcRef = null
    // Désabonner et détruire uniquement les sous-graphiques qu'on va recréer
    // (container fourni = recréation prévue). Si container=null, le sous-graphique
    // est préservé pour éviter le flash/disparition lors du rafraîchissement SMC.
    if (rsiCont !== null && syncMainToRsi) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToRsi)
      syncMainToRsi = null
    }
    if (macdCont !== null && syncMainToMacd) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToMacd)
      syncMainToMacd = null
    }
    if (atrCont !== null && syncMainToAtr) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToAtr)
      syncMainToAtr = null
    }
    for (const s of seriesActives) {
      try { chart.removeSeries(s) } catch { /* serie appartenant a un ancien chart */ }
    }
    seriesActives = []
    if (rsiCont !== null && rsiChart) {
      try { rsiChart.remove() } catch { }
      rsiChart = null
    }
    if (macdCont !== null && macdChart) {
      try { macdChart.remove() } catch { }
      macdChart = null
    }
    if (atrCont !== null && atrChart) {
      try { atrChart.remove() } catch { }
      atrChart = null
    }
  }

  function ajouterLigne(
    chart: IChartApi,
    data: { time: number; value: number }[],
    couleur: string,
    largeur = 1,
    style: LineStyle = LineStyle.Solid,
    titre = '',
  ): ISeriesApi<'Line'> {
    const s = chart.addLineSeries({
      color: couleur,
      lineWidth: largeur as 1 | 2 | 3 | 4,
      lineStyle: style,
      title: titre,
      crosshairMarkerVisible: false,
      lastValueVisible: false,
      priceLineVisible: false,
    } as Partial<LineSeriesOptions>)
    s.setData(data.map((p) => ({ time: p.time as any, value: p.value })))
    seriesActives.push(s)
    return s
  }

  async function chargerEtAppliquer(
    chart: IChartApi,
    asset: string,
    tf: string,
    prefs: PrefsIndicateurs,
    rsiContainer: HTMLElement | null = null,
    macdContainer: HTMLElement | null = null,
    atrContainer: HTMLElement | null = null,
    candleSerie: ISeriesApi<'Candlestick'> | null = null,
    onDonnees?: (data: ReponseIndicators) => void,
  ) {
    const idAppel = ++appelEnCours
    supprimerOverlays(chart, rsiContainer, macdContainer, atrContainer)
    enChargement.value = true
    erreur.value = null

    try {
      const data: ReponseIndicators = await apiService.getIndicators(buildIndicatorsParams(asset, tf, prefs))

      if (idAppel !== appelEnCours) return

      // Guard anti-rebond : synchronisation par timestamps absolus (pas d'indices logiques)
      const ctx: SyncCtx = { syncing: false, initialized: false }

      // EMA
      if (data.ema?.length) {
        ajouterLigne(chart, data.ema, prefs.emaCouleur || COULEURS.ema, 2)
      }

      // RSI
      if (data.rsi?.length && rsiContainer) {
        const r = creerSousGraphiqueRsi(rsiContainer, data.rsi, prefs, chart, ctx)
        rsiChart = r.chart
        syncMainToRsi = r.syncFromMain
        chart.timeScale().subscribeVisibleTimeRangeChange(syncMainToRsi)
      }

      // MACD
      if (data.macd && macdContainer) {
        const r = creerSousGraphiqueMacd(macdContainer, data.macd, chart, ctx)
        macdChart = r.chart
        syncMainToMacd = r.syncFromMain
        chart.timeScale().subscribeVisibleTimeRangeChange(syncMainToMacd)
      }

      // ATR
      if (data.atr?.length && atrContainer) {
        const r = creerSousGraphiqueAtr(atrContainer, data.atr, prefs, chart, ctx)
        atrChart = r.chart
        syncMainToAtr = r.syncFromMain
        chart.timeScale().subscribeVisibleTimeRangeChange(syncMainToAtr)
      }

      // Sync final : rAF garantit la fin des fitContent LW-charts,
      // setTimeout(0) garantit la fin de tous les repaints avant d'activer initialized.
      requestAnimationFrame(() => {
        setTimeout(() => {
          if (idAppel !== appelEnCours) return
          const timeRange = chart.timeScale().getVisibleRange()
          if (!timeRange) return
          ctx.syncing = true
          if (rsiChart)  rsiChart.timeScale().setVisibleRange(timeRange)
          if (macdChart) macdChart.timeScale().setVisibleRange(timeRange)
          if (atrChart)  atrChart.timeScale().setVisibleRange(timeRange)
          ctx.syncing = false
          ctx.initialized = true
        }, 0)
      })

      // Bollinger Bands (overlay principal)
      if (data.bollinger) {
        appliquerBollinger(chart, data.bollinger, prefs, ajouterLigne, (s) => seriesActives.push(s))
      }

      // Fibonacci — lignes SÉRIES ancrées à la bougie d'origine du swing
      // (comme les autres indicateurs), étendues jusqu'à maintenant. Pas
      // d'étiquette de prix : le libellé « Fib x » n'apparaît qu'au survol
      // (crosshair). Retracement auto : 100 % = swing haut, 0 % = swing bas.
      if (prefs.fibonacci && data.fibonacci) {
        const f = data.fibonacci
        const couleur = prefs.fibCouleur || '#94a3b8'
        const depart = Math.min(f.timestamp_haut, f.timestamp_bas)
        const fin = Math.floor(Date.now() / 1000)
        const ajouterFib = (prix: number, titre: string, style: LineStyle) => {
          ajouterLigne(chart, [{ time: depart, value: prix }, { time: fin, value: prix }], couleur, 1, style, titre)
        }
        if (prefs.fibSwings) {
          ajouterFib(f.swing_haut, 'Fib 100 %', LineStyle.Solid)
          ajouterFib(f.swing_bas, 'Fib 0 %', LineStyle.Solid)
        }
        if (prefs.fibNiveau500) ajouterFib(f.niveau_500, 'Fib 0.5', LineStyle.Dashed)
        if (prefs.fibNiveau618) ajouterFib(f.niveau_618, 'Fib 0.618', LineStyle.Dashed)
        if (prefs.fibNiveau786) ajouterFib(f.niveau_786, 'Fib 0.786', LineStyle.Dashed)
      }

      // Overlays SMC v12 — dessinés par canvas dédié (useSmcV12Overlay)
      if (candleSerie) {
        candleSerieSmcRef = candleSerie
      }

      // Callback data SMC (pour canvas overlay)
      onDonnees?.(data)

    } catch (err_: any) {
      if (idAppel === appelEnCours) erreur.value = err_?.message ?? 'Erreur chargement indicateurs'
    } finally {
      if (idAppel === appelEnCours) enChargement.value = false
    }
  }

  // Reinitialiser sans tenter de supprimer des series (utile apres destroy chart)
  function reinitialiser() {
    seriesActives = []
    appelEnCours++
    candleSerieSmcRef = null
    syncMainToRsi = null
    syncMainToMacd = null
    syncMainToAtr = null
    if (rsiChart) { try { rsiChart.remove() } catch { } rsiChart = null }
    if (macdChart) { try { macdChart.remove() } catch { } macdChart = null }
    if (atrChart) { try { atrChart.remove() } catch { } atrChart = null }
  }


  return { enChargement, erreur, chargerEtAppliquer, supprimerOverlays, reinitialiser }
}
