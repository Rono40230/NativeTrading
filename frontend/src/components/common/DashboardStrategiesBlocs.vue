<template>
  <div class="h-full min-h-0 overflow-y-auto flex flex-col gap-2 pr-0.5">
    <div
      v-for="b in blocs"
      :key="b.id"
      class="rounded-xl border border-white/10 bg-white/5 hover:border-white/20 transition-colors cursor-pointer px-4 py-3 flex flex-col gap-2"
      @click="ouvrir(b.id)"
    >
      <!-- En-tête : identité + état -->
      <div class="flex items-center gap-2">
        <span class="text-lg leading-none">{{ b.icone }}</span>
        <span class="font-semibold text-white text-sm">{{ b.nom }}</span>
        <span
          class="ml-auto text-[10px] font-semibold px-2 py-0.5 rounded-full border"
          :class="badgeClasse(b.etat)"
        >{{ b.etat }}</span>
      </div>

      <!-- Section AGENDA (straddle uniquement — étape 4) : événements qui
           arment la stratégie + passes en cours + actifs en attente MT5. -->
      <div v-if="b.id === 'straddle' && agenda" class="flex flex-col gap-1.5 border-b border-white/5 pb-2">
        <div v-if="agenda.annonces.length" class="flex flex-col gap-1">
          <div v-for="a in agenda.annonces.slice(0, 3)" :key="a.ts"
               class="flex items-center gap-2 text-xs">
            <span class="text-amber-400">📅</span>
            <span class="text-gray-300 font-medium truncate">{{ a.titre || 'Annonce US' }}</span>
            <span class="text-gray-500">{{ heureLocale(a.ts) }}</span>
            <span class="ml-auto text-amber-300/90 font-mono text-[11px]">{{ compteARebours(a.ts) }}</span>
          </div>
        </div>
        <div v-else class="text-[11px] text-gray-600">Aucune annonce US forte à 7 jours</div>
        <div v-if="agenda.passes.length" class="text-[11px] text-emerald-400/80">
          {{ agenda.passes.length }} passe(s) en cours sur {{ [...new Set(agenda.passes.map(p => p.asset))].join(', ') }}
        </div>
        <div class="text-[10px] text-gray-600">NAS100 · SP500 · DAX armés au branchement MT5</div>
      </div>

      <!-- Courbe des trades clôturés (R cumulé) -->
      <div class="relative h-20 -mx-1">
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
        <div v-else class="w-full h-full flex items-center justify-center text-[11px] text-gray-600">
          Courbe des trades — dès les premières clôtures
        </div>
      </div>

      <!-- Stats + en-cours — R de référence (palier max atteint) en métrique
           primaire, R réalisé (sorties) en info secondaire (spéc 31/08). -->
      <div class="flex items-center gap-4 text-xs">
        <span
          class="font-mono font-bold cursor-help"
          :class="b.perf.r_total >= 0 ? 'text-emerald-400' : 'text-red-400'"
          title="R de référence : palier max atteint par trade (SL ou TP max touché)"
        >
          {{ b.perf.r_total >= 0 ? '+' : '' }}{{ b.perf.r_total.toFixed(1) }} R
        </span>
        <span
          v-if="b.perf.r_total_realise !== undefined"
          class="font-mono text-gray-500 cursor-help"
          title="R réalisé : sorties réelles (trailing, BE, time-stop)"
        >réalisé {{ b.perf.r_total_realise >= 0 ? '+' : '' }}{{ b.perf.r_total_realise.toFixed(1) }} R</span>
        <span class="text-gray-400">{{ b.perf.total }} trade{{ b.perf.total > 1 ? 's' : '' }} rempli{{ b.perf.total > 1 ? 's' : '' }}</span>
        <span v-if="b.perf.non_remplis > 0" class="text-gray-600" title="Ordres posés jamais touchés par le prix">· {{ b.perf.non_remplis }} jamais remplis</span>
        <span v-if="b.perf.total > 0" class="text-gray-400">WR {{ (b.perf.taux_reussite * 100).toFixed(0) }} %</span>
        <span class="ml-auto text-gray-500">{{ b.perf.en_cours.length }} en cours</span>
      </div>

      <!-- Setups en formation (annonces intrabar — la face app de Telegram) -->
      <div v-if="b.id === 'SMC'" class="border-b border-white/5 pb-2">
        <SetupsFormationPanel strategie="SMC" />
      </div>

      <!-- Signaux en cours (liste courte) -->
      <div v-if="b.perf.en_cours.length" class="flex flex-col gap-1">
        <div
          v-for="s in b.perf.en_cours.slice(0, 4)"
          :key="`${s.asset}-${s.timeframe}-${s.cree_le}`"
          class="flex items-center gap-2 text-xs text-gray-300"
        >
          <span class="font-semibold text-white">{{ s.asset }}</span>
          <span class="text-gray-500">{{ s.timeframe }}</span>
          <span :class="s.direction === 'Long' ? 'text-emerald-400' : 'text-red-400'">
            {{ s.direction === 'Long' ? '🟢' : '🔴' }} {{ s.direction === 'Long' ? 'Achat' : 'Vente' }}
          </span>
          <span class="ml-auto text-gray-500 font-mono">force {{ s.force }}/10</span>
        </div>
        <div v-if="b.perf.en_cours.length > 4" class="text-[10px] text-gray-600">
          + {{ b.perf.en_cours.length - 4 }} autre{{ b.perf.en_cours.length - 4 > 1 ? 's' : '' }}
        </div>
      </div>
    </div>

    <div v-if="!blocs.length && !chargement" class="flex-1 flex items-center justify-center text-sm text-gray-600">
      Aucune stratégie active (hors construction)
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import SetupsFormationPanel from '@/components/common/SetupsFormationPanel.vue'

