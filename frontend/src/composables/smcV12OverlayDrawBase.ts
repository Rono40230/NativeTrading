/**
 * smcV12OverlayDrawBase — fonctions de dessin canvas pour les 9 indicateurs
 * v12 « de base » (structure, BOS/MSS/CHOCH, sweeps, OB, FVG, signals,
 * tendance) + types/constantes partagés.
 *
 * Extrait de useSmcV12Overlay.ts pour respecter la règle vibe (< 600 lignes/
 * fichier). Les indicateurs « étendus » (13 types supplémentaires) vivent dans
 * smcV12OverlayExtra.ts.
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { hexVersRgba } from './chartIndicatorsConfig'

export type TimeScale = ReturnType<IChartApi['timeScale']>
export type KindLigne = 'bos' | 'mss' | 'choch'

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

// ── Types de données de dessin ────────────────────────────────────────────────
export interface ObDessin {
  ts: number
  top: number
  bot: number
  force: number
  dir: 'bull' | 'bear'
  state: string
}
export interface LigneDessin {
  ts: number
  /** Timestamp du pivot cassé (borne de DÉBUT de la ligne). 0 = point unique (sweep). */
  pivot_ts: number
  level: number
  dir: 'bull' | 'bear'
  label: string
}
export interface SignalDessin {
  ts: number
  entry: number
  sl: number
  tp1: number
  dir: 'Long' | 'Short'
  force: number
}
export interface PivotDessin {
  ts: number
  price: number
  type: 'HH' | 'HL' | 'LH' | 'LL'
}
export interface FvgDessin {
  ts: number
  top: number
  bot: number
  dir: 'bull' | 'bear'
  state: string
}
export interface SweepDessin {
  ts: number
  level: number
  dir: 'bull' | 'bear'
}

/** Flags de visibilité des 9 indicateurs de base. */
export interface FlagsV12 {
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

// ── Fonctions de dessin pures ─────────────────────────────────────────────────

/** Bord droit commun : dernière bougie si connue, sinon bord canvas. */
export function xDroit(ts: TimeScale, W: number, dernierTs: number | null): number {
  if (dernierTs !== null) {
    const raw = ts.timeToCoordinate(dernierTs as any)
    if (raw !== null) return Math.min(raw, W - 4)
  }
  return W - 70
}

/** Bgcolor de tendance : teinte verte/rouge sur toute la zone visible. */
export function dessinerTendance(
  ctx: CanvasRenderingContext2D,
  _ts: TimeScale,
  W: number,
  H: number,
  tendance: 'haussiere' | 'baissiere' | 'neutre',
  flags: FlagsV12,
  _dernierTs: number | null,
): void {
  if (!flags.tendance) return
  if (tendance === 'neutre') return
  const hex = tendance === 'haussiere' ? COUL_TENDANCE_HAUSSE : COUL_TENDANCE_BAISSE
  ctx.fillStyle = hexVersRgba(hex, TENDANCE_ALPHA)
  ctx.fillRect(0, 0, W, H)
}

export function dessinerObsEtFvgs(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  obsList: ObDessin[],
  fvgList: FvgDessin[],
  W: number,
  dernierTs: number | null,
  flags: FlagsV12,
): void {
  const xD = xDroit(ts, W, dernierTs)
  // OB — filtre d'affichage identique au Pine (lignes 1238/1315 : seules les
  // zones de force ≥ 5/10 sont dessinées sur TV). Le moteur, lui, qualifie
  // toujours ses trades dès force ≥ 4 (i_forceMin Pine) : ce filtre ne
  // concerne QUE le rendu, pas la détection.
  if (flags.ob) {
    for (const o of obsList) {
      if (o.force < 5) continue
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
      // Score à droite de la zone (bord actif), aligné sur l'affichage TV.
      ctx.textAlign = 'right'
      ctx.textBaseline = 'top'
      ctx.fillText(`OB ${o.force}/10`, xD - 3, yTop + 2)
      ctx.textAlign = 'left'
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

export function dessinerLignes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  lignesList: LigneDessin[],
  W: number,
  _dernierTs: number | null,
  kind: KindLigne,
): void {
  const style = styleLigne(kind)
  for (const l of lignesList) {
    const y = serie.priceToCoordinate(l.level)
    if (y === null) continue
    // Segment BORNÉ : X de fin = cassure (event ts), X de début = pivot cassé.
    const xDRaw = ts.timeToCoordinate(l.ts as any)
    if (xDRaw === null) continue
    const xD: number = xDRaw
    // pivot_ts == 0 (sweep / pivot absent) → point unique : début = fin (label seul).
    let xG: number = xD
    if (l.pivot_ts !== 0) {
      const xGRaw = ts.timeToCoordinate(l.pivot_ts as any)
      xG = xGRaw !== null ? Math.max(0, xGRaw) : xD
    }
    const couleur = l.dir === 'bull' ? style.bull : style.bear
    ctx.strokeStyle = couleur
    ctx.lineWidth = style.width
    ctx.setLineDash(style.dashed ? [6, 4] : [])
    if (xG < xD) {
      ctx.beginPath(); ctx.moveTo(xG, y); ctx.lineTo(xD, y); ctx.stroke()
    }
    ctx.setLineDash([])
    // Label à l'extrémité de la cassure (X de fin).
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

export function dessinerSweeps(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  sweepList: SweepDessin[],
  W: number,
): void {
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

export function dessinerSignaux(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  sigList: SignalDessin[],
  W: number,
  dernierTs: number | null,
): void {
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

export function dessinerPivots(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  pivList: PivotDessin[],
  W: number,
): void {
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
