import type { ISeriesApi, IPriceLine } from 'lightweight-charts'
import type { SignalIndicateur } from './chartSignauxTypes'

// ─── Types ────────────────────────────────────────────────────────────────────

export interface NiveauSlTp {
  entry: number
  sl: number
  tp1: number
  tp2: number
  direction: 'bullish' | 'bearish'
}

export interface LignesSlTp {
  sl: IPriceLine | null
  tp1: IPriceLine | null
  tp2: IPriceLine | null
}

const MULTI_STANDARD = 2.0
const MULTI_LARGE = 3.0

/** Calcule les niveaux SL / TP1 / TP2 à partir du signal et de la valeur ATR */
export function calculerSlTp(
  signal: SignalIndicateur,
  atrValeur: number,
): NiveauSlTp | null {
  if (!atrValeur || atrValeur <= 0 || signal.direction === 'neutre') return null
  const isBull = signal.direction === 'bullish'
  const entry = signal.prix_entree
  return {
    entry,
    sl:  isBull ? entry - atrValeur * MULTI_STANDARD : entry + atrValeur * MULTI_STANDARD,
    tp1: isBull ? entry + atrValeur * MULTI_STANDARD : entry - atrValeur * MULTI_STANDARD,
    tp2: isBull ? entry + atrValeur * MULTI_LARGE    : entry - atrValeur * MULTI_LARGE,
    direction: isBull ? 'bullish' : 'bearish',
  }
}

// ─── Rendu lignes de prix ─────────────────────────────────────────────────────

export function afficherSlTp(
  serie: ISeriesApi<'Candlestick'>,
  niveau: NiveauSlTp,
): LignesSlTp {
  return {
    sl:  serie.createPriceLine({ price: niveau.sl,  color: '#ef4444', lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: 'SL' }),
    tp1: serie.createPriceLine({ price: niveau.tp1, color: '#10b981', lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: 'TP1' }),
    tp2: serie.createPriceLine({ price: niveau.tp2, color: '#34d399', lineWidth: 1, lineStyle: 3, axisLabelVisible: true, title: 'TP2' }),
  }
}

export function effacerSlTp(
  serie: ISeriesApi<'Candlestick'> | null,
  lignes: LignesSlTp,
): void {
  if (!serie) return
  try {
    if (lignes.sl)  { serie.removePriceLine(lignes.sl);  lignes.sl  = null }
    if (lignes.tp1) { serie.removePriceLine(lignes.tp1); lignes.tp1 = null }
    if (lignes.tp2) { serie.removePriceLine(lignes.tp2); lignes.tp2 = null }
  } catch { /* série déjà détruite */ }
}
