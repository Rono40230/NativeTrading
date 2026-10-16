/**
 * Garde fraîcheur pipeline (16/10) — badge ⚠ par asset de la page 📦.
 *
 * Le backend audite chaque minute que chaque asset ACTIF du pipeline a
 * ses barres dans leur contrat de fraîcheur (avertir, jamais réparer) ;
 * ce composable lit l'état publié (mémoire, zéro requête DB côté endpoint)
 * et l'expose pour badge/infobulle, poll à la charge de la vue.
 */
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { FraicheurAnomalie, FraicheurEtatAsset } from '@/services/api.service'

export function useFraicheurPipeline() {
  const fraicheurEtats = ref<FraicheurEtatAsset[]>([])

  async function charger() {
    try {
      const res = await apiService.obtenirFraicheurPipeline()
      fraicheurEtats.value = res.etats
    } catch {
      // On garde l'état précédent : une erreur passagère ne doit pas
      // effacer une alerte de fraîcheur — le cycle suivant reprendra.
    }
  }

  function anomalies(id: string): FraicheurAnomalie[] {
    return fraicheurEtats.value.find(e => e.asset === id)?.anomalies ?? []
  }

  function libelle(a: FraicheurAnomalie): string {
    return a.retard_min < 0
      ? `${a.tf} : aucune barre`
      : `${a.tf} : +${a.retard_min >= 120 ? Math.round(a.retard_min / 60) + ' h' : a.retard_min + ' min'} (seuil ${a.seuil_min})`
  }

  const enAlerte = computed(() => fraicheurEtats.value.filter(e => e.anomalies.length > 0))

  return { fraicheurEtats, charger, anomalies, libelle, enAlerte }
}
