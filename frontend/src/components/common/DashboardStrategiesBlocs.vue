<template>
  <div class="h-full min-h-0 overflow-y-auto flex flex-col gap-2 pr-0.5">
    <div
      v-for="b in blocs"
      :key="b.id"
      class="rounded-xl border border-white/10 bg-white/5 hover:border-white/25 transition-colors cursor-pointer px-4 py-3 flex flex-col gap-2"
      :title="`Ouvrir la page ${b.nom}`"
      @click="ouvrir(b.id)"
    >
      <!-- En-tête : identité + état (titre = bouton) -->
      <div class="flex items-center gap-2">
        <span class="text-lg leading-none">{{ b.icone }}</span>
        <span class="font-semibold text-white text-sm">{{ b.nom }}</span>
        <span
          class="ml-auto text-[10px] font-semibold px-2 py-0.5 rounded-full border"
          :class="badgeClasse(b.etat)"
        >{{ b.etat }}</span>
      </div>

      <!-- Courbe des trades clôturés (R cumulé de référence) -->
      <div class="relative h-16 -mx-1">
        <svg
          v-if="b.perf.clotures.length > 1"
          :viewBox="`0 0 ${LARGEUR} ${HAUTEUR}`"
          preserveAspectRatio="none"
          class="w-full h-full"
        >
          <line
            v-if="ligneZero(b) !== null"
            :x1="0" :x2="LARGEUR" :y1="ligneZero(b)!" :y2="ligneZero(b)!"
            stroke="rgba(255,255,255,0.15)" stroke-width="0.5" stroke-dasharray="2 2"
          />
          <polyline
            :points="points(b)"
            fill="none"
            :stroke="b.perf.r_total >= 0 ? '#34d399' : '#f87171'"
            stroke-width="1.5" vector-effect="non-scaling-stroke"
            stroke-linejoin="round" stroke-linecap="round"
          />
        </svg>
        <div v-else class="w-full h-full flex items-center justify-center text-[11px] text-white">
          Courbe des trades — dès les premières clôtures
        </div>
      </div>

      <!-- Badge line : les 5 métriques (R d'abord) -->
      <div class="flex items-center gap-2 text-[11px] flex-wrap">
        <span class="font-mono font-bold" :class="b.perf.r_total >= 0 ? 'text-emerald-400' : 'text-red-400'"
              title="R de référence : paliers max atteints">{{ b.perf.r_total >= 0 ? '+' : '' }}{{ b.perf.r_total.toFixed(1) }} R</span>
        <span class="text-white">{{ b.perf.total }} rempli{{ b.perf.total > 1 ? 's' : '' }}</span>
        <span v-if="b.perf.non_remplis > 0" class="text-white" title="Ordres posés jamais touchés">{{ b.perf.non_remplis }} non remplis</span>
        <span class="text-white" title="Taux de réussite (R de référence > 0)">WR {{ (b.perf.taux_reussite * 100).toFixed(0) }} %</span>
        <span class="ml-auto text-white">{{ b.perf.en_cours.length }} en cours</span>
      </div>
    </div>

    <div v-if="!blocs.length && !chargement" class="flex-1 flex items-center justify-center text-sm text-white">
      Aucune stratégie active (hors construction)
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'

interface StrategieApi {
  id: string; nom: string; icone: string; etat: string
}
interface PerfApi {
  clotures: { ferme_le: number; r_cumule: number }[]
  en_cours: unknown[]
  total: number
  non_remplis: number
  taux_reussite: number
  /** R total de référence (paliers max atteints) — métrique primaire. */
  r_total: number
  /** R total réalisé (sorties réelles) — info secondaire. */
  r_total_realise?: number
}
interface Bloc {
  id: string; nom: string; icone: string; etat: string; perf: PerfApi
}

const LARGEUR = 100
const HAUTEUR = 32

const router = useRouter()
const blocs = ref<Bloc[]>([])
const chargement = ref(true)
let minuteur: ReturnType<typeof setInterval> | null = null

const ROUTES: Record<string, string> = {
  SMC: '/smc',
  straddle: '/straddle',
  rockets: '/rockets',
}

async function charger() {
  try {
    const res = await http.get<StrategieApi[]>('/api/strategies')
    const actives = (res.data as StrategieApi[]).filter(s => s.etat !== 'Construction')
    const complets = await Promise.allSettled(
      actives.map(async s => {
        let perf = PERF_VIDE
        try {
          const p = await http.get<PerfApi>(`/api/strategies/${s.id}/performance`)
          perf = p.data as PerfApi
        } catch { /* perf indisponible → bloc vide */ }
        return { id: s.id, nom: s.nom, icone: s.icone, etat: s.etat, perf }
      }),
    )
    blocs.value = complets.flatMap(p => (p.status === 'fulfilled' ? [p.value] : []))
  } catch {
    blocs.value = []
  }
  chargement.value = false
}

function ouvrir(id: string) {
  const cible = ROUTES[id]
  if (cible) router.push(cible)
}

function badgeClasse(etat: string) {
  if (etat === 'Officielle') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (etat === 'Observation') return 'bg-amber-500/10 text-amber-400 border-amber-500/30'
  return 'bg-gray-500/10 text-white border-gray-500/30'
}

/// Points SVG de la courbe R cumulé.
function points(b: Bloc): string {
  const vals = [...b.perf.clotures.map(c => c.r_cumule), 0]
  const min = Math.min(...vals)
  const max = Math.max(...vals)
  const amplitude = max - min || 1
  const n = b.perf.clotures.length
  return b.perf.clotures
    .map((c, i) => {
      const x = n > 1 ? (i / (n - 1)) * LARGEUR : 0
      const y = HAUTEUR - 2 - ((c.r_cumule - min) / amplitude) * (HAUTEUR - 4)
      return `${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(' ')
}

/// Position Y du zéro (référence de la courbe) — null si hors cadre.
function ligneZero(b: Bloc): number | null {
  if (b.perf.clotures.length < 2) return null
  const vals = [...b.perf.clotures.map(c => c.r_cumule), 0]
  const min = Math.min(...vals)
  const max = Math.max(...vals)
  const amplitude = max - min || 1
  return HAUTEUR - 2 - ((0 - min) / amplitude) * (HAUTEUR - 4)
}

const PERF_VIDE: PerfApi = {
  clotures: [], en_cours: [], total: 0, non_remplis: 0, taux_reussite: 0, r_total: 0,
}

onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
