<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🔭 Scanner Rockets</h1>
      <span class="text-gray-500 text-base hidden sm:inline">candidats VCP en attente de pivot</span>
      <button class="ml-auto btn-sm" @click="charger">🔄 Actualiser</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto glass-card">
      <div v-if="chargement && !candidats.length" class="text-center text-gray-500 py-10 text-sm">Chargement…</div>
      <div v-else-if="!candidats.length" class="text-center text-gray-500 py-10 text-sm">
        Aucun candidat — le scanner quotidien n'a rien retenu (seuil 5/10)
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-3 py-2.5 text-left">Symbole</th>
            <th class="px-3 py-2.5 text-center">Classement</th>
            <th class="px-3 py-2.5 text-center">Verdict</th>
            <th class="px-3 py-2.5 text-right">Pivot</th>
            <th class="px-3 py-2.5 text-right">Invalidation</th>
            <th class="px-3 py-2.5 text-center">Cassure</th>
            <th class="px-3 py-2.5 text-center">News (IA)</th>
            <th class="px-3 py-2.5 text-left">Critères</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in candidats" :key="c.symbole" class="border-b border-white/5 hover:bg-white/5">
            <td class="px-3 py-2.5 font-semibold text-white">{{ c.symbole }}</td>
            <td class="px-3 py-2.5 text-center font-mono" :class="c.points >= 9 ? 'text-emerald-400' : c.points >= 7 ? 'text-blue-300' : 'text-gray-400'">{{ c.points }}/10</td>
            <td class="px-3 py-2.5 text-center">
              <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
                :class="c.verdict === 'Alpha' ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30' : c.verdict === 'Rocket' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-gray-500/10 text-gray-400 border-gray-500/30'">
                {{ c.verdict === 'Alpha' ? 'ROCKET ALPHA' : c.verdict === 'Rocket' ? 'ROCKET' : 'éliminé' }}
              </span>
            </td>
            <td class="px-3 py-2.5 text-right font-mono text-white">{{ c.pivot.toFixed(4) }}</td>
            <td class="px-3 py-2.5 text-right font-mono text-red-400">{{ c.stop.toFixed(4) }}</td>
            <td class="px-3 py-2.5 text-center">{{ c.cassure ? '🚀' : '—' }}</td>
            <td class="px-3 py-2.5 text-center" :title="c.news_justification || ''">
              <span v-if="c.news_verdict === 'POUR'" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">
                POUR {{ c.news_conviction }}/100 +1pt
              </span>
              <span v-else-if="c.news_verdict === 'CONTRE'" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border bg-red-500/10 text-red-400 border-red-500/30">
                CONTRE {{ c.news_conviction }}/100
              </span>
              <span v-else-if="c.news_verdict === 'NEUTRE'" class="text-[10px] px-2 py-0.5 rounded-full border bg-white/5 text-gray-400 border-white/10">
                Neutre {{ c.news_conviction }}/100
              </span>
              <span v-else class="text-[10px] text-gray-600">{{ c.news_verdict || 'à évaluer' }}</span>
            </td>
            <td class="px-3 py-2.5">
              <div class="flex gap-1 flex-wrap">
                <span v-for="(ok, critere) in c.detail" :key="critere" v-show="ok !== null"
                  class="text-[10px] px-1.5 py-0.5 rounded border"
                  :class="ok ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' : 'bg-white/5 text-gray-500 border-white/10'">
                  {{ LIBELLES[critere] ?? critere }}
                </span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface Candidat {
  symbole: string; points: number; verdict: string
  pivot: number; stop: number; cassure: boolean
  detail: Record<string, boolean | null>
  news_verdict?: string; news_conviction?: number; news_justification?: string
}

const LIBELLES: Record<string, string> = {
  sentiment: 'Sentiment', contexte: 'Contexte', news: 'News',
  tendance: 'Tendance', volatilite: 'Volatilité', interet: 'Intérêt',
  figure: 'Figure', gaps: 'Gaps', breakout: 'Cassure', liquidite: 'Liquidité',
}

const candidats = ref<Candidat[]>([])
const chargement = ref(true)

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
