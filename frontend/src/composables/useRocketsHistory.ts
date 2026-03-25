import { ref, computed, watch } from 'vue'
import type { Ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketSignalHistorique, Signal } from '@/services/api.types'
import { usePrixStore } from '@/stores/prix.store'

// Normalise les verdicts rockets (invalide/confirme) vers les valeurs Signal (SL/TP1)
function normaliserVerdictRocket(v: string | null): string | null {
  if (v === 'invalide') return 'SL'
  if (v === 'confirme') return 'TP1'
  return v // TP1, TP2, TP3, expire, null → inchangés
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
    statut: verdict ? 'Fermé' : 'Actif',
    verdict,
    prix_verdict: r.prix_verdict,
    ferme_le: null,
    cree_le: Math.floor(new Date(r.cree_le).getTime() / 1000),
    llm_valide: r.llm_valide,
    llm_conviction: r.llm_conviction,
    llm_raison: r.llm_raison,
    llm_sl_suggere: null,
    llm_tp1_suggere: null,
  }
}

export function useRocketsHistory(
  rocketsMode: Ref<boolean>,
  filtreStatut: Ref<'en_cours' | 'cloturees' | ''>,
  triColonne: Ref<string>,
  triDir: Ref<'asc' | 'desc'>,
) {
  const prixStore = usePrixStore()
  const rockets = ref<RocketSignalHistorique[]>([])
  const prixActuels = ref<Record<string, number>>({})

  // ── Mise à jour prix depuis le store centralisé ───────────────────────────

  function mettreAJourPrix() {
    const enCours = rockets.value.filter(r => !r.verdict)
    if (enCours.length === 0) return

    for (const r of enCours) {
      const prix = prixStore.getPrix(r.ticker)
      if (prix !== null) prixActuels.value[r.ticker] = prix
    }

    const franchissement = enCours.some(r => {
      const prix = prixActuels.value[r.ticker]
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
  }

  // Réagit automatiquement aux mises à jour du store (toutes les 10s)
  watch(() => prixStore.tickers, () => {
    if (rocketsMode.value && rockets.value.length > 0) mettreAJourPrix()
  })

  // ── Chargement ────────────────────────────────────────────────────────────

  async function chargerRockets() {
    await apiService.syncRockets().catch(() => {})
    rockets.value = await apiService.historiqueRockets(200)
    mettreAJourPrix()
  }

  // ── Filtrage + tri ────────────────────────────────────────────────────────

  const rocketsFiltrés = computed(() => {
    if (filtreStatut.value === 'en_cours') return rockets.value.filter(r => !r.verdict)
    if (filtreStatut.value === 'cloturees') return rockets.value.filter(r => !!r.verdict)
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
