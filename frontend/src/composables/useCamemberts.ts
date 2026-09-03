/**
 * Fonctions pures des camemberts des cartes stratégies du dashboard —
 * répartitions (nombre de trades), classements (pips) et rendu des légendes.
 * Extraites de DashboardStrategiesBlocs.vue (limite 600 lignes, pre-commit).
 */
/** Trade fermé rempli tel que calculé par les cartes (voir DashboardStrategiesBlocs). */
export interface TradeJour {
  id: string; asset: string; tf: string; palier: string; pips: number
}

export interface PartCamembert {
  label: string
  n: number
  /** Part en % (sur 100 = circonférence du donut). */
  part: number
}

export interface PartClassement {
  label: string
  /** Pips signés de la catégorie (les négatifs n'ont pas de tranche). */
  pips: number
  /** Part en % des pips POSITIFS totaux. */
  part: number
}

export const PALETTE = ['#60a5fa', '#34d399', '#fbbf24', '#f87171', '#a78bfa', '#f472b6', '#38bdf8', '#fb923c', '#4ade80', '#e879f9']

export function couleurTf(tf: string): string {
  const ordre = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
  return PALETTE[(ordre.indexOf(tf) + PALETTE.length) % PALETTE.length]
}

export function couleurAsset(asset: string): string {
  let h = 0
  for (const c of asset) h = (h * 31 + c.charCodeAt(0)) % 997
  return PALETTE[h % PALETTE.length]
}

/// Répartition : nombre de trades par catégorie, parts égales à la circonférence.
export function repartition(
  trades: (TradeJour & { fermeLe: number })[],
  cle: (t: TradeJour & { fermeLe: number }) => string,
): PartCamembert[] {
  const par = new Map<string, number>()
  for (const t of trades) par.set(cle(t), (par.get(cle(t)) ?? 0) + 1)
  const total = trades.length || 1
  return [...par.entries()]
    .map(([label, n]) => ({ label, n, part: (n / total) * 100 }))
    .sort((a, b) => b.n - a.n)
}

/// Classement : Σ pips signés par catégorie — les positifs portent des
/// tranches proportionnelles à leur part des gains totaux.
export function classement(
  trades: (TradeJour & { fermeLe: number })[],
  cle: (t: TradeJour & { fermeLe: number }) => string,
): PartClassement[] {
  const par = new Map<string, number>()
  for (const t of trades) par.set(cle(t), (par.get(cle(t)) ?? 0) + t.pips)
  const totalGains = [...par.values()].filter(v => v > 0).reduce((a, b) => a + b, 0)
  return [...par.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([label, pips]) => ({
      label,
      pips,
      part: totalGains > 0 && pips > 0 ? (pips / totalGains) * 100 : 0,
    }))
}

/// Décalage cumulé des segments du donut (stroke-dashoffset).
export function decallage(parts: { part: number }[], index: number): number {
  return parts.slice(0, index).reduce((somme, p) => somme + p.part, 0)
}

/// Total des trades d'une répartition (centre du donut).
export function totalParts(parts: PartCamembert[]): number {
  return parts.reduce((n, p) => n + p.n, 0)
}

/// Lignes de légende d'un classement pips. Deux garanties :
/// 1. au-delà de 4 catégories, une ligne « autres » absorbe les cachées —
///    les lignes somment donc TOUJOURS au total (centre du donut) ;
/// 2. arrondi par plus grand reste — jamais d'écart ±1 par arrondi.
export function lignesClassement(parts: PartClassement[], n = 4): { label: string; pips: number; autres: boolean }[] {
  const affiches = parts.slice(0, n)
  const caches = parts.slice(n).reduce((s, p) => s + p.pips, 0)
  const entrees: { label: string; exact: number; autres: boolean }[] = [
    ...affiches.map(p => ({ label: p.label, exact: p.pips, autres: false })),
  ]
  if (parts.length > n) entrees.push({ label: 'autres', exact: caches, autres: true })
  const arrondis = entrees.map(e => Math.round(e.exact))
  const cible = Math.round(entrees.reduce((s, e) => s + e.exact, 0))
  let ecart = cible - arrondis.reduce((a, b) => a + b, 0)
  if (ecart !== 0) {
    const parReste = entrees
      .map((e, i) => ({ i, reste: Math.abs(e.exact - arrondis[i]) }))
      .sort((a, b) => b.reste - a.reste)
    for (const { i } of parReste) {
      if (ecart === 0) break
      arrondis[i] += ecart > 0 ? 1 : -1
      ecart += ecart > 0 ? -1 : 1
    }
  }
  return entrees
    .map((e, i) => ({ label: e.label, pips: arrondis[i], autres: e.autres }))
    .filter(l => !l.autres || l.pips !== 0)
}
