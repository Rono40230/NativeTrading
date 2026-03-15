import { ref } from 'vue'
import type {
  IChartApi,
  ISeriesApi,
  SeriesType,
  LineSeriesOptions,
} from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { ReponseIndicators } from '@/services/api.service'

// Couleurs des overlays
const COULEURS = {
  ema: '#f59e0b',
  bollingerHaute: '#6366f1',
  bollingerMilieu: '#818cf8',
  bollingerBasse: '#6366f1',
  ob_long: 'rgba(16, 185, 129, 0.25)',
  ob_short: 'rgba(239, 68, 68, 0.25)',
  fvg_long: 'rgba(59, 130, 246, 0.20)',
  fvg_short: 'rgba(245, 158, 11, 0.20)',
  ifvg_long: 'rgba(99, 102, 241, 0.25)',
  ifvg_short: 'rgba(236, 72, 153, 0.25)',
  fib: 'rgba(148, 163, 184, 0.6)',
  bsl: 'rgba(16, 185, 129, 0.8)',
  ssl: 'rgba(239, 68, 68, 0.8)',
}

export function useChartIndicators() {
  const enChargement = ref(false)
  const erreur = ref<string | null>(null)

  // Tableau plain non-réactif — évite tout effet de bord Vue sur la gestion des séries
  let seriesActives: ISeriesApi<SeriesType>[] = []
  // Compteur d'annulation : si un nouvel appel démarre, le précédent est ignoré à sa résolution
  let appelEnCours = 0

  function supprimerOverlays(chart: IChartApi) {
    for (const s of seriesActives) {
      try { chart.removeSeries(s) } catch { /* série appartenant à un ancien chart */ }
    }
    seriesActives = []
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
  ) {
    // Incrémenter AVANT le suppress — tout appel concurrent sera marqué obsolète
    const idAppel = ++appelEnCours
    supprimerOverlays(chart)
    enChargement.value = true
    erreur.value = null

    try {
      const data: ReponseIndicators = await apiService.getIndicators({
        asset, tf,
        ema: prefs.ema, ema_periode: prefs.emaPeriode,
        rsi: prefs.rsi, rsi_periode: prefs.rsiPeriode,
        macd: prefs.macd,
        bollinger: prefs.bollinger,
        atr: prefs.atr,
        smc_ob: prefs.smcOb,
        smc_fvg: prefs.smcFvg,
        smc_ifvg: prefs.smcIfvg,
        smc_fib: prefs.smcFib,
        smc_tendance: prefs.smcTendance,
        smc_liquidites: prefs.smcLiquidites,
        limit: 200,
      })

      // Réponse obsolète (un appel plus récent a déjà pris la main)
      if (idAppel !== appelEnCours) return

      // ── EMA ────────────────────────────────────────────────────────────────
      if (data.ema?.length) {
        ajouterLigne(chart, data.ema, COULEURS.ema, 2)
      }

      // ── Bollinger ──────────────────────────────────────────────────────────
      if (data.bollinger) {
        ajouterLigne(chart, data.bollinger.haute, COULEURS.bollingerHaute)
        ajouterLigne(chart, data.bollinger.milieu, COULEURS.bollingerMilieu, 1)
        ajouterLigne(chart, data.bollinger.basse, COULEURS.bollingerBasse)
      }

      // ── Order Blocks ───────────────────────────────────────────────────────
      if (data.order_blocks?.length) {
        for (const ob of data.order_blocks) {
          const couleur = ob.direction === 'Long' ? COULEURS.ob_long : COULEURS.ob_short
          const f = ajouterFantome(chart)
          f.createPriceLine({ price: ob.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: `OB ${ob.direction}` })
          f.createPriceLine({ price: ob.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
        }
      }

      // ── FVG / Imbalances ───────────────────────────────────────────────────
      if (data.imbalances?.length) {
        for (const fvg of data.imbalances) {
          if (fvg.comble) continue
          const couleur = fvg.direction === 'Long' ? COULEURS.fvg_long : COULEURS.fvg_short
          const f = ajouterFantome(chart)
          f.createPriceLine({ price: fvg.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: 'FVG' })
          f.createPriceLine({ price: fvg.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
        }
      }

      // ── IFVG ───────────────────────────────────────────────────────────────
      if (data.ifvg?.length) {
        for (const i of data.ifvg) {
          const couleur = i.direction === 'Long' ? COULEURS.ifvg_long : COULEURS.ifvg_short
          const f = ajouterFantome(chart)
          f.createPriceLine({ price: i.prix_haut, color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: 'IFVG' })
          f.createPriceLine({ price: i.prix_bas,  color: couleur, lineWidth: 1, lineStyle: 2, axisLabelVisible: false, title: '' })
        }
      }

      // ── Fibonacci ──────────────────────────────────────────────────────────
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

      // ── Niveaux de liquidité BSL/SSL ───────────────────────────────────────
      if (data.liquidites?.length) {
        for (const liq of data.liquidites) {
          if (liq['sweepé']) continue
          const couleur = liq.cote === 'BSL' ? COULEURS.bsl : COULEURS.ssl
          const f = ajouterFantome(chart)
          f.createPriceLine({ price: liq.prix, color: couleur, lineWidth: 2, lineStyle: 0, axisLabelVisible: true, title: `${liq.cote}${liq.equal ? ' (EQ)' : ''}` })
        }
      }

    } catch (err_: any) {
      if (idAppel === appelEnCours) erreur.value = err_?.message ?? 'Erreur chargement indicateurs'
    } finally {
      if (idAppel === appelEnCours) enChargement.value = false
    }
  }

  // Réinitialiser le tableau sans tenter de supprimer des séries (utile après destroy chart)
  function reinitialiser() {
    seriesActives = []
    appelEnCours++
  }

  return { enChargement, erreur, chargerEtAppliquer, supprimerOverlays, reinitialiser }
}

