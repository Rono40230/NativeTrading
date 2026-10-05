/// Armement événementiel straddle (28/09, phase 3) — source partagée de
/// la modale 🎯 « Assets & créneaux » et de l'agenda de la page Straddle :
/// état armé par (asset × événement), stats de test, verdicts de la boucle,
/// archive des créneaux statistiques remplacés. Les ratios de réactivité
/// (×N) de la matrice sont joints pour éclairer l'armement.
import { ref } from 'vue'
import { http } from '@/services/http.client'
import { apiService } from '@/services/api.service'
import type { ReponseEvenements } from '@/services/api.service'

export interface LigneArmement {
  asset: string
  arme: boolean
  occurrences: number
  somme_r: number
  verdict_test: string | null
  conclut_le: number | null
  hors_perimetre: boolean
  /// Réactivité mesurée (×N) jointe depuis la matrice — absente si l'asset
  /// n'a pas assez d'historique M1.
  ratio?: number
}

export interface EvenementArmement {
  ident: string
  nom: string
  fuseau: string
  heure_locale: string
  prochaine_ts: number | null
  prochaine_heure_paris: string | null
  lignes: LigneArmement[]
}

export interface VerdictArchive {
  asset: string
  jour: number
  heure: number
  verdict_test: string
  occurrences: number
  somme_r: number
}

export interface RepArmement {
  assets: string[]
  evenements: EvenementArmement[]
  seuils: { min: number; plancher_r: number }
  archive: { total: number; verdicts: VerdictArchive[] }
}

export function useEvenementsArmement() {
  const evenements = ref<EvenementArmement[]>([])
  const assets = ref<string[]>([])
  const seuils = ref({ min: 4, plancher_r: -1.5 })
  const archive = ref<RepArmement['archive']>({ total: 0, verdicts: [] })
  const chargement = ref(true)
  const enCours = ref(false)

  async function charger() {
    chargement.value = true
    try {
      const [resA, resM] = await Promise.all([
        http.get<RepArmement>('/api/evenements/armement'),
        apiService.obtenirMatriceEvenements().catch(() => null as ReponseEvenements | null),
      ])
      const rep = resA.data
      // Jointure des ratios de réactivité (matrice, cache serveur 1 h).
      const ratios = new Map<string, number>()
      if (resM) {
        for (const ev of resM.evenements) {
          for (const l of ev.reactivite) ratios.set(`${ev.ident}|${l.asset}`, l.ratio)
        }
      }
      for (const ev of rep.evenements) {
        for (const l of ev.lignes) l.ratio = ratios.get(`${ev.ident}|${l.asset}`)
      }
      evenements.value = rep.evenements
      assets.value = rep.assets
      seuils.value = rep.seuils
      archive.value = rep.archive
    } catch {
      evenements.value = []
    }
    chargement.value = false
  }

  /// Bascule une case (asset × événement) — armer = nouveau test à zéro.
  async function basculer(asset: string, ident: string) {
    enCours.value = true
    try {
      await http.post('/api/evenements/armement/basculer', { asset, evenement: ident })
      await charger()
    } finally {
      enCours.value = false
    }
  }

  /// Balayage large : armer (ou désarmer) toutes les cases d'un coup.
  async function toutArmer(arme: boolean) {
    enCours.value = true
    try {
      await http.post('/api/evenements/armement/tout', { arme }, { timeout: 60_000 })
      await charger()
    } finally {
      enCours.value = false
    }
  }

  async function sauverSeuils(min: number, plancher_r: number) {
    await http.put('/api/evenements/seuils', { min: Math.round(min), plancher_r })
    await charger()
  }

  return { evenements, assets, seuils, archive, chargement, enCours, charger, basculer, toutArmer, sauverSeuils }
}
