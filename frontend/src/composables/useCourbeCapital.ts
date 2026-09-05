// ── Courbe du capital simulé — géométrie partagée ───────────────────────────
// Le viewBox (SVG étiré « none ») et l'échelle Y servent au tracé bicolore
// (CourbeCapital.vue), aux zones de survol et au tooltip du parent
// (DashboardStrategiesBlocs) : une seule source de vérité pour que tout
// reste ancré sur la courbe.

export const LARGEUR = 100
export const HAUTEUR = 32

/** Clôture composée (sous-ensemble structurel de l'API capital). */
export interface PointCapital {
  ferme_le: number
  capital_apres: number
  profit: number
}

/** Capital simulé (sous-ensemble structurel de l'API capital). */
export interface CapitalCourbe {
  capital_depart: number
  points: PointCapital[]
}

/** Zone de survol d'une clôture, positionnée en % du conteneur. */
export interface ZoneCapital {
  gauche: string
  haut: string
  point: PointCapital
}

/// Points SVG de la courbe capital : départ + une valeur par clôture,
/// échelle $ propre (min→max de la série).
export function pointsCapital(c: CapitalCourbe): string {
  const serie = [c.capital_depart, ...c.points.map(p => p.capital_apres)]
  const min = Math.min(...serie)
  const max = Math.max(...serie)
  const amplitude = max - min || 1
  const n = serie.length
  return serie
    .map((v, j) => {
      const x = n > 1 ? (j / (n - 1)) * LARGEUR : 0
      const y = HAUTEUR - 2 - ((v - min) / amplitude) * (HAUTEUR - 4)
      return `${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(' ')
}

/// Y du capital de départ dans le viewBox — la ligne pointillée de référence
/// et les rectangles de découpe vert/rouge s'y calent (le départ est le
/// point 0 de la série : la ligne est toujours dans le cadre).
export function yDepartCapital(c: CapitalCourbe): number {
  if (c.points.length === 0) return HAUTEUR - 2
  const serie = [c.capital_depart, ...c.points.map(p => p.capital_apres)]
  const min = Math.min(...serie)
  const max = Math.max(...serie)
  const amplitude = max - min || 1
  return HAUTEUR - 2 - ((c.capital_depart - min) / amplitude) * (HAUTEUR - 4)
}

/// Zones de survol : une par clôture, positionnées en % du conteneur
/// (même géométrie que pointsCapital — SVG étiré « none »).
export function zonesCapital(c: CapitalCourbe | null): ZoneCapital[] {
  if (!c || c.points.length === 0) return []
  const serie = [c.capital_depart, ...c.points.map(p => p.capital_apres)]
  const min = Math.min(...serie)
  const max = Math.max(...serie)
  const amplitude = max - min || 1
  const n = serie.length
  return c.points.map((p, i) => {
    const x = n > 1 ? ((i + 1) / (n - 1)) * LARGEUR : 0
    const y = HAUTEUR - 2 - ((p.capital_apres - min) / amplitude) * (HAUTEUR - 4)
    return { gauche: `${(x / LARGEUR) * 100}%`, haut: `${(y / HAUTEUR) * 100}%`, point: p }
  })
}
