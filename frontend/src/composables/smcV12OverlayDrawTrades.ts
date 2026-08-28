//! Dessin des TRADES v12 : signaux remplis (fidélité Pine — boxes SL/TP,
//! lignes TP1/TP2, label f_lblTrade) + ordres/trades multi-TF (attente ⏳).
//! Extrait de smcV12OverlayDrawBase (limite 600 lignes).

import { hexVersRgba } from './chartIndicatorsConfig'
import { xDroit, type TimeScale } from './smcV12OverlayDrawBase'
import type { ISeriesApi } from 'lightweight-charts'

const COUL_SL = '#ef5350'
const COUL_TP = '#26a69a'
const COUL_TP1_L = '#26a69a'
const COUL_TP2_L = '#66bb6a'
const COUL_ENTRY = '#3b82f6'
const COUL_BUY = '#1b5e20'
const COUL_SELL = '#b71c1c'

export interface SignalDessin {
  ts: number
  entry: number
  sl: number
  tp1: number
  tp2: number
  tp3: number
  dir: 'Long' | 'Short'
  force: number
  /// BE armé (TP1 touché ou forcé) — le Pine supprime box SL + label.
  be: boolean
  /// Lignes du label f_lblTrade (Pine 2503) : force, lot + $ risqués, niveaux.
  label: string[]
}

export function dessinerSignaux(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  sigList: SignalDessin[],
  W: number,
  dernierTs: number | null,
): void {
  // Fidélité Pine v12 (stBull*/stBear*, lignes 3571-3590 + 3895-3930) :
  //   création INVISIBLE (bgcolor=na) → matérialisation au FILL (boxes SL +
  //   TP entry↔TP3, lignes TP1/TP2 solides, label f_lblTrade complet) →
  //   BE/TP1 : box SL et label SUPPRIMÉES → clôture : tout supprimé.
  // (Les trades en attente et clôturés sont déjà filtrés en amont.)
  const xD = xDroit(ts, W, dernierTs)
  // px/seconde pour extrapoler les bords droits futurs (tsFin > dernière bougie).
  let pxParSec = 0
  const vr = ts.getVisibleRange()
  if (vr) {
    const xf = ts.timeToCoordinate(vr.from as any)
    const xt = ts.timeToCoordinate(vr.to as any)
    const d = (vr.to as number) - (vr.from as number)
    if (xf !== null && xt !== null && d > 0) pxParSec = (xt - xf) / d
  }
  for (const s of sigList) {
    const xGRaw = ts.timeToCoordinate(s.ts as any)
    if (xGRaw === null) continue
    const xG = Math.max(0, xGRaw)
    if (xD <= xG) continue
    // Bord droit FINI du trade (Pine i_tpWidth : 40 barres, H1 30, H4 20).
    const tsFin = (s as { tsFin?: number }).tsFin
    let xFin = xD
    if (tsFin !== undefined && pxParSec > 0) {
      const direct = ts.timeToCoordinate(tsFin as any)
      xFin = direct !== null ? direct : xG + (tsFin - s.ts) * pxParSec
    }
    if (xFin <= xG) continue
    const xDTrade = Math.min(xD, xFin)
    const yEntry = serie.priceToCoordinate(s.entry)
    const ySl = serie.priceToCoordinate(s.sl)
    const yTp1 = serie.priceToCoordinate(s.tp1)
    const yTp2 = serie.priceToCoordinate(s.tp2)
    const yTp3 = serie.priceToCoordinate(s.tp3)
    if (yEntry === null) continue
    const long = s.dir === 'Long'

    // Box TP (entry ↔ TP3) — Pine stBullTPBox, matérialisée au fill.
    if (yTp3 !== null) {
      const yTop = Math.min(yEntry, yTp3)
      const h = Math.abs(yEntry - yTp3)
      ctx.fillStyle = hexVersRgba(COUL_TP, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xDTrade - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_TP, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xDTrade - xG, h)
    }
    // Box SL (entry ↔ sl) — Pine stBullSLBox. Au BE (TP1 touché ou forcé),
    // le Pine SUPPRIME la box (SL → entry, hauteur nulle) : rien à tracer.
    if (!s.be && ySl !== null && Math.abs(yEntry - ySl) >= 1) {
      const yTop = Math.min(yEntry, ySl)
      const h = Math.abs(yEntry - ySl)
      ctx.fillStyle = hexVersRgba(COUL_SL, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xDTrade - xG, h)
      ctx.strokeStyle = hexVersRgba(COUL_SL, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xDTrade - xG, h)
    }
    // Lignes TP1 / TP2 solides (Pine C_TP1_L / C_TP2_L, ancrées au fill).
    ctx.lineWidth = 1
    for (const [y, coul] of [[yTp1, COUL_TP1_L], [yTp2, COUL_TP2_L]] as const) {
      if (y === null) continue
      ctx.strokeStyle = coul
      ctx.setLineDash([])
      ctx.beginPath(); ctx.moveTo(xG, y); ctx.lineTo(xDTrade, y); ctx.stroke()
      // Repère textuel au bord droit
      ctx.font = 'bold 9px sans-serif'
      ctx.fillStyle = coul
      ctx.textAlign = 'left'
      ctx.textBaseline = 'bottom'
      ctx.fillText(y === yTp1 ? 'TP1' : 'TP2', xG + 3, y - 1)
    }
    // Ligne entrée (pointillée).
    ctx.strokeStyle = COUL_ENTRY
    ctx.setLineDash([2, 3])
    ctx.beginPath(); ctx.moveTo(xG, yEntry); ctx.lineTo(xDTrade, yEntry); ctx.stroke()
    ctx.setLineDash([])

    // Label f_lblTrade (Pine : label_up au niveau du SL, texte blanc, SMALL).
    // Supprimé au BE comme dans le Pine — le lot y suit le SL courant.
    if (!s.be && s.label.length && ySl !== null) {
      ctx.font = '10px sans-serif'
      const lh = 12
      const yBase = long ? ySl + 14 : Math.max(0, ySl - 14 - lh * s.label.length)
      // Fond lisible (le label Pine a un fond coloré) — bulle sombre.
      let lMax = 0
      for (const l of s.label) lMax = Math.max(lMax, ctx.measureText(l).width)
      ctx.fillStyle = 'rgba(10,12,18,0.82)'
      ctx.fillRect(xG + 2, yBase - 11, lMax + 10, lh * s.label.length + 8)
      ctx.strokeStyle = long ? hexVersRgba(COUL_BUY, 0.5) : hexVersRgba(COUL_SELL, 0.5)
      ctx.strokeRect(xG + 2, yBase - 11, lMax + 10, lh * s.label.length + 8)
      ctx.textAlign = 'left'
      ctx.textBaseline = 'top'
      s.label.forEach((l, i) => {
        ctx.fillStyle = i === 0 ? (long ? COUL_BUY : COUL_SELL) : '#e5e7eb'
        if (i === 1) ctx.fillStyle = '#fbbf24'
        ctx.fillText(l, xG + 7, yBase + i * lh)
      })
    }
  }
}


