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
  /** Capital de départ de la stratégie (re-jeu capital). */
  const capitalDepart = ref<number | null>(null)
  /** Capital composé APRÈS chaque clôture (ordre chronologique) — variante kdj. */
  const capitalApresParId = computed<Record<string, number>>(() => {
    if (capitalDepart.value === null) return {}
    const fermes = signauxFiltres.value
      .filter(s => s.ferme_le !== null && profitParId.value[s.id] !== undefined)
      .sort((a, b) => (a.ferme_le! - b.ferme_le!))
    const carte: Record<string, number> = {}
    let capital = capitalDepart.value
    for (const s of fermes) {
      capital += profitParId.value[s.id]
      carte[s.id] = capital
    }
    return carte
  })

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

  /// Totaux du VÉCU COMPLET, servis par le backend (/api/analyses — la
  /// même source que la carte dashboard). Écart 9 du 10/10 : l'ancienne
  /// somme front ne couvrait que la fenêtre chargée (500 derniers signaux
  /// toutes stratégies → 173 SMC sur 654) — l'en-tête contredisait le
  /// dashboard. En repli si l'analyse échoue : somme de la fenêtre chargée
  /// (annotée comme telle par l'appelant via `complet`).
  const totauxBackend = ref<{ nb: number; sommeR: number } | null>(null)
  const totaux = computed(() => {
    let sommeR: number | null = null
    let enCours = 0
    if (totauxBackend.value) {
      return { sommeR: totauxBackend.value.sommeR, nb: totauxBackend.value.nb, complet: true, enCours }
    }
    // Repli (analyse indisponible) : fenêtre chargée uniquement.
    for (const s of signaux.value) {
      const nom = s.strategie.toLowerCase().trim()
      const ok = cle === 'smc' ? SMC_VARIANTES.includes(nom) : nom === cle
      if (!ok) continue
      if (s.statut !== 'Fermé') { enCours++; continue }
      if (s.heure_entree === null || s.heure_entree === undefined) continue
      const r = s.r_distance
      if (r !== null && r !== undefined) sommeR = (sommeR ?? 0) + r
    }
    return { sommeR, nb: signauxFiltres.value.length, complet: false, enCours }
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
        capitalDepart.value = analyse?.capital_depart ?? null
        // Totaux du vécu complet (écart 9) — mêmes valeurs que le dashboard.
        if (analyse?.nb_trades && analyse.r_distance_total !== undefined) {
          totauxBackend.value = { nb: analyse.nb_trades, sommeR: analyse.r_distance_total }
        }
      } catch { profitParId.value = {} }
    } catch {
      signaux.value = []
    } finally {
      chargement.value = false
    }
  }

  return { signauxFiltres, signauxTriés, totaux, mfeParId, lotParId, journalComptes, profitParId, capitalApresParId, triColonne, triDir, trierPar, chargement, charger }
}
