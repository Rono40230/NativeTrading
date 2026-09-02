/**
 * Analyse des patterns de volatilité horaire — logique partagée entre la page
 * /heatmap (calendrier historique) et le bloc ⏰ Créneaux de volatilité du
 * dashboard : meilleures/pires fenêtres, jours de la semaine, créneau courant.
 * Extraite à l'identique de HoraireHeatmap.vue — une seule source de vérité.
 */
import type { PatternHoraire } from '@/services/api.types.marche'
import { offsetParisHeures } from '@/utils/date'
import { JOURS } from '@/components/common/heatmapConstants'

export const NOM_CLUSTER = ['Calme', 'Modéré', 'Élevé', 'Extrême'] as const
export const COULEUR_CLUSTER_TEXTE = ['text-emerald-400', 'text-amber-400', 'text-orange-400', 'text-red-400'] as const

export interface FenetreVolatilite { heureDebut: number; heureFin: number; cluster: number }
export interface JourVolatilite { index: number; label: string; atrMoyen: number }

export interface AnalyseVolatilite {
  top3: FenetreVolatilite[]
  pires3: FenetreVolatilite[]
  meilleurJour: JourVolatilite
  pireJour: JourVolatilite
  patternActuel: PatternHoraire | null
  hParisActuelle: number
}

/** Heure Paris correspondant à une heure UTC (offset DST auto Europe/Paris). */
export function convertirHeureParis(heureUtc: number): number {
  return (heureUtc + offsetParisHeures()) % 24
}

/** Construit l'analyse complète à partir des patterns horaires d'un asset. */
export function calculerAnalyse(patterns: PatternHoraire[]): AnalyseVolatilite | null {
  if (!patterns.length) return null
  const heures = Array.from({ length: 24 }, (_, i) => i)

  const parHeure = heures.map(h => {
    const pts = patterns.filter(p => p.heure === h && p.nb_points > 0)
    if (!pts.length) return null
    const atrMoyen = pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length
    const clusterMoyen = Math.round(pts.reduce((s, p) => s + p.cluster, 0) / pts.length)
    return { heureUtc: h, heureParis: convertirHeureParis(h), cluster: clusterMoyen, atrMoyen }
  }).filter(Boolean) as { heureUtc: number; heureParis: number; cluster: number; atrMoyen: number }[]

  type Slot = { heureDebut: number; heureFin: number; cluster: number }
  const fusionner = (h: typeof parHeure): Slot[] => [...h].sort((a, b) => a.heureParis - b.heureParis).reduce<Slot[]>((r, x) => { const l = r.at(-1); l && x.heureParis === l.heureFin ? (l.heureFin++, l.cluster = Math.max(l.cluster, x.cluster)) : r.push({ heureDebut: x.heureParis, heureFin: x.heureParis + 1, cluster: x.cluster }); return r }, []).slice(0, 3)
  const top3 = fusionner([...parHeure].sort((a, b) => b.cluster - a.cluster || b.atrMoyen - a.atrMoyen).slice(0, 6))
  const pires3 = fusionner([...parHeure].sort((a, b) => a.cluster - b.cluster || a.atrMoyen - b.atrMoyen).slice(0, 6))

  const parJour = JOURS.map(j => {
    const pts = patterns.filter(p => p.jour_semaine === j.index && p.nb_points > 0)
    // Jours partiels exclus : un jour qui ne couvre que quelques heures (ex.
    // réouverture dimanche 22h UTC des métaux — gap violent sur 1-2 h) ne se
    // compare pas à des journées complètes : son ATR moyen serait gonflé.
    if (pts.length < 8) return null
    return { ...j, atrMoyen: pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length }
  }).filter(Boolean) as JourVolatilite[]

  const meilleurJour = parJour.reduce((a, b) => a.atrMoyen > b.atrMoyen ? a : b)
  const pireJour = parJour.reduce((a, b) => a.atrMoyen < b.atrMoyen ? a : b)

  // Recherche directe dans la convention des données (heure UTC, jour 0=Dim) :
  // on lit l'heure/jour UTC courants pour matcher le bucket exact, et l'heure
  // Paris uniquement pour le label.
  const maintenant = new Date()
  const hParisActuelle = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant))
  const heureUtcActuelle = maintenant.getUTCHours()
  const jourActuel = maintenant.getUTCDay()
  const patternActuel = patterns.find(p => p.heure === heureUtcActuelle && p.jour_semaine === jourActuel) ?? null

  return { top3, pires3, meilleurJour, pireJour, patternActuel, hParisActuelle }
}