export function dessinerTradesExternes(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  liste: (SignalDessin & { tfOrigine: string; enAttente?: boolean })[],
  W: number,
  dernierTs: number | null,
): void {
  const xD = xDroit(ts, W, dernierTs)
  for (const s of liste) {
    // Multi-TF : le timestamp du trade (ex: 09:55 en M5) n'existe pas
    // forcément comme barre sur le TF affiché (M15 = :00/:15/:30/:45).
    // → arrondir au bar la plus proche au lieu de sauter le trade.
    let xGRaw = ts.timeToCoordinate(s.ts as any)
    if (xGRaw === null) {
      const vr = ts.getVisibleRange()
      const lr = ts.getVisibleLogicalRange()
      if (vr && lr && (vr.to as number) > (vr.from as number)) {
        const lg = (lr.from as number) + ((s.ts - (vr.from as number)) / ((vr.to as number) - (vr.from as number))) * ((lr.to as number) - (lr.from as number))
        xGRaw = ts.logicalToCoordinate(Math.round(lg) as never)
      }
    }
    if (xGRaw === null) continue
    const xG = Math.max(0, xGRaw)
    if (xD <= xG) continue
    const tsFin = (s as { tsFin?: number }).tsFin
    let xFin = xD
    if (tsFin !== undefined) {
      const direct = ts.timeToCoordinate(tsFin as any)
      if (direct !== null) xFin = direct
    }
    if (xFin <= xG) continue
    const xDTrade = Math.min(xD, xFin)
    const yEntry = serie.priceToCoordinate(s.entry)
    const ySl = serie.priceToCoordinate(s.sl)
    const yTp = serie.priceToCoordinate(s.tp3)
    if (yEntry === null) continue

    // Ordre EN ATTENTE (jamais rempli) : même géométrie qu'un trade —
    // box SL rouge et box TP verte allégées, bordures pointillées (rien
    // n'existe encore au marché) + badge ⏳.
    if (s.enAttente) {
      if (ySl !== null && Math.abs(yEntry - ySl) >= 1) {
        const yTop = Math.min(yEntry, ySl)
        const h = Math.abs(yEntry - ySl)
        ctx.fillStyle = hexVersRgba(COUL_SL, 0.10)
        ctx.fillRect(xG, yTop, xDTrade - xG, h)
        ctx.strokeStyle = hexVersRgba(COUL_SL, 0.45)
        ctx.lineWidth = 1
        ctx.setLineDash([4, 3])
        ctx.strokeRect(xG, yTop, xDTrade - xG, h)
        ctx.setLineDash([])
      }
      if (yTp !== null) {
        const yTop = Math.min(yEntry, yTp)
        const h = Math.abs(yEntry - yTp)
        ctx.fillStyle = hexVersRgba(COUL_TP, 0.07)
        ctx.fillRect(xG, yTop, xDTrade - xG, h)
        ctx.strokeStyle = hexVersRgba(COUL_TP, 0.35)
        ctx.setLineDash([4, 3])
        ctx.strokeRect(xG, yTop, xDTrade - xG, h)
        ctx.setLineDash([])
      }
      ctx.font = 'bold 9px sans-serif'
      const badge = `⏳ ${s.tfOrigine}`
      const largeur = ctx.measureText(badge).width + 8
      const yBadge = yEntry - 14 < 2 ? yEntry + 4 : yEntry - 14
      ctx.fillStyle = 'rgba(10,12,18,0.8)'
      ctx.fillRect(xG, yBadge, largeur, 12)
      ctx.strokeStyle = 'rgba(148,163,184,0.5)'
      ctx.strokeRect(xG, yBadge, largeur, 12)
      ctx.fillStyle = '#cbd5e1'
      ctx.textAlign = 'left'
      ctx.textBaseline = 'top'
      ctx.fillText(badge, xG + 4, yBadge + 2)
      continue
    }

    // Rendu IDENTIQUE au TF d'origine (décision propriétaire 28/08) :
    // boxes solides, lignes TP, ligne d'entrée — plus d'atténuation.
    const yTp1x = serie.priceToCoordinate(s.tp1)
    if (yTp !== null) {
      const yTop = Math.min(yEntry, yTp)
      ctx.fillStyle = hexVersRgba(COUL_TP, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xDTrade - xG, Math.abs(yEntry - yTp))
      ctx.strokeStyle = hexVersRgba(COUL_TP, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xDTrade - xG, Math.abs(yEntry - yTp))
    }
    if (ySl !== null && Math.abs(yEntry - ySl) >= 1) {
      const yTop = Math.min(yEntry, ySl)
      ctx.fillStyle = hexVersRgba(COUL_SL, (100 - 78) / 100)
      ctx.fillRect(xG, yTop, xDTrade - xG, Math.abs(yEntry - ySl))
      ctx.strokeStyle = hexVersRgba(COUL_SL, 0.6)
      ctx.lineWidth = 1
      ctx.strokeRect(xG, yTop, xDTrade - xG, Math.abs(yEntry - ySl))
    }
    // Lignes TP1 / TP2 solides avec labels (comme le natif).
    ctx.lineWidth = 1
    const yTp2x = serie.priceToCoordinate(s.tp2)
    for (const [y, coul, lbl] of [[yTp1x, COUL_TP1_L, 'TP1'], [yTp2x, '#4ade80', 'TP2']] as const) {
      if (y === null) continue
      ctx.strokeStyle = coul
      ctx.setLineDash([])
      ctx.beginPath(); ctx.moveTo(xG, y); ctx.lineTo(xDTrade, y); ctx.stroke()
      ctx.font = 'bold 9px sans-serif'
      ctx.fillStyle = coul
      ctx.textAlign = 'left'
      ctx.textBaseline = 'bottom'
      ctx.fillText(lbl, xG + 3, y - 1)
    }
    // Ligne d'entrée (pointillée, comme le natif).
    ctx.strokeStyle = COUL_ENTRY
    ctx.setLineDash([2, 3])
    ctx.beginPath(); ctx.moveTo(xG, yEntry); ctx.lineTo(xDTrade, yEntry); ctx.stroke()
    ctx.setLineDash([])

    // Badge du TF d'origine — petit carré discret à gauche de l'entrée.
    const badge = s.tfOrigine
    ctx.font = 'bold 9px sans-serif'
    const largeur = ctx.measureText(badge).width + 8
    const yBadge = yEntry - 14 < 2 ? yEntry + 4 : yEntry - 14
    ctx.fillStyle = 'rgba(10,12,18,0.8)'
    ctx.fillRect(xG, yBadge, largeur, 12)
    ctx.strokeStyle = s.dir === 'Long' ? hexVersRgba('#26a69a', 0.6) : hexVersRgba('#ef5350', 0.6)
    ctx.lineWidth = 1
    ctx.strokeRect(xG, yBadge, largeur, 12)
    ctx.fillStyle = s.dir === 'Long' ? '#8fe3d0' : '#f3a8a0'
    ctx.textAlign = 'left'
    ctx.textBaseline = 'top'
    ctx.fillText(badge, xG + 4, yBadge + 2)
  }
}
