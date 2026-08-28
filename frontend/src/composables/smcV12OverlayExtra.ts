/**
 * smcV12OverlayExtra — fonctions de dessin canvas pour les 13 indicateurs v12
 * « étendus » (tout ce qui n'est pas dans les 9 indicateurs de base de
 * useSmcV12Overlay : tendance, structure, BOS/MSS/CHOCH, sweeps, OB, FVG,
 * signals).
 *
 * Couleurs EXACTES du Pine smc_indicateur_v12 (voir spec). Transparence Pine
 * `transp N` → alpha = (100 - N) / 100.
 *
 * Les noms de champs des données correspondent au JSON backend réel
 * (`smc_v12_out.rs`), pas à la nomenclature idéalisée : snake_case,
 * `sessions[].session`, `gaps[].gtype`, `mtf_obs[].timeframe`, `eqs[].dir`
 * ("high"|"low"), `impulsions[].impulsion`, etc.
 *
 * Trois phases de rendu, appelées dans cet ordre par useSmcV12Overlay :
 *   1. dessinerFonds   — bgcolors verticaux (z bas) ;
 *   2. dessinerBoxes   — rectangles (z moyen) ;
 *   3. dessinerLignes  — lignes horizontales + equilibrium (z haut)
 *      → scindé dans `smcV12OverlayExtraLignes.ts` (règle < 600 lignes).
 */
import type { ISeriesApi } from 'lightweight-charts'
import type {
  AsianHlV12,
  BprV12,
  BreakerV12,
  EqV12,
  GapV12,
  HtfObV12,
  ImbalanceV12,
  ImpRangeV12,
  LiquiditeLevelV12,
  OteV12,
  PremiumDiscountV12,
  SessionRangeV12,
  VolRangeV12,
  ZoneCoeurV12,
} from '@/services/api.smc'
import { hexVersRgba } from './chartIndicatorsConfig'

type TimeScale = ReturnType<import('lightweight-charts').IChartApi['timeScale']>

/** Alpha d'une transparence Pine (transp 90 → 0.1). */
export const t = (transp: number): number => (100 - transp) / 100

// ── Palette v12 (transparence inline pour clarté) ────────────────────────────

const COUL_VOL = '#2962FF'

const COUL_IMP_BULL = '#00E676'
const COUL_IMP_BEAR = '#FF1744'

const COUL_NDOG = '#26C6DA'
const COUL_NWOG = '#AB47BC'

const COUL_BRK_BULL = '#00C853'
const COUL_BRK_BEAR = '#D50000'

const COUL_IB_BULL = '#00C853'
const COUL_IB_BEAR = '#D50000'

const COUL_OTE_BULL = '#00C853'
const COUL_OTE_BEAR = '#D50000'

// BPR Module 6b (Pine : ambre support / orange résistance, transp 88).
const COUL_BPR_BULL = '#FFB300'
const COUL_BPR_BEAR = '#FF6D00'

const COUL_ZC_BULL_F = '#00E676'
const COUL_ZC_BULL_B = '#00C853'
const COUL_ZC_BEAR_F = '#FF1744'
const COUL_ZC_BEAR_B = '#D50000'

// Palette HTF OB : { bull, bear, transp } par timeframe.
const HTF: Record<'H1' | 'H4' | 'W1' | 'MN', { bull: string; bear: string; transp: number }> = {
  H1: { bull: '#00BCD4', bear: '#FF6F00', transp: 70 },
  H4: { bull: '#1565C0', bear: '#B71C1C', transp: 60 },
  W1: { bull: '#006064', bear: '#4A148C', transp: 50 },
  MN: { bull: '#F57F17', bear: '#880E4F', transp: 45 },
}

/** Flags de visibilité des indicateurs étendus (lus depuis settingsStore). */
export interface FlagsV12Etendus {
  sessionAsie: boolean
  sessionLondres: boolean
  sessionNy: boolean
  eqhEql: boolean
  asianHl: boolean
  niveauxCles: boolean
  ndog: boolean
  nwog: boolean
  breaker: boolean
  propulsion: boolean
  imbalance: boolean
  bpr: boolean
  ote: boolean
  premium: boolean
  equilibrium: boolean
  obH1: boolean
  obH4: boolean
  obW1: boolean
  obMn: boolean
  zoneCoeur: boolean
  volume: boolean
  impulsion: boolean
}