interface StrategieApi {
  id: string; nom: string; icone: string; etat: string
}
interface PerfApi {
  clotures: { ferme_le: number; r_cumule: number }[]
  en_cours: { asset: string; timeframe: string; direction: string; force: number; cree_le: number }[]
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
const HAUTEUR = 40

const router = useRouter()
const blocs = ref<Bloc[]>([])
const chargement = ref(true)
let minuteur: ReturnType<typeof setInterval> | null = null

const ROUTES: Record<string, string> = {
  SMC: '/smc',
  straddle: '/straddle',
  rockets: '/rockets',
}

const PERF_VIDE: PerfApi = {
  clotures: [], en_cours: [], total: 0, non_remplis: 0, taux_reussite: 0, r_total: 0,
}

interface AgendaApi {
  annonces: { ts: number; titre: string; devise: string; actifs: string[] }[]
  passes: { asset: string; direction: string }[]
}
const agenda = ref<AgendaApi | null>(null)

async function chargerAgenda() {
  try {
    const res = await http.get<AgendaApi>('/api/straddle/agenda')
    agenda.value = res.data as AgendaApi
  } catch { /* agenda indisponible */ }
}

function heureLocale(ts: number): string {
  return new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit' }).format(new Date(ts * 1000))
}

function compteARebours(ts: number): string {
  const d = ts - Math.floor(Date.now() / 1000)
  if (d <= 0) return 'en cours'
  const j = Math.floor(d / 86400)
  const h = Math.floor((d % 86400) / 3600)
  const m = Math.floor((d % 3600) / 60)
  if (j > 0) return `J-${j} ${h}h`
  if (h > 0) return `${h}h${String(m).padStart(2, '0')}`
  return `${m} min`
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
  return 'bg-gray-500/10 text-gray-400 border-gray-500/30'
}

/// Points SVG de la courbe R cumulé (échelle relative, zero aligné quand possible).
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

onMounted(async () => {
  await Promise.allSettled([charger(), chargerAgenda()])
  minuteur = setInterval(charger, 60_000)
  minuteurAgenda = setInterval(chargerAgenda, 60_000)
})
let minuteurAgenda: ReturnType<typeof setInterval> | null = null
onUnmounted(() => {
  if (minuteur !== null) clearInterval(minuteur)
  if (minuteurAgenda !== null) clearInterval(minuteurAgenda)
})
</script>
