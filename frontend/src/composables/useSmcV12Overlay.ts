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
 * Chaque indicateur est filtré par un flag `settingsStore.indicateurs.v12Xxx`.
 * Si OFF → l'indicateur n'est pas dessiné (comme TradingView).
 *
 * NOTE : seuls les indicateurs réellement retournés par /api/smc/v12/analyse
 * sont dessinables (pivots, bos, mss, chochs, sweeps, obs, fvgs, signals,
 * tendance). Les autres flags v12 (sessions, NDOG, HTF OB, OTE, premium/
 * discount, breaker, zone-cœur, imbalance, volume/impulsion bgcolor,
 * equilibrium, niveaux clés) existent dans le store mais leur donnée n'est
 * pas encore exposée par le backend → rien n'est dessiné tant que l'API
 * n'est pas étendue.
 *
 * z-index 4 — au-dessus des canvas SMC (2) / liquidités (3).
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { SmcV12Analyse } from '@/services/api.smc'
import { useSettingsStore } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'

// ── Palette (Pine smc_indicateur_v12) ─────────────────────────────────────────
const COUL_HH = '#00C853' // structure haussière forte
const COUL_HL = '#69F0AE'
const COUL_LH = '#FF5252' // structure baissière
const COUL_LL = '#D50000'

const COUL_BOS_BULL = '#2962FF'
const COUL_BOS_BEAR = '#FF6D00'

const COUL_MSS_BULL = '#00BCD4' // cyan
const COUL_MSS_BEAR = '#FF9800' // orange

const COUL_CHOCH_BULL = '#AA00FF' // violet
const COUL_CHOCH_BEAR = '#FF1744' // rouge

const COUL_SWEEP_BULL = '#00E676'
const COUL_SWEEP_BEAR = '#FF1744'

const COUL_OB_BULL = '#00C853'
const COUL_OB_BEAR = '#D50000'

const COUL_FVG_BULL = '#00C853'
const COUL_FVG_BEAR = '#D50000'

const COUL_TENDANCE_HAUSSE = '#4CAF50'
const COUL_TENDANCE_BAISSE = '#F44336'

const COUL_SL = '#ef5350'
const COUL_TP = '#26a69a'
const COUL_ENTRY = '#3b82f6'
const COUL_BUY = '#1b5e20'
const COUL_SELL = '#b71c1c'

// Alpha = (100 - transparence_pine) / 100.
const OB_ALPHA: Record<string, number> = {
  vierge: (100 - 70) / 100,
  partiel: (100 - 83) / 100,
  profond: (100 - 91) / 100,
}
const OB_BORD_ALPHA = (100 - 20) / 100 // bordure = couleur sens transp 20
const FVG_ALPHA: Record<string, number> = {
  vierge: (100 - 93) / 100,
  partiel: (100 - 96) / 100,
}
const FVG_BORD_ALPHA = (100 - 85) / 100 // bordure blanche transp 85
const TENDANCE_ALPHA = (100 - 95) / 100

interface ObDessin {
  ts: number
  top: number
  bot: number
  force: number
  dir: 'bull' | 'bear'
  state: string
}
interface LigneDessin {
  ts: number
  level: number
  dir: 'bull' | 'bear'
  label: string
}
interface SignalDessin {
  ts: number
  entry: number
  sl: number
  tp1: number
  dir: 'Long' | 'Short'
  force: number
}
interface PivotDessin {
  ts: number
  price: number
  type: 'HH' | 'HL' | 'LH' | 'LL'
}
interface FvgDessin {
  ts: number
  top: number
  bot: number
  dir: 'bull' | 'bear'
  state: string
}
interface SweepDessin {
  ts: number
  level: number
  dir: 'bull' | 'bear'
}

/** Flags de visibilité lus depuis settingsStore.indicateurs. */
interface FlagsV12 {
  tendance: boolean
  structure: boolean
  bos: boolean
  mss: boolean
  choch: boolean
  sweeps: boolean
  ob: boolean
  fvg: boolean
  signals: boolean
}

