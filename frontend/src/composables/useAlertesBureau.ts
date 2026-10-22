/**
 * useAlertesBureau — notifications OS + son du BUREAU (owner 21/10).
 *
 * Le journal universel (mobile_alertes) est la source : ce poller relève
 * les nouvelles entrées et notifie les familles dont le canal « app » est
 * actif dans la modale 🔔 Gérer les alertes (API /api/alertes-canaux).
 * Il remplace la notification propre d'useAlertesPrix (qui n'alertait que
 * l'asset du graphique ouvert — désormais TOUT est couvert).
 */
import { ref } from 'vue'
import { http } from '@/services/http.client'
import { useNotification } from '@/composables/useNotification'

const FAMILLE: Record<string, string> = {
  evenement: 'evenement_t10',
  presse: 'presse_100',
  smc_approche: 'smc_approche',
  smc_contact: 'smc_contact',
  prix: 'alerte_prix',
  digest: 'briefing',
  sante: 'sante',
}

interface Entree { id: number; ts: number; type: string; titre: string; famille?: string | null }

export function useAlertesBureau() {
  const { notifier } = useNotification()
  const canauxApp = ref<Record<string, boolean>>({})
  let dernierId = 0
  let premierePasse = true

  async function chargerCanaux() {
    try {
      const r = await http.get('/api/alertes-canaux')
      const m: Record<string, boolean> = {}
      for (const c of r.data as { famille: string; app: boolean }[]) m[c.famille] = c.app
      canauxApp.value = m
    } catch { /* hors ligne : on garde */ }
  }

  async function relever() {
    let entrees: Entree[] = []
    try {
      const r = await http.get('/api/mobile/journal', { params: { limite: 15 } })
      entrees = r.data.alertes as Entree[]
    } catch { return }
    if (!entrees.length) return
    const maxId = entrees[0].id
    if (dernierId === 0) { dernierId = maxId; premierePasse = false; return }
    const nouvelles = entrees.filter(e => e.id > dernierId)
    dernierId = maxId
    for (const e of nouvelles) {
      const famille = FAMILLE[e.type]
      // Signaux : la famille réelle (signaux_smc…) vient du payload.
      const cle = e.type === 'signal' ? (e.famille ?? '') : famille
      if (!cle || !canauxApp.value[cle]) continue
      if (Date.now() / 1000 - e.ts > 120) continue // vieille entrée : pas de son tardif
      void notifier(`${e.titre}`, 'Alerte', { son: true, urgence: 'normal' })
    }
  }

  async function demarrer() {
    await chargerCanaux()
    await relever()
    setInterval(() => void relever(), 15_000)
    setInterval(() => void chargerCanaux(), 120_000)
  }

  return { canauxApp, demarrer }
}
