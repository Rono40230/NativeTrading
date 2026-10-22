/**
 * Badges de non-lus pour les onglets « Alertes SMC » et « Prix »
 * (owner 20/10). L'état « déjà vu » vit dans le téléphone (localStorage,
 * dernier id consulté par famille) — chaque appareil a son compteur,
 * comme des notifications. SMC compte la veille de la dernière heure ;
 * Prix ne compte que les DÉCLENCHÉES (les niveaux armés ne sont pas
 * des alertes à lire).
 */
import { ref } from 'vue'
import { api } from './api'
import type { Alerte } from './types'

type Famille = 'smc' | 'prix'

const CLES: Record<Famille, string> = { smc: 'nt_vu_smc', prix: 'nt_vu_prix' }
const vus: Record<Famille, number> = {
  smc: Number(localStorage.getItem(CLES.smc) ?? 0),
  prix: Number(localStorage.getItem(CLES.prix) ?? 0),
}
const maxIds: Record<Famille, number> = { smc: vus.smc, prix: vus.prix }

export const nonLus = ref<{ smc: number; prix: number }>({ smc: 0, prix: 0 })

const demo = new URLSearchParams(location.search).has('demo')
if (demo) nonLus.value = { smc: 1, prix: 0 } // affordance visuelle en démo

function compter(f: Famille, alertes: Alerte[]) {
  maxIds[f] = alertes.reduce((m, a) => Math.max(m, a.id), maxIds[f])
  nonLus.value[f] = alertes.filter((a) => a.id > vus[f]).length
}

/// Recompte les non-lus (montage, minute, retour en ligne).
export async function rafraichirBadges() {
  if (demo) return
  try {
    const rep = await api.journal(undefined, 'smc_approche,smc_contact', 60)
    // Garde : le vieux binaire ignore type/fenetre — on filtre ici.
    const depuis = Date.now() / 1000 - 3600
    compter(
      'smc',
      rep.alertes.filter(
        (a) => (a.type === 'smc_approche' || a.type === 'smc_contact') && a.ts >= depuis,
      ),
    )
  } catch {
    // hors ligne : les badges gardent leur dernière valeur
  }
  try {
    let list: Alerte[]
    try {
      list = (await api.prix()).declenchees
    } catch {
      // Vieux binaire : endpoint mobile absent → journal filtré.
      list = (await api.journal(undefined, 'prix')).alertes.filter((a) => a.type === 'prix')
    }
    compter('prix', list)
  } catch {
    // hors ligne
  }
}

/// Ouvrir l'onglet = tout consulter : le compteur retombe à zéro.
export function marquerVus(f: Famille) {
  vus[f] = Math.max(vus[f], maxIds[f])
  localStorage.setItem(CLES[f], String(vus[f]))
  nonLus.value[f] = 0
}