type TimeScale = ReturnType<IChartApi['timeScale']>
type KindLigne = 'bos' | 'mss' | 'choch'

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
  let tendance: 'haussiere' | 'baissiere' | 'neutre' = 'neutre'

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

    dessinerTendance(ctx, ts, W, H, tendance, flags, dernierTsRef)
    if (flags.ob || flags.fvg) dessinerObsEtFvgs(ctx, serieRef, ts, obs, fvgs, W, dernierTsRef, flags)
    if (flags.bos) dessinerLignes(ctx, serieRef, ts, bos, W, dernierTsRef, 'bos')
    if (flags.mss) dessinerLignes(ctx, serieRef, ts, mss, W, dernierTsRef, 'mss')
    if (flags.choch) dessinerLignes(ctx, serieRef, ts, chochs, W, dernierTsRef, 'choch')
    if (flags.sweeps) dessinerSweeps(ctx, serieRef, ts, sweeps, W)
    if (flags.signals) dessinerSignaux(ctx, serieRef, ts, signals, W, dernierTsRef)
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
      ts: b.ts, level: b.level, dir: b.dir,
      label: b.dir === 'bull' ? 'BOS ↑' : 'BOS ↓',
    }))
    mss = data.mss.map((m) => ({
      ts: m.ts, level: m.level, dir: m.dir,
      label: m.dir === 'bull' ? 'MSS ↑' : 'MSS ↓',
    }))
    chochs = data.chochs.map((c) => ({
      ts: c.ts, level: c.level, dir: c.dir,
      label: c.dir === 'bull' ? 'CHOCH ↑' : 'CHOCH ↓',
    }))
    sweeps = data.sweeps.map((s) => ({ ts: s.ts, level: s.level, dir: s.dir }))
    obs = data.obs.map((o) => ({
      ts: o.ts, top: o.top, bot: o.bot, force: o.force, dir: o.dir, state: o.state,
    }))
    fvgs = data.fvgs.map((f) => ({
      ts: f.ts, top: f.top, bot: f.bot, dir: f.dir, state: f.state,
    }))
    signals = data.signals.map((s) => ({
      ts: s.ts, entry: s.entry, sl: s.sl, tp1: s.tp1, dir: s.dir, force: s.force,
    }))
    tendance = data.tendance
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
    tendance = 'neutre'
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

  return { initialiser, charger, effacer, detruire, setDernierTs, redessiner: planifierRedessiner }
}

// ── Fonctions de dessin pures ─────────────────────────────────────────────────

/** Bord droit commun : dernière bougie si connue, sinon bord canvas. */
function xDroit(ts: TimeScale, W: number, dernierTs: number | null): number {
  if (dernierTs !== null) {
    const raw = ts.timeToCoordinate(dernierTs as any)
    if (raw !== null) return Math.min(raw, W - 4)
  }
  return W - 70
}

/** Bgcolor de tendance : teinte verte/rouge sur toute la zone visible. */
function dessinerTendance(
  ctx: CanvasRenderingContext2D,
  _ts: TimeScale,
  W: number,
  H: number,
  tendance: 'haussiere' | 'baissiere' | 'neutre',
  flags: FlagsV12,
  _dernierTs: number | null,
) {
  if (!flags.tendance) return
  if (tendance === 'neutre') return
  const hex = tendance === 'haussiere' ? COUL_TENDANCE_HAUSSE : COUL_TENDANCE_BAISSE
  ctx.fillStyle = hexVersRgba(hex, TENDANCE_ALPHA)
  ctx.fillRect(0, 0, W, H)
}

