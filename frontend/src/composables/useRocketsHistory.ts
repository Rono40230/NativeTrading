import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { Ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { RocketSignalHistorique } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'

export function useRocketsHistory(
  rocketsMode: Ref<boolean>,
  filtreStatut: Ref<'en_cours' | 'cloturees' | ''>,
  triColonne: Ref<string>,
  triDir: Ref<'asc' | 'desc'>,
) {
  const alerteStore = useAlerteStore()
  const rockets = ref<RocketSignalHistorique[]>([])
  const prixActuels = ref<Record<string, number>>({})

  // ── Chargement ────────────────────────────────────────────────────────────

  async function chargerRockets() {
    await apiService.syncRockets().catch(() => {})
    rockets.value = await apiService.historiqueRockets(200)
    const tickers = [...new Set(rockets.value.map(r => r.ticker))]
    prixActuels.value = {}
    await Promise.allSettled(
      tickers.map(async ticker => {
        const prix = await apiService.getPrixActuel(ticker)
        if (prix !== null) prixActuels.value[ticker] = prix
      })
    )
  }

  // ── Rafraîchissement prix (toutes les 5s) ─────────────────────────────────

  async function rafraichirPrix() {
    if (!rocketsMode.value || rockets.value.length === 0) return
    const enCours = rockets.value.filter(r => !r.verdict)
    const tickers = [...new Set(enCours.map(r => r.ticker))]
    if (tickers.length === 0) return

    await Promise.allSettled(
      tickers.map(async ticker => {
        const prix = await apiService.getPrixActuel(ticker)
        if (prix !== null) prixActuels.value[ticker] = prix
      })
    )

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
      await apiService.syncRockets().catch(() => {})
      rockets.value = await apiService.historiqueRockets(200).catch(() => rockets.value)
    }
  }

  // ── Boucle de rafraîchissement ────────────────────────────────────────────

  let timeoutPrix: ReturnType<typeof setTimeout> | null = null
  let actif = false

  async function boucleRafraichissement() {
    if (!actif) return
    await rafraichirPrix()
    if (actif) timeoutPrix = setTimeout(boucleRafraichissement, 5_000)
  }

  onMounted(() => { actif = true; boucleRafraichissement() })
  onUnmounted(() => { actif = false; if (timeoutPrix) clearTimeout(timeoutPrix) })

  // ── Filtrage + tri ────────────────────────────────────────────────────────

  const rocketsFiltrés = computed(() => {
    if (filtreStatut.value === 'en_cours') return rockets.value.filter(r => !r.verdict)
    if (filtreStatut.value === 'cloturees') return rockets.value.filter(r => !!r.verdict)
    return rockets.value
  })

  const rocketsTries = computed(() => {
    const col = triColonne.value
    const liste = [...rocketsFiltrés.value] as unknown as Record<string, unknown>[]
    if (!col) return rocketsFiltrés.value
    return liste.sort((a, b) => {
      let va: unknown = a[col] ?? ''
      let vb: unknown = b[col] ?? ''
      if (typeof va === 'string') va = va.toLowerCase()
      if (typeof vb === 'string') vb = vb.toLowerCase()
      const cmp = (va as string | number) < (vb as string | number) ? -1
        : (va as string | number) > (vb as string | number) ? 1 : 0
      return triDir.value === 'asc' ? cmp : -cmp
    }) as unknown as RocketSignalHistorique[]
  })

  return { rockets, prixActuels, chargerRockets, rocketsTries, rocketsFiltrés }
}
