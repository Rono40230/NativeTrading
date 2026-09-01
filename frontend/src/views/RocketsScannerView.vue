<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">
    <div v-if="!embarque" class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🔭 Scanner Rockets</h1>
      <span class="text-white text-base hidden sm:inline">candidats VCP en attente de pivot</span>
      <div class="flex gap-1 ml-2">
        <button
          v-for="f in filtresUnivers" :key="f.val"
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreUnivers === f.val }"
          @click="filtreUnivers = f.val"
        >{{ f.label }}</button>
      </div>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto glass-card">
      <div v-if="chargement && !candidats.length" class="text-center text-white py-10 text-sm">Chargement…</div>
      <div v-else-if="!candidatsAffiches.length" class="text-center text-white py-10 text-sm">
        Aucun candidat — le scanner quotidien n'a rien retenu (seuil 5/10)
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-white text-xs uppercase border-b border-white/10">
            <th class="px-3 py-2.5 text-left">#</th>
            <th class="px-3 py-2.5 text-left cursor-pointer select-none hover:text-white" @click="trierPar('symbole')">Symbole <span class="text-[9px]">{{ iconeTri('symbole') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('univers')">Type <span class="text-[9px]">{{ iconeTri('univers') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('points')">Classement <span class="text-[9px]">{{ iconeTri('points') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('verdict')">Verdict <span class="text-[9px]">{{ iconeTri('verdict') }}</span></th>
            <th class="px-3 py-2.5 text-right cursor-pointer select-none hover:text-white" @click="trierPar('pivot')">Pivot <span class="text-[9px]">{{ iconeTri('pivot') }}</span></th>
            <th class="px-3 py-2.5 text-right cursor-pointer select-none hover:text-white" @click="trierPar('stop')">Invalidation <span class="text-[9px]">{{ iconeTri('stop') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('cassure')">Cassure <span class="text-[9px]">{{ iconeTri('cassure') }}</span></th>
            <th class="px-3 py-2.5 text-center">News (IA)</th>
            <th class="px-3 py-2.5 text-left">Critères</th>
            <th class="px-3 py-2.5 text-left cursor-pointer select-none hover:text-white" @click="trierPar('maj_le')">Détecté le <span class="text-[9px]">{{ iconeTri('maj_le') }}</span></th>
            <th class="px-3 py-2.5 text-left cursor-pointer select-none hover:text-white" @click="trierPar('elimine_le')">Éliminé le <span class="text-[9px]">{{ iconeTri('elimine_le') }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(c, i) in candidatsAffiches" :key="c.symbole" class="border-b border-white/5 hover:bg-white/5" :class="c.elimine_le ? 'opacity-50' : ''">
            <td class="px-3 py-2.5 text-white">{{ i + 1 }}</td>
            <td class="px-3 py-2.5 font-semibold whitespace-nowrap" :class="c.elimine_le ? 'text-white' : 'text-white'">
              {{ c.symbole }}
              <span v-if="c.earnings_le" class="ml-1 text-[9px] px-1 py-0.5 rounded bg-orange-900/60 text-orange-300 align-middle"
                    :title="`Résultats trimestriels attendus le ${c.earnings_le} — risque de gap (avertissement, pas de veto)`">📊 {{ c.earnings_le.slice(5) }}</span>
            </td>
            <td class="px-3 py-2.5 text-center">
              <span v-if="c.univers === 'action'" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-blue-500/10 text-blue-300 border-blue-500/30"
                    title="Action US — veille en Observation (source Tiingo, référence QQQ)">Action US</span>
              <span v-else class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-amber-500/10 text-amber-300 border-amber-500/30"
                    title="Crypto — scanner Binance quotidien">Crypto</span>
            </td>
            <td class="px-3 py-2.5 text-center font-mono" :class="c.points >= 9 ? 'text-emerald-400' : c.points >= 7 ? 'text-blue-300' : 'text-white'">{{ c.points }}/10</td>
            <td class="px-3 py-2.5 text-center">
              <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
                :class="c.verdict === 'Alpha' ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30' : c.verdict === 'Rocket' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-gray-500/10 text-white border-gray-500/30'">
                {{ c.verdict === 'Alpha' ? 'ROCKET ALPHA' : c.verdict === 'Rocket' ? 'ROCKET' : 'éliminé' }}
              </span>
            </td>
            <td class="px-3 py-2.5 text-right font-mono text-white">{{ c.pivot.toFixed(4) }}</td>
            <td class="px-3 py-2.5 text-right font-mono text-red-400">{{ c.stop.toFixed(4) }}</td>
            <td class="px-3 py-2.5 text-center" :title="c.conviction_raison || ''">
              <template v-if="c.cassure && c.conviction_ia != null">
                <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
                  :class="c.conviction_ia >= 60 ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30' : c.conviction_ia >= 40 ? 'bg-amber-500/10 text-amber-400 border-amber-500/30' : 'bg-red-500/10 text-red-400 border-red-500/30'">
                  🚀 {{ c.conviction_ia }}/100
                </span>
              </template>
              <template v-else>{{ c.cassure ? '🚀' : '—' }}</template>
            </td>
            <td class="px-3 py-2.5 text-center" :title="c.news_justification || ''">
              <span v-if="c.news_verdict === 'POUR'" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">
                POUR {{ c.news_conviction }}/100 +1pt
              </span>
              <span v-else-if="c.news_verdict === 'CONTRE'" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-red-500/10 text-red-400 border-red-500/30">
                CONTRE {{ c.news_conviction }}/100
              </span>
              <span v-else-if="c.news_verdict === 'NEUTRE'" class="text-[10px] px-2 py-0.5 rounded-full border bg-white/5 text-white border-white/10">
                Neutre {{ c.news_conviction }}/100
              </span>
              <span v-else class="text-[10px] text-white">{{ c.news_verdict || 'à évaluer' }}</span>
            </td>
            <td class="px-3 py-2.5">
              <div class="flex gap-1 flex-wrap">
                <span v-for="(ok, critere) in c.detail" :key="critere" v-show="ok !== null"
                  class="text-[10px] px-1.5 py-0.5 rounded border"
                  :class="ok ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' : 'bg-white/5 text-white border-white/10'">
                  {{ LIBELLES[critere] ?? critere }}
                </span>
              </div>
            </td>
            <td class="px-3 py-2.5 text-xs text-white whitespace-nowrap">{{ c.maj_le ? formatDate(c.maj_le) : '—' }}</td>
            <td class="px-3 py-2.5 text-xs whitespace-nowrap" :class="c.elimine_le ? 'text-red-400' : 'text-white'">
              {{ c.elimine_le ? formatDate(c.elimine_le) : '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'
import { formatDate } from '@/composables/useSignalFormat'

interface Candidat {
  symbole: string; points: number; verdict: string
  univers?: string   // 'crypto' (défaut) | 'action'
  maj_le?: number    // détection du setup (dernier passage l'ayant retenu)
  elimine_le?: number | null  // sortie du suivi (null = actif)
  earnings_le?: string | null // date de résultats (action US) si annoncée
  pivot: number; stop: number; cassure: boolean
  detail: Record<string, boolean | null>
  news_verdict?: string; news_conviction?: number; news_justification?: string
  conviction_ia?: number; conviction_raison?: string
}

const LIBELLES: Record<string, string> = {
  sentiment: 'Sentiment', contexte: 'Contexte', news: 'News',
  tendance: 'Tendance', volatilite: 'Volatilité', interet: 'Intérêt',
  figure: 'Figure', gaps: 'Gaps', breakout: 'Cassure', liquidite: 'Liquidité',
}

withDefaults(defineProps<{ embarque?: boolean }>(), { embarque: false })

const candidats = ref<Candidat[]>([])
const chargement = ref(true)

// ── Filtres univers + tri par colonne ──
const filtreUnivers = ref<'' | 'crypto' | 'action'>('')
const filtresUnivers = [
  { val: '' as const, label: 'Tous' },
  { val: 'crypto' as const, label: 'Crypto' },
  { val: 'action' as const, label: 'Actions US' },
]
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}
function iconeTri(col: string): string {
  if (triColonne.value !== col) return '\u21c5'
  return triDir.value === 'asc' ? '\u2191' : '\u2193'
}

/// Valeur de tri d'un candidat pour une colonne (éliminés toujours en fin
/// de liste SAUF tri explicite sur « Éliminé le »).
function valeurTri(c: Candidat, col: string): string | number | boolean {
  switch (col) {
    case 'symbole': return c.symbole.toLowerCase()
    case 'univers': return c.univers ?? 'crypto'
    case 'points': return c.points
    case 'verdict': return c.verdict.toLowerCase()
    case 'pivot': return c.pivot
    case 'stop': return c.stop
    case 'cassure': return c.cassure
    case 'maj_le': return c.maj_le ?? 0
    case 'elimine_le': return c.elimine_le ?? 0
    default: return ''
  }
}

const candidatsAffiches = computed(() => {
  const liste = candidats.value.filter(c =>
    !filtreUnivers.value || (c.univers ?? 'crypto') === filtreUnivers.value
  )
  const col = triColonne.value
  const tries = col
    ? [...liste].sort((a, b) => {
        const va = valeurTri(a, col)
        const vb = valeurTri(b, col)
        const cmp = va < vb ? -1 : va > vb ? 1 : 0
        return triDir.value === 'asc' ? cmp : -cmp
      })
    : liste // ordre serveur : actifs d'abord, points décroissants
  return tries
})

async function charger() {
  chargement.value = true
  try {
    const res = await http.get<Candidat[]>('/api/rockets/candidats')
    candidats.value = res.data as Candidat[]
  } catch { candidats.value = [] }
  chargement.value = false
}
onMounted(charger)
</script>

<style scoped>
.filtre-btn {
  padding: 0.25rem 0.75rem;
  border-radius: 0.5rem;
  font-size: 0.75rem;
  color: #ffffff;
  background: rgba(255, 255, 255, 0.05);
  transition: all 0.15s ease;
}
.filtre-btn:hover { color: #fff; background: rgba(255, 255, 255, 0.1); }
.filtre-btn-actif {
  color: #fff;
  background: rgba(59, 130, 246, 0.25);
}
</style>
