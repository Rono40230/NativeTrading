/**
 * useHistoriqueStrategie — historique filtré d'une verticale pour les pages
 * stratégies. Deux voix distinctes (décision 23/09) : le R DISTANCE
 * (niveau le plus lointain atteint — juge entrée + TP) et le $ réel du
 * trade (profit du re-jeu capital, rapproché par id). Variantes de nommage
 * SMC, jamais remplis, MFE des perdants.
 */
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import { http } from '@/services/http.client'
import { chargerAnalyse } from '@/composables/useAnalyses'
import type { Signal } from '@/services/api.service'

export type CleStrategie = 'smc' | 'straddle' | 'rockets' | 'kdj_halftrend'

const SMC_VARIANTES = ['smc', 'smcdirectional', 'smc directionnel', 'smc+ia']

export function useHistoriqueStrategie(cle: CleStrategie) {
  const signaux = ref<Signal[]>([])
  const chargement = ref(false)
  const mfeParId = ref<Record<string, { mfe_r: number | null; meilleur_prix: number | null }>>({})
  /** Lot recalculé par trade (capital composé d'époque) — colonne Lot. */
  const lotParId = ref<Record<string, number>>({})
  /** Nombre de notes du journal par trade — badge 📝. */
  const journalComptes = ref<Record<string, number>>({})
  /** $ réel encaissé par trade (re-jeu capital — rapprochement par id). */
  const profitParId = ref<Record<string, number>>({})

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
  /// Date d'« Ouvert le » : le REMPLISSAGE (ouverture réelle de la position),
  /// l'émission en repli — tri initial, égalités et colonne suivent l'affichage.
  const ouvertLe = (s: Signal) => s.heure_entree ?? s.cree_le

  /// (les nulls restent en queue quel que soit le sens).
  function valeurTri(s: Signal, col: string): number | string | null {
    if (col === 'tp1' || col === 'tp2' || col === 'tp3') {
      return s.take_profit[col === 'tp1' ? 0 : col === 'tp2' ? 1 : 2] ?? null
    }
    if (col === 'r_reference') return s.r_distance ?? null
    // « Ouvert le » trie sur la valeur affichée : le remplissage (l'ouverture
    // réelle de la position), pas l'émission de l'ordre.
    if (col === 'cree_le') return ouvertLe(s)
    // « Durée » : vie de la position, du remplissage à la fermeture.
    if (col === 'duree') return s.heure_entree != null && s.ferme_le ? s.ferme_le - s.heure_entree : null
    const v = (s as unknown as Record<string, unknown>)[col]
    return typeof v === 'number' || typeof v === 'string' ? v : null
  }

  const signauxFiltres = computed(() =>
    signaux.value.filter(s => {
      const nom = s.strategie.toLowerCase().trim()
      return cle === 'smc'
        ? SMC_VARIANTES.includes(nom)
        : nom === cle
    }).filter(s => s.statut === 'Fermé' && s.verdict !== null
      && s.heure_entree !== null && s.heure_entree !== undefined),
  )

  /// Liste filtrée PUIS triée par la colonne active (défaut : plus récents).
  const signauxTriés = computed(() => {
    const liste = [...signauxFiltres.value]
    if (!triColonne.value) {
      return liste.sort((a, b) => ouvertLe(b) - ouvertLe(a))
    }
    const col = triColonne.value
    const sens = triDir.value === 'asc' ? 1 : -1
    return liste.sort((a, b) => {
      const va = valeurTri(a, col)
      const vb = valeurTri(b, col)
      if (va === null && vb === null) return ouvertLe(b) - ouvertLe(a)
      if (va === null) return 1
      if (vb === null) return -1
      if (typeof va === 'number' && typeof vb === 'number') {
        return va === vb ? ouvertLe(b) - ouvertLe(a) : (va - vb) * sens
      }
      const c = String(va).localeCompare(String(vb))
      return c === 0 ? ouvertLe(b) - ouvertLe(a) : c * sens
    })
  })

  /// Σ R ENCAISSÉ (décision 16/09) : la colonne R du tableau, servie par le
  /// backend — strictement la même valeur que le badge R du dashboard et le
  /// rapport (le front ne recalcule plus rien).
  const totaux = computed(() => {
    let sommeR: number | null = null
    let jamaisRemplis = 0
    let enCours = 0
    for (const s of signaux.value) {
      const nom = s.strategie.toLowerCase().trim()
      const ok = cle === 'smc' ? SMC_VARIANTES.includes(nom) : nom === cle
      if (!ok) continue
      if (s.statut !== 'Fermé') { enCours++; continue }
      if (s.heure_entree === null || s.heure_entree === undefined) { jamaisRemplis++; continue }
      // R DISTANCE — LE R affiché (23/09) : juge la stratégie, indépendant
      // des réglages de sortie. Le $ vit dans profitParId.
      const r = s.r_distance
      if (r !== null && r !== undefined) sommeR = (sommeR ?? 0) + r
    }
    return { sommeR, jamaisRemplis, enCours }
  })

  async function charger() {
    chargement.value = true
    try {
      signaux.value = await apiService.getSignaux(500)
      const idsSl = signauxFiltres.value
        .filter(s => (s.verdict ?? '').toLowerCase().includes('sl'))
        .map(s => s.id)
      const idsTous = signauxFiltres.value.map(s => s.id)
      const [mfe, lots, comptes] = await Promise.all([
        apiService.getMfeSignaux(idsSl),
        apiService.getLotsSignaux(idsTous),
        http.get<Record<string, number>>('/api/journal/comptes').then(r => r.data).catch(() => ({})),
      ])
      mfeParId.value = mfe
      lotParId.value = lots
      journalComptes.value = comptes
      // $ par trade (voix « résultat ») — les clôtures exposées du re-jeu.
      try {
        const analyse = await chargerAnalyse(cle === 'smc' ? 'SMC' : cle)
        const carte: Record<string, number> = {}
        for (const c of analyse?.clotures ?? []) carte[c.id] = c.dollars
        profitParId.value = carte
      } catch { profitParId.value = {} }
    } catch {
      signaux.value = []
    } finally {
      chargement.value = false
    }
  }

  return { signauxFiltres, signauxTriés, totaux, mfeParId, lotParId, journalComptes, profitParId, triColonne, triDir, trierPar, chargement, charger }
}
