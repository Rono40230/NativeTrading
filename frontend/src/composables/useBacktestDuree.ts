import { ref, computed } from 'vue'

// M5 fixe : 12 bougies/heure
const BOUGIES_PAR_HEURE_M5 = 12

const DUREES = [
  { label: '1 mois',   jours: 30 },
  { label: '3 mois',   jours: 90 },
  { label: '6 mois',   jours: 180 },
  { label: '12 mois',  jours: 365 },
  { label: '24 mois',  jours: 730 },
]

export function useBacktestDuree(_timeframe?: ReturnType<typeof ref<string>>) {
  const dureeLabel = ref('3 mois')

  const dureesDisponibles = computed(() => DUREES)

  const limiteBougies = computed(() => {
    const duree = DUREES.find(d => d.label === dureeLabel.value) ?? DUREES[1]
    return duree.jours
  })

  return { dureeLabel, dureesDisponibles, limiteBougies }
}
