/**
 * smcV12OverlayExtraLignes — Phase 3 du rendu v12 étendu : lignes horizontales
 * (Asian HL, liquidités PDH/PDL/PWH/PWL, EQH/EQL, premium/discount +
 * equilibrium). Scindé de `smcV12OverlayExtra.ts` (règle < 600 lignes).
 *
 * Couleurs EXACTES du Pine smc_indicateur_v12. Helpers partagés (alpha `t`,
 * bord droit `xDroit`) et types importés du module parent.
 */
import type { ISeriesApi } from 'lightweight-charts'
import type { LiquiditeLevelV12 } from '@/services/api.smc'
import { hexVersRgba } from './chartIndicatorsConfig'
import { t, xDroit, type DonneesV12Etendues, type FlagsV12Etendus } from './smcV12OverlayExtra'

type TimeScale = ReturnType<import('lightweight-charts').IChartApi['timeScale']>

const COUL_PREMIUM = '#F44336'
const COUL_DISCOUNT = '#4CAF50'
const COUL_EQUILIBRIUM = '#FFD700'
const COUL_AH = '#FFD600'
const COUL_AL = '#FF6F00'
const COUL_GRIS = '#616161'
const COUL_LIQ_PDH = '#FFC107'
const COUL_LIQ_PDL = '#FF9800'
const COUL_LIQ_PWH = '#42A5F5'
const COUL_LIQ_PWL = '#26C6DA'
const COUL_EQH = '#FFD600'
const COUL_EQL = '#00BCD4'

// ════════════════════════════════════════════════════════════════════════════
// Phase 3 — LIGNES horizontales (z haut) + equilibrium
// ════════════════════════════════════════════════════════════════════════════

/** Asian HL + liquidités PDH/PDL/PWH/PWL + EQH/EQL + equilibrium. */
export function dessinerLignes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  W: number,
  d: DonneesV12Etendues,
  flags: FlagsV12Etendus,
  dernierTs: number | null,
): void {
  const xD = xDroit(ts, W, dernierTs)

  // Asian High / Low — Pine MODULE 14 : lignes du DÉBUT de session Asie
  // (Paris 00:00-06:30, _ahStartBar) au bord droit, couleurs exactes
  // #FFD600 (High) / #FF6F00 (Low), style SOLIDE (Pine line.new défaut),
  // labels 'Asian High' / 'Asian Low', gris si invalidé par close.
  if (flags.asianHl && d.asian_hl) {
    const ah = d.asian_hl
    const xGRaw = ah.start_ts ? ts.timeToCoordinate(ah.start_ts as any) : null
    const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
    const yH = serie.priceToCoordinate(ah.high)
    if (yH !== null) {
      const hex = ah.invalidated_up ? COUL_GRIS : COUL_AH
      ligneHoriz(ctx, hex, yH, xG, xD, {})
      labelDroite(ctx, hex, 'Asian High', xD, W, yH)
    }
    const yL = serie.priceToCoordinate(ah.low)
    if (yL !== null) {
      const hex = ah.invalidated_down ? COUL_GRIS : COUL_AL
      ligneHoriz(ctx, hex, yL, xG, xD, {})
      labelDroite(ctx, hex, 'Asian Low', xD, W, yL)
    }
  }

  // Liquidités PDH / PDL / PWH / PWL — Pine MODULE 2 : ligne du TIMESTAMP
  // où le niveau s'est formé (pas pleine largeur), dashed (day) / dotted
  // (week), labels complets "Previous Day High" etc.
  if (flags.niveauxCles) {
    const LABELS: Record<string, string> = {
      pdh: 'Previous Day High', pdl: 'Previous Day Low',
      pwh: 'Previous Week High', pwl: 'Previous Week Low',
    }
    for (const liq of d.liquidites) {
      if (liq.price == null || !liq.active) continue
      const y = serie.priceToCoordinate(liq.price)
      if (y === null) continue
      const { hex, dotted } = couleurLiquidite(liq.level)
      const xGRaw = liq.ts_origine ? ts.timeToCoordinate(liq.ts_origine as any) : null
      const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
      ligneHoriz(ctx, hex, y, xG, xD, { dashed: !dotted, dotted })
      labelDroite(ctx, hex, LABELS[liq.level] ?? liq.level.toUpperCase(), xD, W, y)
    }
  }

  // EQH / EQL — MODULE 4 Pine : dashed TOUJOURS (style_dashed, même sweepé),
  // largeur 2 si touches ≥ 3 sinon 1, ligne du 1er PIVOT (tFirst) au bord
  // droit — pas pleine largeur (Pine line.new(_ll.tFirst, ...)).
  if (flags.eqhEql) {
    for (const eq of d.eqs) {
      const y = serie.priceToCoordinate(eq.price)
      if (y === null) continue
      const isHigh = eq.dir === 'high'
      const hex = eq.swept ? COUL_GRIS : (isHigh ? COUL_EQH : COUL_EQL)
      const strong = eq.touches >= 3
      const xGRaw = eq.ts ? ts.timeToCoordinate(eq.ts as any) : null
      const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
      ligneHoriz(ctx, hex, y, xG, xD, { dashed: true, width: strong ? 2 : 1 })
      labelDroite(ctx, hex, `${isHigh ? 'EQH' : 'EQL'} ×${eq.touches}`, xD, W, y)
    }
  }

  // Premium/Discount — 3 lignes horizontales (standard ICT).
  const pdAffiche = flags.premium || flags.equilibrium
  if (pdAffiche && d.premium_discount) {
    const pd = d.premium_discount
    const pdLigne = (prix: number | null | undefined, hex: string, lbl: string, tr: number) => {
      if (prix == null) return
      const y = serie.priceToCoordinate(prix)
      if (y === null) return
      ligneHoriz(ctx, hexVersRgba(hex, t(tr)), y, 0, W, { dashed: true, width: 1 })
      labelDroite(ctx, hexVersRgba(hex, 1), lbl, W, W, y)
    }
    pdLigne(pd.equilibrium, COUL_EQUILIBRIUM, 'EQ', 20)
    if (flags.premium) { pdLigne(pd.pd_range_h, COUL_PREMIUM, 'PREMIUM', 40); pdLigne(pd.pd_range_l, COUL_DISCOUNT, 'DISCOUNT', 40) }
  }
}

