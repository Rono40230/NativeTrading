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
      <div class="relative flex flex-col gap-1 text-[14px] leading-snug">
        <p class="text-white">Dernières 24h - <span class="font-bold text-teal-200">{{ journal.emis }} émis</span> · <span class="font-bold text-teal-200">{{ journal.clotures }} clôturés</span></p>
        <p v-for="s in journal.parStrategie" :key="s.id" class="flex items-baseline gap-1.5">
          <span class="w-4 text-center">{{ s.icone }}</span>
          <span class="text-white/85 flex-1">{{ s.nom }}</span>
          <span class="font-bold tabular-nums" :class="s.r >= 0 ? 'text-teal-200' : 'text-red-300'">
            {{ s.n === 0 ? '—' : (s.r >= 0 ? '+' : '−') + Math.abs(s.r).toFixed(1).replace('.', ',') + ' R' }}
          </span>
        </p>
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

  return { emis: emis.length, clotures: clots.length, parStrategie }
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
