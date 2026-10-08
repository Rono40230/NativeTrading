//! Dessin de la couche « OB Institutionnels » — spec
//! docs/spec_indicateur_unifie_ob_ote.md § 2.4 (rétro-ingénierie SMC
//! Institutional Scalper v1.3, KASPER Trading) :
//!   - bande OTE swing 61,8–78,6 % de la jambe (née à la confirmation du
//!     pivot le plus récent, s'étend à droite tant que vivante) ;
//!   - trait 50 % cyan pointillé ;
//!   - ancres : disque violet (pivot haut) / vert (pivot bas) + fines lignes
//!     horizontales oranges aux deux niveaux, sur la durée de vie de l'OTE.
//!
//! Phase B3 : dessin en PARALLÈLE de l'existant (aucun retrait). La dorure
//! des OB vit dans `dessinerObsEtFvgs` (champ `dore`).
import { hexVersRgba } from './chartIndicatorsConfig'
import { xDroit, type TimeScale } from './smcV12OverlayDrawBase'
import type { SwingOteV12 } from '@/services/api.smc'
import type { ISeriesApi } from 'lightweight-charts'

/** Bande gris-bleu (captures XAU M15/H2 du 08/10). */
const COUL_OTE = '#607D8B'
/** Trait 50 % cyan pointillé (capture M15 : trait cyan à 4126). */
const COUL_MID = '#26C6DA'
/** Disque ancre pivot haut (violet) / pivot bas (vert). */
const COUL_ANCRE_HAUT = '#AB47BC'
const COUL_ANCRE_BAS = '#66BB6A'
/** Lignes d'ancre horizontales fines (orange). */
const COUL_LIGNE_ANCRE = '#FFA726'

export function dessinerInstitutional(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  ote: SwingOteV12 | null,
  W: number,
  dernierTs: number | null,
  actif: boolean,
): void {
  if (!actif || !ote) return
  const xD = xDroit(ts, W, dernierTs)
  const xG = coordX(ts, ote.ts_naissance, 0)
  if (xD <= xG) return
  const yTopP = serie.priceToCoordinate(ote.top)
  const yBotP = serie.priceToCoordinate(ote.bot)
  if (yTopP === null || yBotP === null) return
  const yTop = Math.min(yTopP, yBotP)
  const h = Math.abs(yTopP - yBotP)

  // ── Bande OTE (61,8–78,6 %) + bordures fines ──
  ctx.fillStyle = hexVersRgba(COUL_OTE, 0.18)
  ctx.fillRect(xG, yTop, xD - xG, h)
  ctx.strokeStyle = hexVersRgba(COUL_OTE, 0.55)
  ctx.lineWidth = 1
  ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
  ctx.beginPath(); ctx.moveTo(xG, yTop + h); ctx.lineTo(xD, yTop + h); ctx.stroke()

  // ── Trait 50 % cyan pointillé + label ──
  const yMid = serie.priceToCoordinate(ote.mid)
  if (yMid !== null) {
    ctx.strokeStyle = COUL_MID
    ctx.setLineDash([4, 3])
    ctx.beginPath(); ctx.moveTo(xG, yMid); ctx.lineTo(xD, yMid); ctx.stroke()
    ctx.setLineDash([])
    ctx.font = 'bold 9px sans-serif'
    ctx.fillStyle = COUL_MID
    ctx.textAlign = 'left'
    ctx.textBaseline = 'bottom'
    ctx.fillText('OTE 50 %', xG + 3, yMid - 1)
  }

  // ── Ancres de jambe : lignes oranges (niveau pivot, sur la durée de vie)
  //    + disque sur la barre pivot. ──
  ancre(ctx, serie, ts, ote.pivot_haut, COUL_ANCRE_HAUT, xD)
  ancre(ctx, serie, ts, ote.pivot_bas, COUL_ANCRE_BAS, xD)
}

function ancre(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  a: { prix: number; ts: number },
  coulDisque: string,
  xD: number,
): void {
  const y = serie.priceToCoordinate(a.prix)
  if (y === null) return
  const xA = coordX(ts, a.ts, -1)
  if (xA >= 0 && xD > xA) {
    ctx.strokeStyle = hexVersRgba(COUL_LIGNE_ANCRE, 0.5)
    ctx.lineWidth = 1
    ctx.beginPath(); ctx.moveTo(xA, y); ctx.lineTo(xD, y); ctx.stroke()
  }
  // Disque (r=3) centré sur la barre pivot.
  ctx.fillStyle = coulDisque
  ctx.beginPath()
  ctx.arc(Math.max(xA, 3), y, 3, 0, Math.PI * 2)
  ctx.fill()
}

/** timeToCoordinate avec fallback (null → fallback, -1 = hors chart → skip). */
function coordX(ts: TimeScale, time: number, fallback: number): number {
  const raw = ts.timeToCoordinate(time as any)
  return raw !== null ? raw : fallback
}