function dessinerObsEtFvgs(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  obsList: ObDessin[],
  fvgList: FvgDessin[],
  W: number,
  dernierTs: number | null,
  flags: FlagsV12,
) {
  const xD = xDroit(ts, W, dernierTs)
  // OB
  if (flags.ob) {
    for (const o of obsList) {
      const yHaut = serie.priceToCoordinate(o.top)
      const yBas = serie.priceToCoordinate(o.bot)
      if (yHaut === null || yBas === null) continue
      const yTop = Math.min(yHaut, yBas)
      const hauteur = Math.abs(yHaut - yBas)
      if (hauteur < 1) continue
      const xGRaw = ts.timeToCoordinate(o.ts as any)
      const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
      if (xD <= xG) continue
      const hex = o.dir === 'bull' ? COUL_OB_BULL : COUL_OB_BEAR
      const alpha = OB_ALPHA[o.state] ?? OB_ALPHA.vierge
      ctx.fillStyle = hexVersRgba(hex, alpha)
      ctx.fillRect(xG, yTop, xD - xG, hauteur)
      // Bordure = couleur sens transp 20.
      ctx.strokeStyle = hexVersRgba(hex, OB_BORD_ALPHA)
      ctx.lineWidth = 1.5
      ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
      ctx.beginPath(); ctx.moveTo(xG, yTop + hauteur); ctx.lineTo(xD, yTop + hauteur); ctx.stroke()
      ctx.lineWidth = 2
      ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xG, yTop + hauteur); ctx.stroke()
      ctx.font = 'bold 10px sans-serif'
      ctx.fillStyle = hexVersRgba(hex, 1)
      ctx.textAlign = 'left'
      ctx.textBaseline = 'top'
      ctx.fillText(`OB ${o.force}/10`, xG + 3, yTop + 2)
    }
  }
  // FVG
  if (flags.fvg) {
    for (const f of fvgList) {
      const yHaut = serie.priceToCoordinate(f.top)
      const yBas = serie.priceToCoordinate(f.bot)
      if (yHaut === null || yBas === null) continue
      const yTop = Math.min(yHaut, yBas)
      const hauteur = Math.abs(yHaut - yBas)
      if (hauteur < 1) continue
      const xGRaw = ts.timeToCoordinate(f.ts as any)
      const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
      if (xD <= xG) continue
      const hex = f.dir === 'bull' ? COUL_FVG_BULL : COUL_FVG_BEAR
      const alpha = FVG_ALPHA[f.state] ?? FVG_ALPHA.vierge
      ctx.fillStyle = hexVersRgba(hex, alpha)
      ctx.fillRect(xG, yTop, xD - xG, hauteur)
      // Bordure blanche transp 85.
      ctx.strokeStyle = hexVersRgba('#FFFFFF', FVG_BORD_ALPHA)
      ctx.lineWidth = 1
      ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
      ctx.beginPath(); ctx.moveTo(xG, yTop + hauteur); ctx.lineTo(xD, yTop + hauteur); ctx.stroke()
    }
  }
}

function dessinerLignes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  lignesList: LigneDessin[],
  W: number,
  dernierTs: number | null,
  kind: KindLigne,
) {
  const xD = xDroit(ts, W, dernierTs)
  const style = styleLigne(kind)
  for (const l of lignesList) {
    const y = serie.priceToCoordinate(l.level)
    if (y === null) continue
    const xGRaw = ts.timeToCoordinate(l.ts as any)
    const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
    const couleur = l.dir === 'bull' ? style.bull : style.bear
    ctx.strokeStyle = couleur
    ctx.lineWidth = style.width
    ctx.setLineDash(style.dashed ? [6, 4] : [])
    if (xD > xG) {
      ctx.beginPath(); ctx.moveTo(xG, y); ctx.lineTo(xD, y); ctx.stroke()
    }
    ctx.setLineDash([])
    if (l.label) {
      ctx.font = 'bold 10px sans-serif'
      ctx.fillStyle = couleur
      ctx.textAlign = 'right'
      ctx.textBaseline = 'bottom'
      ctx.fillText(l.label, Math.min(xD - 2, W - 4), y - 2)
    }
  }
}

function styleLigne(kind: KindLigne): { bull: string; bear: string; width: number; dashed: boolean } {
  if (kind === 'bos') return { bull: COUL_BOS_BULL, bear: COUL_BOS_BEAR, width: 2, dashed: false }
  if (kind === 'mss') return { bull: COUL_MSS_BULL, bear: COUL_MSS_BEAR, width: 2, dashed: true }
  return { bull: COUL_CHOCH_BULL, bear: COUL_CHOCH_BEAR, width: 3, dashed: false } // choch
}

