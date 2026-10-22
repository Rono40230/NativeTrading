import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { alertesApi, type AlertePrix } from '@/services/api.alertes'

/**
 * Store PARTAGÉ des alertes de prix — source unique des graphiques (lignes)
 * ET du dashboard (bloc alertes armées). Règle owner 20/10 : toute pose /
 * glisser-déposer / suppression se reflète PARTOUT immédiatement, sans
 * rechargement ni attente de poll.
 *
 * Le rattrapage des déclenchées (> 2 min) vit ici : il tourne dès qu'un
 * consommateur recharge (poll chart 10 s ou dashboard 60 s), même sans
 * graphique ouvert. La NOTIFICATION (son + OS) reste au composable du
 * graphique — lui seul sait si elle est fraîche et sur son asset.
 */
export const useAlertesPrixStore = defineStore('alertes-prix', () => {
  const alertes = ref<AlertePrix[]>([])
  const actives = computed(() => alertes.value.filter(a => a.active))

  async function recharger() {
    let liste: AlertePrix[]
    try {
      liste = await alertesApi.lister()
    } catch { return }
    // Rattrapage : une alerte déclenchée n'existe nulle part — les graphs
    // notifient puis suppriment ; celles de plus de 2 min (aucun graphique
    // ouvert) partent silencieusement.
    const vieilles = liste.filter(
      a => !a.active && a.declenchee_le && Date.now() / 1000 - a.declenchee_le > 120,
    )
    if (vieilles.length) {
      await Promise.all(vieilles.map(a => alertesApi.supprimer(a.id).catch(() => null)))
      liste = liste.filter(a => !vieilles.includes(a))
    }
    alertes.value = liste
  }

  async function creer(asset: string, prix: number, sens: 'au_dessus' | 'en_dessous') {
    await alertesApi.creer(asset, prix, sens)
    await recharger()
  }

  async function maj(id: number, prix: number, sens: 'au_dessus' | 'en_dessous') {
    try { await alertesApi.maj(id, prix, sens) } catch { /* réseau : recharger tranchera */ }
    await recharger()
  }

  async function supprimer(id: number) {
    await alertesApi.supprimer(id)
    alertes.value = alertes.value.filter(a => a.id !== id)
  }

  async function supprimerPlusieurs(ids: number[]) {
    await Promise.all(ids.map(id => alertesApi.supprimer(id).catch(() => null)))
    const s = new Set(ids)
    alertes.value = alertes.value.filter(a => !s.has(a.id))
  }

  return { alertes, actives, recharger, creer, maj, supprimer, supprimerPlusieurs }
})
