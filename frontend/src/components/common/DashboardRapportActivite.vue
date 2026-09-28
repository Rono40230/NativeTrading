<template>
  <!-- Interrupteur Korry « Rapport d'activité » du pedestal COMMUNICATIONS &
       NAVIGATION (dessin owner 25/09) : le journal de bord du jour —
       émissions et clôtures des dernières 24 h, verdicts, et Σ R distance
       PAR STRATÉGIE (la voix stratégie). Toile de fond : feuilles de
       rapport (histogramme + courbe). Les anciens raccourcis vivent en
       onglets dans la page. -->
  <div
    class="rounded-lg border p-2 flex flex-col gap-1.5 min-h-[168px] shrink-0 cursor-pointer transition-all bg-white/[0.03] border-white/10 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] hover:bg-teal-500/15 hover:border-teal-400/50 hover:shadow-[0_0_12px_rgba(255,255,255,0.08)]"
    title="Ouvrir le rapport d'activité (toutes les stratégies)"
    @click="router.push('/analyses')"
  >
    <div class="flex items-center gap-1.5">
      <span class="text-[15px] leading-none">📊</span>
      <span class="text-[11px] font-extrabold uppercase tracking-[0.15em] text-white/75 truncate">Rapport d'activité</span>
    </div>

    <div class="relative rounded bg-black/30 px-1.5 py-1 min-h-0 flex-1 overflow-hidden">
      <FondTheme theme="rapport" />
      <div class="relative flex flex-col h-full">
        <p class="text-[9px] font-bold uppercase tracking-wider text-white/60 text-center mb-0.5">Activité des dernières 24h</p>
        <table class="w-full text-[10px] leading-snug">
          <thead>
            <tr class="text-white/50 uppercase tracking-wider border-b border-white/10">
              <th class="text-left font-bold">Stratégie</th>
              <th class="text-right font-bold px-1">Émis</th>
              <th class="text-right font-bold px-1">Clôturés</th>
              <th class="text-right font-bold">±R</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in journal.parStrategie" :key="s.id" class="border-b border-white/5">
              <td class="py-0.5 text-white font-semibold">{{ s.icone }} {{ s.nom }}</td>
              <td class="text-right px-1 text-white tabular-nums">{{ journal.emisPar(s.id) }}</td>
              <td class="text-right px-1 text-white tabular-nums">{{ s.n }}</td>
              <td class="text-right font-bold tabular-nums" :class="s.r >= 0 ? 'text-emerald-300' : 'text-red-300'">
                {{ s.n === 0 ? '—' : (s.r >= 0 ? '+' : '−') + Math.abs(s.r).toFixed(1).replace('.', ',') }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import FondTheme from './FondTheme.vue'

const router = useRouter()

/// Le journal de bord des dernières 24 h (les signaux vivants — verdict
/// null — ne comptent que côté émissions).
interface SignalJournal {
  strategie?: string; asset?: string
  cree_le?: number; ferme_le?: number | null
  verdict?: string | null
  r_distance?: number | null
}
const signaux = ref<SignalJournal[]>([])

const VINGT_QUATRE_H = 86400

/// L'ordre maison des stratégies (même lisibilité que partout).
const STRATEGIES = [
  { id: 'SMC', nom: 'SMC', icone: '📐' },
  { id: 'straddle', nom: 'Straddle', icone: '⚡' },
  { id: 'rockets', nom: 'Rockets', icone: '🚀' },
  { id: 'kdj_halftrend', nom: 'KDJ', icone: '📈' },
]

const journal = computed(() => {
  const maintenant = Date.now() / 1000
  const limite = maintenant - VINGT_QUATRE_H
  const emis = signaux.value.filter(s => (s.cree_le ?? 0) >= limite)
  const clots = signaux.value.filter(s => s.verdict != null && (s.ferme_le ?? 0) >= limite)

  /// Σ R distance PAR STRATÉGIE (décision owner 25/09 — remplace la somme
  /// globale) ; n = clôtures de la fenêtre, — si aucune.
  const parStrategie = STRATEGIES.map(({ id, nom, icone }) => {
    const cl = clots.filter(s => s.strategie === id)
    return { id, nom, icone, n: cl.length, r: cl.reduce((acc, s) => acc + (s.r_distance ?? 0), 0) }
  })

  return { emis: emis.length, clotures: clots.length, parStrategie, emisPar: (id: string) => emis.filter(s => s.strategie === id).length }
})

async function charger() {
  try {
    const r = await http.get('/api/signaux', { params: { limit: 150 } })
    signaux.value = r.data as SignalJournal[]
  } catch { signaux.value = [] }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  poll = setInterval(charger, 60_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>
