/**
 * useSmcV12Overlay — overlay canvas dédié aux indicateurs SMC v12.
 *
 * Affiche, par-dessus le chart lightweight-charts :
 *   - bgcolor de tendance (vert/rouge) sur toute la zone visible ;
 *   - labels de structure HH/HL/LH/LL positionnés sur les pivots réels ;
 *   - lignes BOS (continues), MSS (dashed) et CHOCH (solid épaisse) ;
 *   - labels Sweeps sur la bougie concernée ;
 *   - boxes OB (rectangle top→bot étendu vers la droite) + label "OB x/10" ;
 *   - boxes FVG ;
 *   - boxes trade (SL/TP) + label BUY/SELL pour les signaux.
 *
 * + 13 indicateurs étendus (rendu délégué à `smcV12OverlayExtra.ts`) :
 *   bgcolor sessions (Asie/Londres/NY), volume fort, impulsions,
 *   premium/discount + equilibrium, Asian HL, liquidités PDH/PDL/PWH/PWL,
 *   EQH/EQL, boxes NDOG/NWOG, MTF OB (H1/H4/W1/MN), zone-cœur, breaker,
 *   imbalance, OTE.
 *
 * Chaque indicateur est filtré par un flag `settingsStore.indicateurs.v12Xxx`.
 * Si OFF → l'indicateur n'est pas dessiné (comme TradingView). Les indicateurs
 * étendus sont optionnels côté API (champs absents si backend non mis à jour).
 *
 * Le rendu canvas des 9 indicateurs de base vit dans `smcV12OverlayDrawBase.ts`
 * (split pour la règle vibe < 600 lignes/fichier).
 *
 * z-index 4 — au-dessus des canvas SMC (2) / liquidités (3).
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { SmcV12Analyse } from '@/services/api.smc'
import { useSettingsStore } from '@/stores/settings.store'
import {
  dessinerTendance,
  dessinerObsEtFvgs,
  dessinerLignes,
  dessinerSweeps,
  dessinerPivots,
} from './smcV12OverlayDrawBase'
import { dessinerSignaux, dessinerTradesExternes } from './smcV12OverlayDrawTrades'
import type { SignalDessin } from './smcV12OverlayDrawTrades'
import type {
  ObDessin,
  LigneDessin,
  PivotDessin,
  FvgDessin,
  SweepDessin,
  FlagsV12,
} from './smcV12OverlayDrawBase'
import {
  dessinerFonds as dessinerFondsExt,
  dessinerBoxes as dessinerBoxesExt,
  donneesV12EtenduesVides,
} from './smcV12OverlayExtra'
import { dessinerLignes as dessinerLignesExt } from './smcV12OverlayExtraLignes'
import type { DonneesV12Etendues, FlagsV12Etendus } from './smcV12OverlayExtra'

export function useSmcV12Overlay() {
  const settingsStore = useSettingsStore()

  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null
  let dernierTsRef: number | null = null

  let pivots: PivotDessin[] = []
  let bos: LigneDessin[] = []
  let mss: LigneDessin[] = []
  let chochs: LigneDessin[] = []
  let sweeps: SweepDessin[] = []
  let obs: ObDessin[] = []
  let fvgs: FvgDessin[] = []
  let signals: SignalDessin[] = []
  // Trades ouverts des AUTRES timeframes du même actif (multi-TF — dessin
  // atténué avec badge du TF d'origine, dédupliqués avec le TF affiché).
  let tradesExternes: (SignalDessin & { tfOrigine: string; enAttente?: boolean })[] = []
  let tendance: 'haussiere' | 'baissiere' | 'neutre' = 'neutre'
  // Indicateurs v12 étendus (13 types supplémentaires).
  let donneesExt: DonneesV12Etendues = donneesV12EtenduesVides()

  // ── Canvas lifecycle ────────────────────────────────────────────────────────
  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top = '0'
    c.style.left = '0'
    c.style.width = '100%'
    c.style.height = '100%'
    c.style.pointerEvents = 'none'
    c.style.zIndex = '4'
    container.appendChild(c)
    return c
  }

  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  function lireFlags(): FlagsV12 {
    const p = settingsStore.indicateurs
    return {
      tendance: p.v12Tendance,
      structure: p.v12Structure,
      bos: p.v12Bos,
      mss: p.v12Mss,
      choch: p.v12Choch,
      sweeps: p.v12Sweeps,
      ob: p.v12Ob,
      fvg: p.v12Fvg,
      signals: p.v12Signals,
    }
  }

  /** Flags de visibilité des 13 indicateurs étendus. */
  function lireFlagsExt(): FlagsV12Etendus {
    const p = settingsStore.indicateurs
    return {
      sessionAsie: p.v12SessionAsie,
      sessionLondres: p.v12SessionLondres,
      sessionNy: p.v12SessionNy,
      eqhEql: p.v12EqhEql,
      asianHl: p.v12AsianHl,
      niveauxCles: p.v12NiveauxCles,
      ndog: p.v12Ndog,
      nwog: p.v12Nwog,
      breaker: p.v12Breaker,
      propulsion: p.v12Propulsion,
      imbalance: p.v12Imbalance,
      bpr: p.v12Bpr,
      ote: p.v12Ote,
      premium: p.v12Premium,
      equilibrium: p.v12Equilibrium,
      obH1: p.v12ObH1,
      obH4: p.v12ObH4,
      obW1: p.v12ObW1,
      obMn: p.v12ObMn,
      zoneCoeur: p.v12ZoneCoeur,
      volume: p.v12Volume,
      impulsion: p.v12Impulsion,
    }
  }

  // ── Rendu ───────────────────────────────────────────────────────────────────
  function redessiner() {
    if (!canvas || !chartRef || !serieRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)
    const ts = chartRef.timeScale()
    const flags = lireFlags()
    const flagsExt = lireFlagsExt()

    // Phase 0 — bgcolor tendance (z le plus bas).
    dessinerTendance(ctx, ts, W, H, tendance, flags, dernierTsRef, donneesExt.trend_ranges ?? [])
    // Phase 1 — fonds étendus (sessions, volume fort, impulsions, premium/discount).
    dessinerFondsExt(ctx, ts, W, H, serieRef, donneesExt, flagsExt, dernierTsRef)
    // Phase 2 — boxes étendues (NDOG/NWOG, MTF OB, zone-cœur, breaker, imbalance, OTE).
    dessinerBoxesExt(ctx, serieRef, ts, W, donneesExt, flagsExt, dernierTsRef)
    // Phase 2b — boxes OB / FVG (par-dessus les boxes étendues).
    if (flags.ob || flags.fvg) dessinerObsEtFvgs(ctx, serieRef, ts, obs, fvgs, W, dernierTsRef, flags)
    // Phase 3 — lignes horizontales étendues (Asian HL, liquidités, EQH/EQL, equilibrium).
    dessinerLignesExt(ctx, serieRef, ts, W, donneesExt, flagsExt, dernierTsRef)
    if (flags.bos) dessinerLignes(ctx, serieRef, ts, bos, W, dernierTsRef, 'bos')
    if (flags.mss) dessinerLignes(ctx, serieRef, ts, mss, W, dernierTsRef, 'mss')
    if (flags.choch) dessinerLignes(ctx, serieRef, ts, chochs, W, dernierTsRef, 'choch')
    if (flags.sweeps) dessinerSweeps(ctx, serieRef, ts, sweeps, W)
    if (flags.signals) dessinerSignaux(ctx, serieRef, ts, signals, W, dernierTsRef)
    if (flags.signals) dessinerTradesExternes(ctx, serieRef, ts, tradesExternes, W, dernierTsRef)
    if (flags.structure) dessinerPivots(ctx, serieRef, ts, pivots, W)
  }

  function planifierRedessiner() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => {
      animFrame = null
      redessiner()
    })
  }

  function initialiser(chart: IChartApi, serie: ISeriesApi<'Candlestick'>, container: HTMLElement) {
    detruire()
    chartRef = chart
    serieRef = serie
    canvas = monterCanvas(container)
    redimensionner()

    const handler = () => planifierRedessiner()
    chart.timeScale().subscribeVisibleTimeRangeChange(handler)
    chart.timeScale().subscribeVisibleLogicalRangeChange(handler)
    unsubscribe = () => {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(handler)
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(handler)
    }

    const ro = new ResizeObserver(() => {
      redimensionner()
      planifierRedessiner()
    })
    ro.observe(container)
    ;(canvas as any).__ro = ro

    planifierRedessiner()
  }

  /** Charge les données v12 depuis l'API et déclenche le redessin. */
  /// Définit les trades ouverts des autres TF (appelé par la vue à chaque
  /// rafraîchissement — source : table signaux, actif courant, Actifs).
  function definirTradesExternes(trades: (SignalDessin & { tfOrigine: string; enAttente?: boolean })[]) {
    tradesExternes = trades
    planifierRedessiner()
  }

  async function charger(asset: string, timeframe: string, limit = 500, dernierTimestamp?: number) {
    dernierTsRef = dernierTimestamp ?? null
    let data: SmcV12Analyse | null = null
    try {
      data = await apiService.getSmcV12Analyse(asset, timeframe, limit)
    } catch {
      data = null
    }
    if (!data) {
      effacer()
      return
    }
    pivots = data.pivots.map((p) => ({ ts: p.ts, price: p.price, type: p.type }))
    bos = data.bos.map((b) => ({
      ts: b.ts, pivot_ts: b.pivot_ts, level: b.level, dir: b.dir,
      label: b.dir === 'bull' ? 'BOS ↑' : 'BOS ↓',
    }))
    mss = data.mss.map((m) => ({
      ts: m.ts, pivot_ts: m.pivot_ts, level: m.level, dir: m.dir,
      label: m.dir === 'bull' ? 'MSS ↑' : 'MSS ↓',
    }))
    chochs = data.chochs.map((c) => ({
      ts: c.ts, pivot_ts: c.pivot_ts, level: c.level, dir: c.dir,
      label: c.dir === 'bull' ? 'CHOCH ↑' : 'CHOCH ↓',
    }))
    sweeps = data.sweeps.slice(-6).map((s) => ({ ts: s.ts, level: s.level, dir: s.dir, candleHigh: s.candle_high, candleLow: s.candle_low }))
    obs = data.obs.map((o) => ({
      ts: o.ts, top: o.top, bot: o.bot, force: o.force, dir: o.dir, state: o.state,
    }))
    fvgs = data.fvgs.map((f) => ({
      ts: f.ts, top: f.top, bot: f.bot, dir: f.dir, state: f.state,
    }))
    // Largeur finie des boxes trade (Pine i_tpWidth : 40 barres, H1=30,
    // H4=20 — jamais jusqu'au bord droit) + visibilité au fill (Pine :
    // ordre en attente INVISIBLE, objets rendus visibles au fill).
    const largeurTp: Record<string, number> = { H1: 30, H4: 20 }
    const dureeBarre: Record<string, number> = {
      M1: 60, M5: 300, M15: 900, M30: 1800,
      H1: 3600, H4: 14400, D1: 86400, W1: 604800,
    }
    const nbBarres = largeurTp[timeframe] ?? 40
    const duree = dureeBarre[timeframe] ?? 900
    // Fidélité Pine : invisible avant fill (bgcolor=na à la création) et
    // SUPPRIMÉ à la clôture (f_delBullSignal) — seuls les trades remplis
    // vivants se dessinent, ancrés sur leur barre de fill.
    signals = data.signals
      .filter((s) => s.filled !== false && s.ferme !== true)
      .map((s) => {
        const ancrage = s.fill_ts ?? s.ts
        return {
          ts: ancrage, entry: s.entry, sl: s.sl,
          tp1: s.tp1, tp2: s.tp2, tp3: s.tp3,
          dir: s.dir, force: s.force,
          be: s.be === true, label: (s.label ?? []) as string[],
          tsFin: ancrage + nbBarres * duree,
        }
      })
    tendance = data.tendance
    // 13 indicateurs étendus (optionnels : absents si backend non mis à jour).
    donneesExt = {
      sessions: data.sessions ?? [],
      trend_ranges: data.trend_ranges ?? [],
      prem_ranges: data.prem_ranges ?? [],
      session_boxes: data.session_boxes ?? [],
      vol_fort: data.vol_fort ?? [],
      impulsions: data.impulsions ?? [],
      premium_discount: data.premium_discount ?? null,
      asian_hl: data.asian_hl ?? null,
      liquidites: data.liquidites ?? [],
      eqs: data.eqs ?? [],
      gaps: data.gaps ?? [],
      breakers: data.breakers ?? [],
      propulsions: data.propulsions ?? [],
      imbalances: data.imbalances ?? [],
      bprs: data.bprs ?? [],
      otes: data.otes ?? [],
      mtf_obs: data.mtf_obs ?? [],
      zone_coeur: data.zone_coeur ?? [],
    }
    planifierRedessiner()
  }

  function effacer() {
    pivots = []
    bos = []
    mss = []
    chochs = []
    sweeps = []
    obs = []
    fvgs = []
    signals = []
    tradesExternes = []
    tendance = 'neutre'
    donneesExt = donneesV12EtenduesVides()
    if (canvas) {
      const ctx = canvas.getContext('2d')
      if (ctx) ctx.clearRect(0, 0, canvas.offsetWidth, canvas.offsetHeight)
    }
  }

  function detruire() {
    if (animFrame !== null) {
      cancelAnimationFrame(animFrame)
      animFrame = null
    }
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
    effacer()
  }

  /** Met à jour uniquement le bord droit (dernière bougie) sans refetch. */
  function setDernierTs(ts?: number) {
    dernierTsRef = ts ?? null
    planifierRedessiner()
  }

  return { initialiser, charger, definirTradesExternes, effacer, detruire, setDernierTs, redessiner: planifierRedessiner }
}