// ── Helpers internes (lignes) ────────────────────────────────────────────────

/** Couleur + style de ligne d'un niveau de liquidité. */
function couleurLiquidite(level: LiquiditeLevelV12['level']): { hex: string; dotted: boolean } {
  switch (level) {
    case 'pdh': return { hex: COUL_LIQ_PDH, dotted: false }
    case 'pdl': return { hex: COUL_LIQ_PDL, dotted: false }
    case 'pwh': return { hex: COUL_LIQ_PWH, dotted: true }
    case 'pwl': return { hex: COUL_LIQ_PWL, dotted: true }
  }
}

/** Ligne horizontale dashed / dotted / solid. */
function ligneHoriz(
  ctx: CanvasRenderingContext2D,
  couleur: string,
  y: number,
  xG: number,
  xD: number,
  opts: { dashed?: boolean; dotted?: boolean; width?: number },
): void {
  ctx.strokeStyle = couleur
  ctx.lineWidth = opts.width ?? 1
  if (opts.dotted) ctx.setLineDash([2, 3])
  else if (opts.dashed) ctx.setLineDash([6, 4])
  else ctx.setLineDash([])
  ctx.beginPath(); ctx.moveTo(xG, y); ctx.lineTo(xD, y); ctx.stroke()
  ctx.setLineDash([])
}

/** Label à droite d'une ligne (aligné sur le bord visible). */
function labelDroite(
  ctx: CanvasRenderingContext2D,
  couleur: string,
  texte: string,
  xD: number,
  W: number,
  y: number,
): void {
  ctx.font = 'bold 10px sans-serif'
  ctx.fillStyle = couleur
  ctx.textAlign = 'right'
  ctx.textBaseline = 'bottom'
  ctx.fillText(texte, Math.min(xD - 2, W - 4), y - 2)
}
