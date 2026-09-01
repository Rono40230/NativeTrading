/**
 * useHistoriqueStrategie — historique filtré d'une verticale pour les pages
 * stratégies (gabarit étape 01/09). Réutilise les règles de la page
 * Historique : variantes de nommage SMC (writer v1 vs runtime), totaux
 * Σ palier / Σ réalisé / jamais remplis, MFE des perdants.
 */
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { palierMax } from '@/composables/useSignalFormat'

export type CleStrategie = 'smc' | 'straddle'

const SMC_VARIANTES = ['smc', 'smcdirectional', 'smc directionnel', 'smc+ia']

export function useHistoriqueStrategie(cle: CleStrategie) {
  const signaux = ref<Signal[]>([])
  const chargement = ref(false)
  const mfeParId = ref<Record<string, { mfe_r: number | null; meilleur_prix: number | null }>>({})

  const signauxFiltres = computed(() =>
    signaux.value.filter(s => {
      const nom = s.strategie.toLowerCase().trim()
      return cle === 'smc'
        ? SMC_VARIANTES.includes(nom)
        : nom === 'straddle'
    }).filter(s => s.statut === 'Fermé' && s.verdict !== null),
  )

  const totaux = computed(() => {
    let ref: number | null = null
    let realise: number | null = null
    let jamaisRemplis = 0
    let enCours = 0
    for (const s of signaux.value) {
      const nom = s.strategie.toLowerCase().trim()
      const ok = cle === 'smc' ? SMC_VARIANTES.includes(nom) : nom === 'straddle'
      if (!ok) continue
      if (s.statut !== 'Fermé') { enCours++; continue }
      if (s.heure_entree === null || s.heure_entree === undefined) { jamaisRemplis++; continue }
      const r = palierMax(s).rReference
      if (r !== null) ref = (ref ?? 0) + r
      if (s.r_realise !== null && s.r_realise !== undefined) realise = (realise ?? 0) + s.r_realise
    }
    return { ref, realise, jamaisRemplis, enCours }
  })

  async function charger() {
    chargement.value = true
    try {
      signaux.value = await apiService.getSignaux(500)
      const idsSl = signauxFiltres.value
        .filter(s => (s.verdict ?? '').toLowerCase().includes('sl'))
        .map(s => s.id)
      mfeParId.value = await apiService.getMfeSignaux(idsSl)
    } catch {
      signaux.value = []
    } finally {
      chargement.value = false
    }
  }

  return { signauxFiltres, totaux, mfeParId, chargement, charger }
}