function dessinerSweeps(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  sweepList: SweepDessin[],
  W: number,
) {
  for (const s of sweepList) {
    const y = serie.priceToCoordinate(s.level)
    if (y === null) continue
    const xRaw = ts.timeToCoordinate(s.ts as any)
    if (xRaw === null) continue
    const x = Math.max(4, Math.min(xRaw, W - 40))
    const couleur = s.dir === 'bull' ? COUL_SWEEP_BULL : COUL_SWEEP_BEAR
    const isHaut = s.dir === 'bear' // sweep baissier = prise de liquidité au-dessus
    ctx.font = 'bold 10px sans-serif'
    ctx.fillStyle = couleur
    ctx.textAlign = 'left'
    ctx.textBaseline = isHaut ? 'bottom' : 'top'
    const yTxt = isHaut ? y - 3 : y + 3
    ctx.fillText(s.dir === 'bull' ? 'SWEEP ↑' : 'SWEEP ↓', x + 3, yTxt)
    ctx.beginPath()
    ctx.arc(x, y, 2, 0, Math.PI * 2)
    ctx.fill()
  }
}

function dessinerSignaux(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  sigList: SignalDessin[],
  W: number,
  dernierTs: number | null,
) {
  const xD = xDroit(ts, W, dernierTs)
  for (const s of sigList) {
    const xGRaw = ts.timeToCoordinate(s.ts as any)
    const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
    if (xD <= xG) continue
    const yEntry = serie.priceToCoordinate(s.entry)
    const ySl = serie.priceToCoordinate(s.sl)
    const yTp = serie.priceToCoordinate(s.tp1)
    if (yEntry === null) continue
    // Box SL (entry ↔ sl) — transp 78.
    if (ySl !== null) {
      const yTop = Math.min(yEntry, ySl)
      const h = Math.abs(yEntry - ySl)
      ctx.fillStyle = hexVersRgba(COUL_SL, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xD - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_SL, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xD - xG, h)
    }
    // Box TP (entry ↔ tp1) — transp 78.
    if (yTp !== null) {
      const yTop = Math.min(yEntry, yTp)
      const h = Math.abs(yEntry - yTp)
      ctx.fillStyle = hexVersRgba(COUL_TP, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xD - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_TP, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xD - xG, h)
    }
    // Ligne entrée + label BUY/SELL.
    ctx.strokeStyle = COUL_ENTRY
    ctx.lineWidth = 1
    ctx.setLineDash([2, 3])
    ctx.beginPath(); ctx.moveTo(xG, yEntry); ctx.lineTo(xD, yEntry); ctx.stroke()
    ctx.setLineDash([])
    const txt = s.dir === 'Long' ? `BUY ${s.force}/10` : `SELL ${s.force}/10`
    ctx.font = 'bold 10px sans-serif'
    ctx.fillStyle = s.dir === 'Long' ? COUL_BUY : COUL_SELL
    ctx.textAlign = 'left'
    ctx.textBaseline = 'bottom'
    ctx.fillText(txt, xG + 3, yEntry - 2)
  }
}

function dessinerPivots(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  pivList: PivotDessin[],
  W: number,
) {
  for (const p of pivList) {
    const y = serie.priceToCoordinate(p.price)
    if (y === null) continue
    const xRaw = ts.timeToCoordinate(p.ts as any)
    if (xRaw === null) continue
    const x = Math.max(4, Math.min(xRaw, W - 30))
    const isHaut = p.type === 'HH' || p.type === 'LH'
    const couleur = p.type === 'HH' ? COUL_HH
      : p.type === 'HL' ? COUL_HL
        : p.type === 'LH' ? COUL_LH : COUL_LL
    ctx.font = 'bold 11px sans-serif'
    ctx.fillStyle = couleur
    ctx.textAlign = 'center'
    ctx.textBaseline = isHaut ? 'bottom' : 'top'
    const yTxt = isHaut ? y - 4 : y + 4
    ctx.fillText(p.type, x, yTxt)
    // Petit point pivot
    ctx.beginPath()
    ctx.arc(x, y, 2, 0, Math.PI * 2)
    ctx.fill()
  }
}
