import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { Signal, RocketSignalHistorique } from '@/services/api.types'
import { apiService } from '@/services/api.service'
import { http } from '@/services/http.client'
import { usePrixStore } from '@/stores/prix.store'
import { useAssetParamsStore } from '@/stores/assetParams.store'
import { useSettingsStore } from '@/stores/settings.store'
import { rocketToSignal } from '@/composables/useRocketsHistory'

export function useSignauxTableau(strategie: 'SMC' | 'straddle' | 'Rockets') {
  const prixStore = usePrixStore()
  const assetParamsStore = useAssetParamsStore()
  const settingsStore = useSettingsStore()

  const signaux = ref<Signal[]>([])
  const rocketsRaw = ref<RocketSignalHistorique[]>([])
  const chargement = ref(true)
  const analyseOuverte = ref(false)
  const filtreStatut = ref<'en_cours' | 'cloturees' | ''>('en_cours')
  const triColonne = ref('')
  const triDir = ref<'asc' | 'desc'>('desc')
  const annulationEnCours = ref(new Set<string | number>())

  async function annuler(s: Signal) {
    const rkt = rocketsRaw.value.find(r => r.ticker === s.asset)
    if (!rkt) return
    annulationEnCours.value = new Set(annulationEnCours.value).add(s.id)
    try {
      await apiService.annulerRocket(rkt.id)
      rocketsRaw.value = rocketsRaw.value.filter(r => r.id !== rkt.id)
      signaux.value = rocketsRaw.value.map(rocketToSignal)
    } catch { /* silencieux */ } finally {
      const s2 = new Set(annulationEnCours.value)
      s2.delete(s.id)
      annulationEnCours.value = s2
    }
  }

  function trierPar(col: string) {
    if (triColonne.value === col) triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
    else { triColonne.value = col; triDir.value = 'desc' }
  }

  function icone(col: string): string {
    if (triColonne.value !== col) return '⇅'
    return triDir.value === 'asc' ? '↑' : '↓'
  }

  function classeConviction(c: number | null): string {
    if (c === null) return 'bg-gray-700 text-gray-400'
    return c >= 70 ? 'bg-emerald-900 text-emerald-300 border border-emerald-600'
      : c >= 50 ? 'bg-yellow-900 text-yellow-300 border border-yellow-600'
      : 'bg-red-900 text-red-300 border border-red-600'
  }

  function classePrix(s: Signal): string {
    const prix = prixStore.getPrix(s.asset)
    if (!prix || s.direction === 'Both') return 'text-gray-400'
    const long = s.direction === 'LONG'
    if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'text-red-400'
    if (s.take_profit[2] && (long ? prix >= s.take_profit[2] : prix <= s.take_profit[2])) return 'text-emerald-200'
    if (s.take_profit[1] && (long ? prix >= s.take_profit[1] : prix <= s.take_profit[1])) return 'text-emerald-300'
    return (long ? prix >= s.take_profit[0] : prix <= s.take_profit[0]) ? 'text-emerald-400' : 'text-blue-300'
  }

  /// Option A (01/09) : ne garder que les trades RÉELLEMENT engagés —
  /// SMC : entrée touchée (heure_entree) · Straddle : heure E atteinte ·
  /// Rockets : tout (position ouverte dès le signal). Les ordres en
  /// attente vivent dans la section Setups (SignauxEnAttente).
  const remplisSeuls = ref(false)

  const estEngage = (s: typeof signaux.value[number]): boolean => {
    if (s.statut === 'Fermé') return false
    if (strategie === 'SMC') return s.heure_entree !== null && s.heure_entree !== undefined
    if (strategie === 'straddle')
      return (s.heure_entree ?? 0) <= Math.floor(Date.now() / 1000)
    return true
  }

  const listeActive = computed(() =>
    signaux.value.filter(s => {
      if (filtreStatut.value === 'en_cours') return remplisSeuls.value ? estEngage(s) : s.statut !== 'Fermé'
      if (filtreStatut.value === 'cloturees') return s.statut === 'Fermé'
      return true
    })
  )

  const signauxTries = computed(() => {
    const col = triColonne.value
    if (!col) return listeActive.value
    return [...listeActive.value].sort((a, b) => {
      let va: unknown, vb: unknown
      if (col === 'tp1') { va = a.take_profit[0] ?? 0; vb = b.take_profit[0] ?? 0 }
      else { va = (a as Record<string, unknown>)[col] ?? ''; vb = (b as Record<string, unknown>)[col] ?? '' }
      if (typeof va === 'string') va = va.toLowerCase()
      if (typeof vb === 'string') vb = vb.toLowerCase()
      const cmp = (va as string | number) < (vb as string | number) ? -1 : (va as string | number) > (vb as string | number) ? 1 : 0
      return triDir.value === 'asc' ? cmp : -cmp
    })
  })

  /// Risque % de la verticale (registre stratégies) — la table asset_params
  /// n'a jamais porté de risque_pct : l'ancien calcul produisait « Lot : NaN ».
  const ID_REGISTRE: Record<'SMC' | 'straddle' | 'Rockets', string> = {
    SMC: 'SMC', straddle: 'straddle', Rockets: 'rockets',
  }
  const risquePct = ref(0)

  async function chargerRisque() {
    try {
      const res = await http.get<{ id: string; risque_pct?: number }[]>('/api/strategies')
      risquePct.value = res.data.find(r => r.id === ID_REGISTRE[strategie])?.risque_pct ?? 0
    } catch { risquePct.value = 0 }
  }

  function lotPourSignal(s: Signal): string {
    const params = assetParamsStore.liste.find(p => p.asset === s.asset)
    if (!params) return ''
    const capital = settingsStore.capitalDepart
    if (capital <= 0 || risquePct.value <= 0 || params.valeur_pips <= 0 || params.sl_pips <= 0) return ''
    const investi = capital * (risquePct.value / 100)
    const lot = investi / (params.sl_pips * params.valeur_pips)
    const lotClampe = Math.min(Math.max(lot, params.lot_min), params.lot_max)
    return lotClampe.toFixed(2)
  }

  /// Montant risqué du trade en dollars (capital × risque % de la verticale).
  function montantRisque(): number {
    return settingsStore.capitalDepart * (risquePct.value / 100)
  }


  function infosPips(cible: number | null | undefined, base: number, actif: string): string {
    if (!cible || cible === 0 || base === 0 || !actif) return ''
    const param = assetParamsStore.liste.find(p => p.asset === actif)
    if (!param || param.taille_pip <= 0) return ''
    const distanceAbs = Math.abs(cible - base)
    const pips = (distanceAbs / param.taille_pip).toFixed(1)
    const pts = (distanceAbs / (param.taille_pip / param.pip_to_points)).toFixed(0)
    return `(${pips} pips | ${pts} pts)`
  }

  async function charger() {
    if (!listeActive.value.length) chargement.value = true
    void chargerRisque()
    try {
      if (strategie === 'Rockets') {
        rocketsRaw.value = await apiService.rocketsActifs()
        signaux.value = rocketsRaw.value.map(rocketToSignal)
        const openTickers = rocketsRaw.value.map(r => r.ticker)
        if (openTickers.length > 0) prixStore.abonner(openTickers)
      } else {
        const data = await apiService.getSignaux(500)
        const SMC_NOMS = ['SMC', 'SmcDirectional', 'SMC Directionnel', 'SMC+IA']
        signaux.value = data.filter(s =>
          strategie === 'SMC'
            ? SMC_NOMS.includes(s.strategie)
            : s.strategie.toLowerCase() === strategie.toLowerCase()
        )
      }
    } catch { /* silencieux */ } finally {
      chargement.value = false
    }
  }

  let _poll: ReturnType<typeof setInterval> | null = null

  onMounted(() => {
    charger()
    if (!assetParamsStore.liste.length) assetParamsStore.charger()
    _poll = setInterval(() => charger(), 30_000)
  })

  onUnmounted(() => {
    if (_poll !== null) { clearInterval(_poll); _poll = null }
  })

  return {
    signaux, rocketsRaw, chargement, analyseOuverte, filtreStatut, remplisSeuls, estEngage, montantRisque,
    annulationEnCours, listeActive, signauxTries,
    charger, annuler, trierPar, icone, infosPips,
    classeConviction, classePrix, lotPourSignal,
    prixStore, assetParamsStore, settingsStore,
  }
}
