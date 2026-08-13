/**
 * useSmcV12Overlay — overlay canvas dédié aux indicateurs SMC v12.
 *
 * Affiche, par-dessus le chart lightweight-charts :
 *   - labels de structure HH/HL/LH/LL positionnés sur les pivots réels ;
 *   - lignes BOS (continues) et MSS (tiretées) au niveau cassé ;
 *   - boxes OB (rectangle top→bot étendu vers la droite) ;
 *   - boxes FVG ;
 *   - boxes trade (SL/TP) + label pour les signaux.
 *
 * Le fetch des données se fait via getSmcV12Analyse ; on garde les données en
 * mémoire et on redessine à chaque scroll/zoom via l'API du chart.
 * z-index 4 — au-dessus des canvas SMC (2) / liquidités (3).
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { SmcV12Analyse } from '@/services/api.smc'
import { hexVersRgba } from './chartIndicatorsConfig'

// ── Palette (Pine smc_indicateur_v12) ─────────────────────────────────────────
const COUL_HH = '#00C853' // structure haussière forte
const COUL_HL = '#69F0AE'
const COUL_LH = '#FF5252' // structure baissière
const COUL_LL = '#D50000'

const COUL_BOS_BULL = '#2962FF'
const COUL_BOS_BEAR = '#FF6D00'

const COUL_OB_BULL = '#00C853'
const COUL_OB_BEAR = '#D50000'

const COUL_MSS_BULL = '#00BCD4' // cyan
const COUL_MSS_BEAR = '#FF9800' // orange

const COUL_FVG_BULL = '#2962FF'
const COUL_FVG_BEAR = '#FF6D00'

const COUL_SL = '#ef5350'
const COUL_TP = '#26a69a'
const COUL_ENTRY = '#3b82f6'

// Alpha = (100 - transparence_pine) / 100.
const OB_ALPHA: Record<string, number> = {
  vierge: (100 - 70) / 100,
  partiel: (100 - 83) / 100,
  profond: (100 - 91) / 100,
}

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
  dashed?: boolean
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

type TimeScale = ReturnType<IChartApi['timeScale']>

export function useSmcV12Overlay() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null
  let dernierTsRef: number | null = null

  let pivots: PivotDessin[] = []
  let lignes: LigneDessin[] = [] // BOS + MSS
  let obs: ObDessin[] = []
  let fvgs: FvgDessin[] = []
  let signals: SignalDessin[] = []

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

  // ── Rendu ───────────────────────────────────────────────────────────────────
  function redessiner() {
    if (!canvas || !chartRef || !serieRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const W = canvas.offsetWidth
    ctx.clearRect(0, 0, W, canvas.offsetHeight)
    const ts = chartRef.timeScale()

    dessinerObsetFvgs(ctx, serieRef, ts, obs, fvgs, W, dernierTsRef)
    dessinerLignes(ctx, serieRef, ts, lignes, W, dernierTsRef)
    dessinerSignaux(ctx, serieRef, ts, signals, W, dernierTsRef)
    dessinerPivots(ctx, serieRef, ts, pivots, W)
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
    lignes = [
      ...data.bos.map((b) => ({
        ts: b.ts, level: b.level, dir: b.dir,
        label: b.dir === 'bull' ? 'BOS ↑' : 'BOS ↓',
      })),
      ...data.mss.map((m) => ({
        ts: m.ts, level: m.level, dir: m.dir,
        label: m.dir === 'bull' ? 'MSS ↑' : 'MSS ↓', dashed: true,
      })),
    ]
    obs = data.obs.map((o) => ({
      ts: o.ts, top: o.top, bot: o.bot, force: o.force, dir: o.dir, state: o.state,
    }))
    fvgs = data.fvgs.map((f) => ({
      ts: f.ts, top: f.top, bot: f.bot, dir: f.dir, state: f.state,
    }))
    signals = data.signals.map((s) => ({
      ts: s.ts, entry: s.entry, sl: s.sl, tp1: s.tp1, dir: s.dir, force: s.force,
    }))
    planifierRedessiner()
  }

  function effacer() {
    pivots = []
    lignes = []
    obs = []
    fvgs = []
    signals = []
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

function dessinerObsetFvgs(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  obsList: ObDessin[],
  fvgList: FvgDessin[],
  W: number,
  dernierTs: number | null,
) {
  const xD = xDroit(ts, W, dernierTs)
  // OB
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
    ctx.strokeStyle = hexVersRgba(hex, Math.min(alpha * 4, 1))
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
  // FVG
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
    ctx.fillStyle = hexVersRgba(hex, f.state === 'vierge' ? 0.18 : 0.10)
    ctx.fillRect(xG, yTop, xD - xG, hauteur)
    ctx.strokeStyle = hexVersRgba(hex, 0.4)
    ctx.lineWidth = 1
    ctx.setLineDash([3, 3])
    ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
    ctx.beginPath(); ctx.moveTo(xG, yTop + hauteur); ctx.lineTo(xD, yTop + hauteur); ctx.stroke()
    ctx.setLineDash([])
  }
}

function dessinerLignes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  lignesList: LigneDessin[],
  W: number,
  dernierTs: number | null,
) {
  const xD = xDroit(ts, W, dernierTs)
  for (const l of lignesList) {
    const y = serie.priceToCoordinate(l.level)
    if (y === null) continue
    const xGRaw = ts.timeToCoordinate(l.ts as any)
    const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
    const couleur = couleurLigne(l)
    ctx.strokeStyle = couleur
    ctx.lineWidth = 2
    ctx.setLineDash(l.dashed ? [6, 4] : [])
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

function couleurLigne(l: LigneDessin): string {
  if (l.dashed) return l.dir === 'bull' ? COUL_MSS_BULL : COUL_MSS_BEAR
  return l.dir === 'bull' ? COUL_BOS_BULL : COUL_BOS_BEAR
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
    // Box SL (entry ↔ sl)
    if (ySl !== null) {
      const yTop = Math.min(yEntry, ySl)
      const h = Math.abs(yEntry - ySl)
      ctx.fillStyle = hexVersRgba(COUL_SL, 0.12)
      ctx.fillRect(xG, yTop, xD - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_SL, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xD - xG, h)
    }
    // Box TP (entry ↔ tp1)
    if (yTp !== null) {
      const yTop = Math.min(yEntry, yTp)
      const h = Math.abs(yEntry - yTp)
      ctx.fillStyle = hexVersRgba(COUL_TP, 0.12)
      ctx.fillRect(xG, yTop, xD - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_TP, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xD - xG, h)
    }
    // Ligne entrée + label
    ctx.strokeStyle = COUL_ENTRY
    ctx.lineWidth = 1
    ctx.setLineDash([2, 3])
    ctx.beginPath(); ctx.moveTo(xG, yEntry); ctx.lineTo(xD, yEntry); ctx.stroke()
    ctx.setLineDash([])
    const txt = s.dir === 'Long' ? `BUY ${s.force}/10` : `SELL ${s.force}/10`
    ctx.font = 'bold 10px sans-serif'
    ctx.fillStyle = COUL_ENTRY
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