/** Données des 13 indicateurs étendus (vide = rien à dessiner). */
export interface DonneesV12Etendues {
  propulsions: import('@/services/api.smc').PropulsionV12[]
  sessions: SessionRangeV12[]
  trend_ranges: import('@/services/api.smc').TrendRange[]
  prem_ranges: import('@/services/api.smc').PremRange[]
  session_boxes: import('@/services/api.smc').SessionBox[]
  vol_fort: VolRangeV12[]
  impulsions: ImpRangeV12[]
  premium_discount: PremiumDiscountV12 | null
  asian_hl: AsianHlV12 | null
  liquidites: LiquiditeLevelV12[]
  eqs: EqV12[]
  gaps: GapV12[]
  breakers: BreakerV12[]
  imbalances: ImbalanceV12[]
  bprs: BprV12[]
  otes: OteV12[]
  mtf_obs: HtfObV12[]
  zone_coeur: ZoneCoeurV12[]
}

export const donneesV12EtenduesVides = (): DonneesV12Etendues => ({
  sessions: [],
  trend_ranges: [],
  prem_ranges: [],
  session_boxes: [],
  vol_fort: [],
  impulsions: [],
  premium_discount: null,
  asian_hl: null,
  liquidites: [],
  eqs: [],
  gaps: [],
  breakers: [],
  propulsions: [],
  imbalances: [],
  bprs: [],
  otes: [],
  mtf_obs: [],
  zone_coeur: [],
})

/** Bord droit commun : dernière bougie si connue, sinon bord canvas. */
export function xDroit(ts: TimeScale, W: number, dernierTs: number | null): number {
  if (dernierTs !== null) {
    const raw = ts.timeToCoordinate(dernierTs as any)
    if (raw !== null) return Math.min(raw, W - 4)
  }
  return W - 70
}

// ════════════════════════════════════════════════════════════════════════════
// Phase 1 — FONDS (bgcolors verticaux, z le plus bas)
// ════════════════════════════════════════════════════════════════════════════

/** Sessions Kill Zones + volume fort + impulsions + premium/discount. */
export function dessinerFonds(
  ctx: CanvasRenderingContext2D,
  ts: TimeScale,
  W: number,
  H: number,
  serie: ISeriesApi<'Candlestick'>,
  d: DonneesV12Etendues,
  flags: FlagsV12Etendus,
  dernierTs: number | null,
): void {
  // (Sessions : PLUS de bandes verticales — uniquement les rectangles du
  //  MODULE 14 Pine, dessinés dans dessinerBoxes via session_boxes.)

  // Volume fort — fond vertical.
  if (flags.volume) {
    for (const r of d.vol_fort) {
      const xG = coordX(ts, r.start_ts, 0)
      const xD = coordX(ts, r.end_ts, W)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(COUL_VOL, t(82))
      ctx.fillRect(xG, 0, xD - xG, H)
    }
  }

  // Impulsions — fond vertical bull/bear.
  if (flags.impulsion) {
    for (const r of d.impulsions) {
      const hex = r.impulsion === 'bull' ? COUL_IMP_BULL : COUL_IMP_BEAR
      const xG = coordX(ts, r.start_ts, 0)
      const xD = coordX(ts, r.end_ts, W)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hex, t(75))
      ctx.fillRect(xG, 0, xD - xG, H)
    }
  }

}

// ════════════════════════════════════════════════════════════════════════════
// Phase 2 — BOXES (rectangles, z moyen)
// ════════════════════════════════════════════════════════════════════════════

