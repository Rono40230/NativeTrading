import { ref } from 'vue'
import type { IChartApi, ISeriesApi, SeriesType, LineSeriesOptions } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { ReponseIndicators } from '@/services/api.service'
import { COULEURS } from './chartIndicatorsConfig'
import { creerSousGraphiqueRsi, creerSousGraphiqueMacd, creerSousGraphiqueAtr, type SyncCtx } from './chartSubgraphs'
import { appliquerBollinger, appliquerSmcOverlays } from './chartMainOverlays'
import { rendreSurSerie, effacerMarqueurs } from './chartSignauxRendu'
import { filtreDefaut, type FiltreSignaux, type SignalIndicateur } from './chartSignauxTypes'
import { calculerSlTp, afficherSlTp, effacerSlTp, type LignesSlTp } from './chartAtrSlTp'

export function useChartIndicators() {
  const enChargement = ref(false)
  const erreur = ref<string | null>(null)
  const signauxActifs = ref<SignalIndicateur[]>([])

  // Tableau plain non-reactif
  let seriesActives: ISeriesApi<SeriesType>[] = []
  let rsiChart: IChartApi | null = null
  let syncMainToRsi: ((range: any) => void) | null = null
  let macdChart: IChartApi | null = null
  let syncMainToMacd: ((range: any) => void) | null = null
  let atrChart: IChartApi | null = null
  let syncMainToAtr: ((range: any) => void) | null = null
  // SL/TP : valeurs ATR indexées par timestamp + lignes de prix actives
  let atrValeurs = new Map<number, number>()
  let lignesSlTp: LignesSlTp = { sl: null, tp1: null, tp2: null }
  let candleSerieSlTp: ISeriesApi<'Candlestick'> | null = null
  // Compteur d'annulation : si un nouvel appel demarre, le precedent est ignore
  let appelEnCours = 0

  function supprimerOverlays(chart: IChartApi) {
    if (syncMainToRsi) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToRsi)
      syncMainToRsi = null
    }
    if (syncMainToMacd) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToMacd)
      syncMainToMacd = null
    }
    if (syncMainToAtr) {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(syncMainToAtr)
      syncMainToAtr = null
    }
    for (const s of seriesActives) {
      try { chart.removeSeries(s) } catch { /* serie appartenant a un ancien chart */ }
    }
    seriesActives = []
    if (rsiChart) {
      try { rsiChart.remove() } catch { }
      rsiChart = null
    }
    if (macdChart) {
      try { macdChart.remove() } catch { }
      macdChart = null
    }
    if (atrChart) {
      try { atrChart.remove() } catch { }
      atrChart = null
    }
  }

  function ajouterLigne(
    chart: IChartApi,
    data: { time: number; value: number }[],
    couleur: string,
    largeur = 1,
  ): ISeriesApi<'Line'> {
    const s = chart.addLineSeries({
      color: couleur,
      lineWidth: largeur as 1 | 2 | 3 | 4,
      crosshairMarkerVisible: false,
      lastValueVisible: false,
      priceLineVisible: false,
    } as Partial<LineSeriesOptions>)
    s.setData(data.map((p) => ({ time: p.time as any, value: p.value })))
    seriesActives.push(s)
    return s
  }

  function ajouterFantome(chart: IChartApi): ISeriesApi<'Line'> {
    const s = chart.addLineSeries({
      color: 'transparent',
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    } as Partial<LineSeriesOptions>)
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
    filtre: FiltreSignaux = filtreDefaut(),
  ) {
    const idAppel = ++appelEnCours
    supprimerOverlays(chart)
    enChargement.value = true
    erreur.value = null

    try {
      const data: ReponseIndicators = await apiService.getIndicators({
        asset, tf,
        ema: prefs.ema, ema_periode: prefs.emaPeriode, ema_ma_type: prefs.emaMaType,
        rsi: prefs.rsi, rsi_periode: prefs.rsiPeriode,
        macd: prefs.macd, macd_rapide: prefs.macdRapide, macd_lente: prefs.macdLente, macd_signal: prefs.macdSignal,
        bollinger: prefs.bollinger,
        bollinger_periode: prefs.bollingerPeriode,
        bollinger_stddev: prefs.bollingerStdDev,
        bollinger_ma_type: prefs.bollingerMaType,
        atr: prefs.atr,
        atr_periode: prefs.atrPeriode,
        smc_ob: prefs.smcOb,
        smc_fvg: prefs.smcFvg,
        smc_ifvg: prefs.smcIfvg,
        smc_fib: prefs.smcFib,
        smc_tendance: prefs.smcTendance,
        smc_liquidites: prefs.smcLiquidites,
        signaux: true,
        limit: 500,
      })

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

      // Overlays SMC (Order Blocks, FVG, IFVG, Fibonacci, BSL/SSL)
      appliquerSmcOverlays(chart, data, ajouterFantome)

      // Signaux indicateurs — marqueurs sur la série candlestick
      if (data.signaux) {
        signauxActifs.value = data.signaux
        if (candleSerie) rendreSurSerie(candleSerie, data.signaux, filtre)
      } else {
        signauxActifs.value = []
      }

      // Valeurs ATR pour SL/TP
      if (data.atr_valeurs) {
        atrValeurs = new Map(data.atr_valeurs.map((p) => [p.time, p.value]))
      } else {
        atrValeurs = new Map()
      }
      candleSerieSlTp = candleSerie

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
    syncMainToRsi = null
    syncMainToMacd = null
    syncMainToAtr = null
    atrValeurs = new Map()
    lignesSlTp = { sl: null, tp1: null, tp2: null }
    candleSerieSlTp = null
    if (rsiChart) { try { rsiChart.remove() } catch { } rsiChart = null }
    if (macdChart) { try { macdChart.remove() } catch { } macdChart = null }
    if (atrChart) { try { atrChart.remove() } catch { } atrChart = null }
  }

  /** Re-rend les marqueurs avec un nouveau filtre (sans recharger les données) */
  function appliquerMarqueursSignaux(
    candleSerie: ISeriesApi<'Candlestick'> | null,
    filtre: FiltreSignaux,
  ) {
    if (!candleSerie) return
    if (signauxActifs.value.length === 0) {
      effacerMarqueurs(candleSerie)
      return
    }
    rendreSurSerie(candleSerie, signauxActifs.value, filtre)
  }

  /**
   * Affiche ou met à jour les lignes SL/TP pour le signal Fort le plus proche du timestamp.
   * Respecte le filtre courant (seuls les signaux des sources actives sont considérés).
   */
  function mettreAJourSlTp(
    candleSerie: ISeriesApi<'Candlestick'> | null,
    timestamp: number | null,
    slTpActif: boolean,
  ) {
    const serie = candleSerie ?? candleSerieSlTp
    effacerSlTp(serie, lignesSlTp)
    if (!serie || !timestamp || !slTpActif) return

    // SL/TP : tous les signaux Fort directionnels, indépendamment du filtre source
    const signal = signauxActifs.value
      .filter((s) => s.direction !== 'neutre' && s.force === 'fort')
      .sort((a, b) => Math.abs(a.timestamp - timestamp) - Math.abs(b.timestamp - timestamp))
      .at(0)

    if (!signal) return
    const atr = atrValeurs.get(signal.timestamp)
    if (!atr) return

    const niveau = calculerSlTp(signal, atr)
    if (niveau) lignesSlTp = afficherSlTp(serie, niveau)
  }

  /**
   * Cherche un signal par l'id de son marqueur LW-Charts (`${source}_${type_signal}_${timestamp}`)
   * et calcule les niveaux SL/TP si les valeurs ATR sont disponibles.
   */
  function obtenirSignalEtNiveaux(
    markerId: string,
  ): { signal: SignalIndicateur; niveaux: ReturnType<typeof calculerSlTp> } | null {
    const signal = signauxActifs.value.find(
      (s) => `${s.source}_${s.type_signal}_${s.timestamp}` === markerId,
    )
    if (!signal) return null
    const atr = atrValeurs.get(signal.timestamp)
    return { signal, niveaux: atr ? calculerSlTp(signal, atr) : null }
  }

  return { enChargement, erreur, signauxActifs, chargerEtAppliquer, supprimerOverlays, reinitialiser, appliquerMarqueursSignaux, mettreAJourSlTp, obtenirSignalEtNiveaux }
}
