import { computed } from 'vue'
import type { Ref } from 'vue'
import type { ReponsePatternsVolatilite } from '@/services/api.service'

const HEURES = Array.from({ length: 24 }, (_, i) => i)
const JOURS = [
  { index: 0, label: 'Dim' },
  { index: 1, label: 'Lun' },
  { index: 2, label: 'Mar' },
  { index: 3, label: 'Mer' },
  { index: 4, label: 'Jeu' },
  { index: 5, label: 'Ven' },
  { index: 6, label: 'Sam' },
]

export function useHoraireAnalyse(
  reponse: Ref<ReponsePatternsVolatilite | null>,
  decalageParis: 1 | 2,
) {
  const analyse = computed(() => {
    const patterns = reponse.value?.patterns
    if (!patterns?.length) return null

    const parHeure = HEURES.map((h) => {
      const pts = patterns.filter((p) => p.heure === h && p.nb_points > 0)
      if (!pts.length) return null
      const atrMoyen = pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length
      const clusterMoyen = Math.round(pts.reduce((s, p) => s + p.cluster, 0) / pts.length)
      return { heureUtc: h, heureParis: (h + decalageParis) % 24, cluster: clusterMoyen, atrMoyen }
    }).filter(Boolean) as { heureUtc: number; heureParis: number; cluster: number; atrMoyen: number }[]

    const top3 = [...parHeure].sort((a, b) => b.cluster - a.cluster || b.atrMoyen - a.atrMoyen).slice(0, 3)
    const pires3 = [...parHeure].sort((a, b) => a.cluster - b.cluster || a.atrMoyen - b.atrMoyen).slice(0, 3)

    const parJour = JOURS.map((j) => {
      const pts = patterns.filter((p) => p.jour_semaine === j.index && p.nb_points > 0)
      if (!pts.length) return null
      return { ...j, atrMoyen: pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length }
    }).filter(Boolean) as { index: number; label: string; atrMoyen: number }[]

    const meilleurJour = parJour.reduce((a, b) => (a.atrMoyen > b.atrMoyen ? a : b))
    const pireJour = parJour.reduce((a, b) => (a.atrMoyen < b.atrMoyen ? a : b))

    const hParisActuelle = Number(
      new Intl.DateTimeFormat('en-US', {
        timeZone: 'Europe/Paris',
        hour: 'numeric',
        hour12: false,
      }).format(new Date()),
    )
    const heureUtcActuelle = (hParisActuelle - decalageParis + 24) % 24
    const jourActuel = new Date().getDay()
    const patternActuel =
      patterns.find((p) => p.heure === heureUtcActuelle && p.jour_semaine === jourActuel) ?? null

    return { top3, pires3, meilleurJour, pireJour, patternActuel, hParisActuelle }
  })

  return { analyse }
}
