import { ref, computed, watch } from 'vue'
import type { Ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketSignalHistorique, Signal } from '@/services/api.types'
import { usePrixStore } from '@/stores/prix.store'

// Normalise les verdicts rockets vers les valeurs Signal affichées dans HistoryView (SMC/Straddle)
function normaliserVerdictRocket(v: string | null): string | null {
  if (v === 'sl') return 'SL'
  if (v === 'invalide') return 'invalide'  // badge orange distinct du SL
  if (v === 'confirme') return 'TP1'
  return v // tp1, tp2, tp3, be, expire, null → inchangés
}

export function rocketToSignal(r: RocketSignalHistorique): Signal {
  const verdict = normaliserVerdictRocket(r.verdict)
  return {
    id: String(r.id),
    asset: r.ticker,
    timeframe: r.phase,
    direction: 'LONG',
    score: r.score,
    prix_entree: r.prix_entree,
    stop_loss: r.stop_loss,
    take_profit: [r.target, ...(r.target2 ? [r.target2] : []), ...(r.target3 ? [r.target3] : [])],
    strategie: 'Rockets',
    statut: r.statut === 'ferme' ? 'Fermé' : 'Actif',
    verdict,
    prix_verdict: r.prix_verdict,
    ferme_le: r.maj_le ? Math.floor(new Date(r.maj_le.replace(' ', 'T') + 'Z').getTime() / 1000) : null,
    cree_le: Math.floor(new Date(r.cree_le.replace(' ', 'T') + 'Z').getTime() / 1000),
    llm_valide: r.llm_valide,
    llm_conviction: r.llm_conviction,
    llm_raison: r.llm_raison,
    llm_sl_suggere: null,
    llm_tp1_suggere: null,
  }
}

export function useRocketsHistory(
  filtreStatut: Ref<'en_cours' | 'cloturees' | ''>,
  triColonne: Ref<string>,
  triDir: Ref<'asc' | 'desc'>,
) {
  const prixStore = usePrixStore()
  // Garantit que le polling des prix est actif même si le Dashboard n'est pas monté
  prixStore.demarrer()
  const rockets = ref<RocketSignalHistorique[]>([])

  // prixActuels est un computed dérivé directement du store :
  // toujours réactif, se met à jour automatiquement à chaque poll (toutes les 10s)
  const prixActuels = computed<Record<string, number>>(() => {
    const result: Record<string, number> = {}
    for (const r of rockets.value) {
      if (r.verdict) continue
      const prix = prixStore.getPrix(r.ticker)
      if (prix !== null) result[r.ticker] = prix
    }
    return result
  })

  // Franchissement SL/TP → re-sync depuis le backend pour récupérer le verdict
  watch(prixActuels, (nouveaux) => {
    const enCours = rockets.value.filter(r => r.statut !== 'ferme')
    const franchissement = enCours.some(r => {
      const prix = nouveaux[r.ticker]
      if (!prix) return false
      if (prix <= r.stop_loss) return true
      if (r.target3 && prix >= r.target3) return true
      if (r.target2 && prix >= r.target2) return true
      if (!r.target2 && prix >= r.target) return true
      return false
    })
    if (franchissement) {
      apiService.syncRockets().catch(() => {}).then(() =>
        apiService.historiqueRockets(200).then(data => { rockets.value = data }).catch(() => {})
      )
    }
  }, { deep: true })

  // ── Chargement ────────────────────────────────────────────────────────────

  async function chargerRockets() {
    await apiService.syncRockets().catch(() => {})
    rockets.value = await apiService.historiqueRockets(200)
    // Abonner les tickers ouverts au WS prix (1s) — couvre FRONT et tout autre token
    const openTickers = rockets.value.filter(r => r.statut !== 'ferme').map(r => r.ticker)
    if (openTickers.length > 0) prixStore.abonner(openTickers)
  }

  // ── Filtrage + tri ────────────────────────────────────────────────────────

  const rocketsFiltrés = computed(() => {
    if (filtreStatut.value === 'en_cours') return rockets.value.filter(r => r.statut !== 'ferme')
    if (filtreStatut.value === 'cloturees') return rockets.value.filter(r => r.statut === 'ferme')
    return rockets.value
  })

  // Priorité de tri pour le verdict :
  // tient compte du verdict stocké ET du prix live pour les rockets "en cours"
  const VERDICT_ORDRE: Record<string, number> = {
    invalide: 1, expire: 2, confirme: 3, TP1: 3, TP2: 4, TP3: 5,
  }

  function prioriteEffective(r: RocketSignalHistorique): number {
    if (r.verdict) return VERDICT_ORDRE[r.verdict] ?? 0
    const prix = prixActuels.value[r.ticker]
    if (!prix) return 0
    if (r.target3 && prix >= r.target3) return 5.5
    if (r.target2 && prix >= r.target2) return 4.5
    if (prix >= r.target) return 3.5
    if (prix <= r.stop_loss) return 1.5
    return 0
  }

  const rocketsTries = computed(() => {
    const col = triColonne.value
    const dir = triDir.value
    if (!col) return rocketsFiltrés.value

    return [...rocketsFiltrés.value].sort((a, b) => {
      let cmp: number
      if (col === 'verdict') {
        cmp = prioriteEffective(a) - prioriteEffective(b)
      } else {
        const ra = a as unknown as Record<string, unknown>
        const rb = b as unknown as Record<string, unknown>
        let va: unknown = ra[col] ?? ''
        let vb: unknown = rb[col] ?? ''
        if (typeof va === 'string') va = va.toLowerCase()
        if (typeof vb === 'string') vb = vb.toLowerCase()
        cmp = (va as string | number) < (vb as string | number) ? -1
            : (va as string | number) > (vb as string | number) ? 1 : 0
      }
      return dir === 'asc' ? cmp : -cmp
    })
  })

  return { rockets, prixActuels, chargerRockets, rocketsTries, rocketsFiltrés }
}