/** NDOG/NWOG + MTF OB + zone-cœur + breaker + imbalance + OTE. */
export function dessinerBoxes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  W: number,
  d: DonneesV12Etendues,
  flags: FlagsV12Etendus,
  dernierTs: number | null,
): void {
  // ── Propulsion Blocks (Pine MODULE 8c : FVG ∩ OB même sens) : boxes
  //    vert/rouge estompé (#00C853/#D50000 α93), bord gauche = création.
  if (flags.propulsion && d.propulsions) {
    const xD2 = xDroit(ts, W, dernierTs)
    for (const p of d.propulsions) {
      const yT = serie.priceToCoordinate(p.top)
      const yB = serie.priceToCoordinate(p.bot)
      if (yT === null || yB === null) continue
      const xGRaw = ts.timeToCoordinate(p.ts as any)
      if (xGRaw === null) continue
      const hex = p.dir === 'bull' ? '#00C853' : '#D50000'
      const yTop = Math.min(yT, yB)
      ctx.fillStyle = hexVersRgba(hex, 0.07)
      ctx.fillRect(xGRaw, yTop, xD2 - xGRaw, Math.abs(yT - yB))
      ctx.strokeStyle = hexVersRgba(hex, 0.4)
      ctx.lineWidth = 1
      ctx.strokeRect(xGRaw, yTop, xD2 - xGRaw, Math.abs(yT - yB))
      ctx.font = 'bold 9px sans-serif'
      ctx.fillStyle = hex
      ctx.textAlign = 'right'
      ctx.textBaseline = 'top'
      ctx.fillText('PROP', xD2 - 3, yTop + 2)
    }
  }

  /// Position x d'un instant : barre exacte si possible, sinon
  /// interpolation sur la plage visible (les bords de session tombant
  /// dans un vide de données — marché fermé — restent ainsi à leur
  /// heure Paris réelle).
  const xTemporel = (t: number): number | null => {
    const direct = ts.timeToCoordinate(t as never)
    if (direct !== null) return direct
    const vr = ts.getVisibleRange()
    const lr = ts.getVisibleLogicalRange()
    if (!vr || !lr || (vr.to as number) <= (vr.from as number)) return null
    const logique = (lr.from as number) + ((t - (vr.from as number)) / ((vr.to as number) - (vr.from as number))) * ((lr.to as number) - (lr.from as number))
    return ts.logicalToCoordinate(Math.round(logique) as never) ?? null
  }
  // Sessions MODULE 14 : rectangles range high/low (α90, bordure fine).
  if (flags.sessionAsie || flags.sessionLondres || flags.sessionNy) {
    const COUL_SESSION: Record<string, string> = { asie: '#F9A825', londres: '#1565C0', ny: '#B71C1C' }
    for (const s of d.session_boxes) {
      const visible = s.session === 'asie' ? flags.sessionAsie : s.session === 'londres' ? flags.sessionLondres : flags.sessionNy
      if (!visible) continue
      const yH = serie.priceToCoordinate(s.high)
      const yL = serie.priceToCoordinate(s.low)
      if (yH === null || yL === null) continue // prix hors échelle
      const x1 = xTemporel(s.start_ts)
      const x2 = xTemporel(s.end_ts)
      if (x1 === null && x2 === null) continue
      const borneDroite = xDroit(ts, W, dernierTs)
      const gauche = x1 !== null ? Math.max(0, x1) : 0
      const droite = Math.min(x2 !== null ? x2 : borneDroite, borneDroite)
      if (droite <= gauche) continue // hors écran
      const coul = COUL_SESSION[s.session] ?? '#666666'
      const yTop = Math.min(yH, yL); const h = Math.abs(yH - yL)
      ctx.fillStyle = hexVersRgba(coul, 0.1) // α90 Pine
      ctx.fillRect(gauche, yTop, droite - gauche, h)
      ctx.strokeStyle = 'rgba(255,255,255,0.15)'; ctx.lineWidth = 1 // α85
      ctx.strokeRect(gauche, yTop, droite - gauche, h)
      ctx.font = 'bold 9px sans-serif'
      ctx.fillStyle = coul; ctx.textAlign = 'right'; ctx.textBaseline = 'top'
      const NOMS: Record<string, string> = { asie: 'Session Asiatique', londres: 'Session Européenne', ny: 'Session Américaine' }
      ctx.fillText(NOMS[s.session] ?? s.session, droite - 4, yTop + 2)
    }
  }
  const xD = xDroit(ts, W, dernierTs)

  // NDOG / NWOG (+ ligne CE 50% — réaction ICT).
  for (const g of d.gaps) {
    if (g.gtype === 'ndog' && !flags.ndog) continue
    if (g.gtype === 'nwog' && !flags.nwog) continue
    const hex = g.gtype === 'ndog' ? COUL_NDOG : COUL_NWOG
    const transp = g.mitigated ? 92 : 75
    const box = boxPrix(serie, g.top, g.bot)
    if (!box) continue
    const xG = coordX(ts, g.ts, 0)
    if (xD <= xG) continue
    ctx.fillStyle = hexVersRgba(hex, t(transp))
    ctx.fillRect(xG, box.yTop, xD - xG, box.h)
    tracerBordsBox(ctx, hexVersRgba(hex, 1), xG, xD, box.yTop, box.h)
    // Pine : texte « New Day/Week Opening Gap » (tiny, haut droite).
    labelBox(ctx, hexVersRgba(hex, 1), g.gtype === 'ndog' ? 'New Day Opening Gap' : 'New Week Opening Gap', xD, box.yTop)
    // CE = 50% du gap.
    const ce = (g as { ce?: number }).ce
    if (ce !== undefined) {
      const yCe = serie.priceToCoordinate(ce)
      if (yCe !== null) {
        ctx.strokeStyle = hexVersRgba(hex, 1); ctx.lineWidth = 1; ctx.setLineDash([4, 3])
        ctx.beginPath(); ctx.moveTo(xG, yCe); ctx.lineTo(xD, yCe); ctx.stroke(); ctx.setLineDash([])
        ctx.font = '8px sans-serif'; ctx.fillStyle = hexVersRgba(hex, 1); ctx.textAlign = 'right'; ctx.textBaseline = 'bottom'
        ctx.fillText('CE', xD - 3, yCe - 1)
      }
    }
  }

  // MTF OB (H1/H4/W1/MN).
  for (const o of d.mtf_obs) {
    if (o.timeframe === 'H1' && !flags.obH1) continue
    if (o.timeframe === 'H4' && !flags.obH4) continue
    if (o.timeframe === 'W1' && !flags.obW1) continue
    if (o.timeframe === 'MN' && !flags.obMn) continue
    const pal = HTF[o.timeframe]
    const hex = o.dir === 'bull' ? pal.bull : pal.bear
    const box = boxPrix(serie, o.top, o.bot)
    if (!box) continue
    const xG = coordX(ts, o.ts, 0)
    if (xD <= xG) continue
    ctx.fillStyle = hexVersRgba(hex, t(pal.transp))
    ctx.fillRect(xG, box.yTop, xD - xG, box.h)
    tracerBordsBox(ctx, hexVersRgba(hex, 1), xG, xD, box.yTop, box.h)
    const emoji = o.dir === 'bull' ? '🟢' : '🔴'
    // Pine f_htfTag : « ◀ ici » si le close courant est DANS la zone.
    const c = dernierClose(serie)
    const ici = c !== null && c >= o.bot && c <= o.top ? ' ◀ ici' : ''
    labelBox(ctx, hexVersRgba(hex, 1), `${emoji} ${o.timeframe}${ici}`, xD, box.yTop)
  }

  // Zone-cœur — bord gauche = bougie d'origine de l'OB parent (Pine
  // box.new(obBullBar[_zi], …)), pas la barre de détection.
  if (flags.zoneCoeur) {
    for (const z of d.zone_coeur) {
      const isBull = z.dir === 'bull'
      const hexF = isBull ? COUL_ZC_BULL_F : COUL_ZC_BEAR_F
      const hexB = isBull ? COUL_ZC_BULL_B : COUL_ZC_BEAR_B
      const box = boxPrix(serie, z.top, z.bot)
      if (!box) continue
      const xG = coordX(ts, z.ob_ts > 0 ? z.ob_ts : z.ts, 0)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hexF, t(20))
      ctx.fillRect(xG, box.yTop, xD - xG, box.h)
      tracerBordsBox(ctx, hexVersRgba(hexB, 1), xG, xD, box.yTop, box.h)
      labelBox(ctx, hexVersRgba(hexB, 1), isBull ? 'Zone Achat' : 'Zone Vente', xD, box.yTop)
    }
  }

  // Breaker.
  if (flags.breaker) {
    for (const b of d.breakers) {
      const hex = b.dir === 'bull' ? COUL_BRK_BULL : COUL_BRK_BEAR
      const box = boxPrix(serie, b.top, b.bot)
      if (!box) continue
      const xG = coordX(ts, b.ts, 0)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hex, t(93))
      ctx.fillRect(xG, box.yTop, xD - xG, box.h)
      tracerBordsBox(ctx, hexVersRgba('#FFFFFF', t(60)), xG, xD, box.yTop, box.h)
      labelBox(ctx, hexVersRgba(hex, 1), 'BREAKER', xD, box.yTop)
    }
  }

  // Imbalance.
  if (flags.imbalance) {
    for (const im of d.imbalances) {
      const hex = im.dir === 'bull' ? COUL_IB_BULL : COUL_IB_BEAR
      const box = boxPrix(serie, im.top, im.bot)
      if (!box) continue
      const xG = coordX(ts, im.ts, 0)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hex, t(93))
      ctx.fillRect(xG, box.yTop, xD - xG, box.h)
      tracerBordsBox(ctx, hexVersRgba('#FFFFFF', t(85)), xG, xD, box.yTop, box.h)
      labelBox(ctx, hexVersRgba(hex, 1), 'Imbalance', xD, box.yTop)
    }
  }

  // BPR (Module 6b) — box ambre (bull/support) ou orange (bear/résistance),
  // CE en pointillés, s'arrête à la bougie en cours. Figé (dead) = grisé
  // (Pine : freeze LuxAlgo, jamais supprimé avant éviction FIFO 20).
  if (flags.bpr) {
    for (const b of d.bprs) {
      const hex = b.dead ? '#9E9E9E' : b.dir === 'bull' ? COUL_BPR_BULL : COUL_BPR_BEAR
      const box = boxPrix(serie, b.top, b.bot)
      if (!box) continue
      const xG = coordX(ts, b.ts, 0)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hex, t(88))
      ctx.fillRect(xG, box.yTop, xD - xG, box.h)
      tracerBordsBox(ctx, hexVersRgba('#FFFFFF', t(80)), xG, xD, box.yTop, box.h)
      labelBox(ctx, hexVersRgba(hex, 1), 'BPR', xD, box.yTop)
      // CE (consequent encroachment) = mi-range, pointillée (Pine bprCeLine).
      const yCe = serie.priceToCoordinate(b.ce)
      if (yCe !== null) {
        ctx.strokeStyle = hexVersRgba(hex, t(40))
        ctx.lineWidth = 1
        ctx.setLineDash([4, 3])
        ctx.beginPath()
        ctx.moveTo(xG, yCe)
        ctx.lineTo(xD, yCe)
        ctx.stroke()
        ctx.setLineDash([])
      }
    }
  }

  // OTE — box d'affichage Pine _oteBullBox/_oteBearBox : créée au BOS
  // (bord gauche = bar du BOS), persiste après expiration de la plage.
  if (flags.ote) {
    for (const o of d.otes) {
      const hex = o.dir === 'bull' ? COUL_OTE_BULL : COUL_OTE_BEAR
      const box = boxPrix(serie, o.top, o.bot)
      if (!box) continue
      const xG = coordX(ts, o.ts, 0)
      if (xD <= xG) continue
      ctx.fillStyle = hexVersRgba(hex, t(80))
      ctx.fillRect(xG, box.yTop, xD - xG, box.h)
      tracerBordsBox(ctx, hexVersRgba(hex, t(40)), xG, xD, box.yTop, box.h)
      labelBox(ctx, hexVersRgba(hex, 1), 'OTE', xD, box.yTop)
    }
  }
}

