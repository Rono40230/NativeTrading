/// Badge « N en cours » des cartes stratégies du dashboard (15/09) —
/// généralisation du badge Rockets aux quatre verticales. Un trade est
/// « en cours » quand il est Actif ET rempli (même sémantique que la
/// colonne Engagé du tableau des signaux : SMC/KDJ = heure d'entrée
/// présente, straddle = heure d'entrée passée). Rockets garde son badge
/// natif (poste d'observation avec P/L latent) — exclu ici.

import { computed, type Ref } from 'vue'
import { usePrixStore } from '@/stores/prix.store'

export interface SignalEnCours {
  id: string
  asset: string
  timeframe: string
  strategie: string
  statut: string
  direction: string
  prix_entree: number
  stop_loss: number
  heure_entree: number | null
}

const IDS_BLOCS = ['SMC', 'straddle', 'kdj_halftrend']

export function useEnCoursStrategies(signaux: Ref<SignalEnCours[]>) {
  const prixStore = usePrixStore()

  /// Appartenance d'un signal à une carte (la base peut porter des
  /// variantes de nom SMC : « SMC », « SmcDirectionnel »…).
  function appartient(idBloc: string, strategie: string): boolean {
    if (idBloc === 'SMC') return strategie.toUpperCase().startsWith('SMC')
    return strategie.toLowerCase() === idBloc
  }

  /// Signaux en position, groupés par id de carte.
  const parBloc = computed<Record<string, SignalEnCours[]>>(() => {
    const maintenant = Math.floor(Date.now() / 1000)
    const map: Record<string, SignalEnCours[]> = {}
    for (const s of signaux.value) {
      if (s.statut !== 'Actif' || s.heure_entree === null) continue
      for (const id of IDS_BLOCS) {
        if (!appartient(id, s.strategie)) continue
        if (id === 'straddle' && s.heure_entree > maintenant) continue
        ;(map[id] ??= []).push(s)
      }
    }
    return map
  })

  /// R latent d'un signal : dir × (prix − entrée) / |entrée − SL| —
  /// null si le prix live n'est pas disponible.
  function rLatent(s: SignalEnCours): number | null {
    const prix = prixStore.getPrix(s.asset)
    if (prix === null || prix === 0) return null
    const dir = s.direction === 'Long' ? 1 : -1
    const risque = Math.abs(s.prix_entree - s.stop_loss)
    if (risque === 0) return null
    return (dir * (prix - s.prix_entree)) / risque
  }

  /// Tooltip du badge : une ligne par trade en cours, R latent compris.
  function titre(idBloc: string): string {
    const liste = parBloc.value[idBloc] ?? []
    const lignes = liste.map(s => {
      const r = rLatent(s)
      const rTxt = r === null ? '—' : `${r >= 0 ? '+' : '−'}${Math.abs(r).toFixed(2)} R`
      return `${s.asset} ${s.timeframe} · ${s.direction === 'Long' ? '▲' : '▼'} · ${rTxt}`
    })
    return [`Trades en cours (${liste.length})`, ...lignes].join('\n')
  }

  return { parBloc, titre }
}
