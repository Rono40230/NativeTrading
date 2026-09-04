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
  /** Lot recalculé par trade (capital composé d'époque) — colonne Lot. */
  const lotParId = ref<Record<string, number>>({})

  // ── Tri par colonne (HistoryTable émet « trier-par ») ────────────────────
  const triColonne = ref('')
  const triDir = ref<'asc' | 'desc'>('desc')

  function trierPar(col: string) {
    if (triColonne.value === col) {
      triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
    } else {
      triColonne.value = col
      triDir.value = 'desc'
    }
  }

  /// Valeur de tri d'un signal pour la colonne : nombres, chaînes ou null
  /// (les nulls restent en queue quel que soit le sens).
  function valeurTri(s: Signal, col: string): number | string | null {
    if (col === 'tp1' || col === 'tp2' || col === 'tp3') {
      return s.take_profit[col === 'tp1' ? 0 : col === 'tp2' ? 1 : 2] ?? null
    }
    if (col === 'r_reference') return palierMax(s).rReference
    const v = (s as unknown as Record<string, unknown>)[col]
    return typeof v === 'number' || typeof v === 'string' ? v : null
  }

  const signauxFiltres = computed(() =>
    signaux.value.filter(s => {
      const nom = s.strategie.toLowerCase().trim()
      return cle === 'smc'
        ? SMC_VARIANTES.includes(nom)
        : nom === 'straddle'
    }).filter(s => s.statut === 'Fermé' && s.verdict !== null),
  )

  /// Liste filtrée PUIS triée par la colonne active (défaut : plus récents).
  const signauxTriés = computed(() => {
    const liste = [...signauxFiltres.value]
    if (!triColonne.value) {
      return liste.sort((a, b) => b.cree_le - a.cree_le)
    }
    const col = triColonne.value
    const sens = triDir.value === 'asc' ? 1 : -1
    return liste.sort((a, b) => {
      const va = valeurTri(a, col)
      const vb = valeurTri(b, col)
      if (va === null && vb === null) return b.cree_le - a.cree_le
      if (va === null) return 1
      if (vb === null) return -1
      if (typeof va === 'number' && typeof vb === 'number') {
        return va === vb ? b.cree_le - a.cree_le : (va - vb) * sens
      }
      const c = String(va).localeCompare(String(vb))
      return c === 0 ? b.cree_le - a.cree_le : c * sens
    })
  })

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
      const idsTous = signauxFiltres.value.map(s => s.id)
      const [mfe, lots] = await Promise.all([
        apiService.getMfeSignaux(idsSl),
        apiService.getLotsSignaux(idsTous),
      ])
      mfeParId.value = mfe
      lotParId.value = lots
    } catch {
      signaux.value = []
    } finally {
      chargement.value = false
    }
  }

  return { signauxFiltres, signauxTriés, totaux, mfeParId, lotParId, triColonne, triDir, trierPar, chargement, charger }
}