// ── Helpers internes ─────────────────────────────────────────────────────────

/** timeToCoordinate avec borne et fallback (null → fallback). */
function coordX(ts: TimeScale, time: number, fallback: number): number {
  const raw = ts.timeToCoordinate(time as any)
  return raw !== null ? raw : fallback
}
/** Dernier close de la série (Pine `close` pour le tag « ◀ ici »). */
function dernierClose(serie: ISeriesApi<'Candlestick'>): number | null {
  const data = serie.data()
  const last = data.length > 0 ? data[data.length - 1] : null
  const c = (last as { close?: number } | null)?.close
  return typeof c === 'number' ? c : null
}

/** Coordonnées canvas d'une box prix [top, bot] (yTop = plus petit y). */
function boxPrix(
  serie: ISeriesApi<'Candlestick'>,
  top: number,
  bot: number,
): { yTop: number; h: number } | null {
  const yH = serie.priceToCoordinate(top)
  const yB = serie.priceToCoordinate(bot)
  if (yH === null || yB === null) return null
  const yTop = Math.min(yH, yB)
  const h = Math.abs(yH - yB)
  if (h < 1) return null
  return { yTop, h }
}

/** Trace les bords haut + bas d'une box (style TradingView). */
function tracerBordsBox(
  ctx: CanvasRenderingContext2D,
  couleur: string,
  xG: number,
  xD: number,
  yTop: number,
  h: number,
): void {
  ctx.strokeStyle = couleur
  ctx.lineWidth = 1
  ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
  ctx.beginPath(); ctx.moveTo(xG, yTop + h); ctx.lineTo(xD, yTop + h); ctx.stroke()
}

/** Label en haut à DROITE d'une box (Pine C_BLOC_TXT_HALIGN = text.align_right,
 *  C_BLOC_TXT_VALIGN = text.align_top ; MQL5 SmcBlockLabel ANCHOR_RIGHT_UPPER). */
function labelBox(
  ctx: CanvasRenderingContext2D,
  couleur: string,
  texte: string,
  xD: number,
  yTop: number,
): void {
  ctx.font = 'bold 10px sans-serif'
  ctx.fillStyle = couleur
  ctx.textAlign = 'right'
  ctx.textBaseline = 'top'
  ctx.fillText(texte, xD - 3, yTop + 2)
}

