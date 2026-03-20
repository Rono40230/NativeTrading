import type { IChartApi, ISeriesApi } from 'lightweight-charts'

type TimeScale = ReturnType<IChartApi['timeScale']>

export interface LigneLiq {
  prix: number
  timestamp: number  // Unix secondes — bord gauche (formation)
  couleur: string
  label: string  // 'EQH' | 'EQL' | ''
}

export interface RectAsie {
  timestampDebut: number
  timestampFin: number  // bord droit fixe (fin de session)
  haut: number
  bas: number
  couleurFond: string
  couleurBord: string
}

export interface LigneDeviation {
  prix: number
  timestampDebut: number  // même origine que la session
  couleur: string
  label: string  // ex: '+1', '+2', '-1', '-2'
}

export function dessinerLignesLiq(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  lignes: LigneLiq[],
  W: number,
  dernierTs: number | null,
): void {
  for (const ligne of lignes) {
    const y = serie.priceToCoordinate(ligne.prix)
    if (y === null) continue

    const xGaucheRaw = ts.timeToCoordinate(ligne.timestamp as any)
    const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
    const xDroitRaw = dernierTs !== null ? ts.timeToCoordinate(dernierTs as any) : null
    const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70

    ctx.strokeStyle = ligne.couleur
    if (xDroit > xGauche) {
      ctx.lineWidth = 1.5
      ctx.beginPath()
      ctx.moveTo(xGauche, y)
      ctx.lineTo(xDroit, y)
      ctx.stroke()
      ctx.lineWidth = 2
      ctx.beginPath()
      ctx.moveTo(xGauche, y - 4)
      ctx.lineTo(xGauche, y + 4)
      ctx.stroke()
    }

    if (ligne.label) {
      const xLabel = Math.max(Math.min(xGauche, W - 60), 4)
      ctx.font = 'bold 10px sans-serif'
      ctx.fillStyle = ligne.couleur
      ctx.textAlign = 'left'
      ctx.textBaseline = 'bottom'
      ctx.fillText(ligne.label, xLabel + 3, y - 2)
    }
  }
}

export function dessinerRangesAsie(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  rects: RectAsie[],
  W: number,
): void {
  for (const rect of rects) {
    const yHaut = serie.priceToCoordinate(rect.haut)
    const yBas  = serie.priceToCoordinate(rect.bas)
    if (yHaut === null || yBas === null) continue
    const yTop    = Math.min(yHaut, yBas)
    const yBottom = Math.max(yHaut, yBas)
    const hauteur = yBottom - yTop
    if (hauteur < 1) continue

    const xGaucheRaw = ts.timeToCoordinate(rect.timestampDebut as any)
    const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
    const xDroitRaw = ts.timeToCoordinate(rect.timestampFin as any)
    const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70
    if (xDroit <= xGauche) continue

    ctx.fillStyle = rect.couleurFond
    ctx.fillRect(xGauche, yTop, xDroit - xGauche, hauteur)
    ctx.strokeStyle = rect.couleurBord
    ctx.lineWidth = 1
    ctx.beginPath(); ctx.moveTo(xGauche, yTop);    ctx.lineTo(xDroit, yTop);    ctx.stroke()
    ctx.beginPath(); ctx.moveTo(xGauche, yBottom); ctx.lineTo(xDroit, yBottom); ctx.stroke()
    ctx.lineWidth = 2
    ctx.beginPath(); ctx.moveTo(xGauche, yTop); ctx.lineTo(xGauche, yBottom); ctx.stroke()
  }
}

export function dessinerDeviationsAsie(
  ctx: CanvasRenderingContext2D,
  serie: ISeriesApi<'Candlestick'>,
  ts: TimeScale,
  devs: LigneDeviation[],
  W: number,
  dernierTs: number | null,
): void {
  for (const dev of devs) {
    const y = serie.priceToCoordinate(dev.prix)
    if (y === null) continue

    const xGaucheRaw = ts.timeToCoordinate(dev.timestampDebut as any)
    const xGauche = xGaucheRaw !== null ? Math.max(0, xGaucheRaw) : 0
    const xDroitRaw = dernierTs !== null ? ts.timeToCoordinate(dernierTs as any) : null
    const xDroit = xDroitRaw !== null ? Math.min(xDroitRaw, W - 4) : W - 70

    ctx.strokeStyle = dev.couleur
    ctx.lineWidth = 1
    ctx.setLineDash([4, 4])
    if (xDroit > xGauche) {
      ctx.beginPath(); ctx.moveTo(xGauche, y); ctx.lineTo(xDroit, y); ctx.stroke()
    }
    ctx.setLineDash([])

    if (dev.label) {
      const xLabel = Math.max(Math.min(xGauche, W - 60), 4)
      ctx.font = '9px sans-serif'
      ctx.fillStyle = dev.couleur
      ctx.textAlign = 'left'
      ctx.textBaseline = 'bottom'
      ctx.fillText(dev.label, xLabel + 3, y - 2)
    }
  }
}
