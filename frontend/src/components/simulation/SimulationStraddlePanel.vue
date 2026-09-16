<template>
  <div class="flex flex-col gap-4">
    <!-- Limite méthodologique -->
    <p class="text-[11px] text-white/70 border-l-2 border-amber-400/50 pl-3">
      La simulation rejoue les <b>passes réellement prises</b> avec un autre pilotage de sortie.
      Les niveaux (ATR, TP multiples) restent ceux historiques des passes — les rejouer
      exigera le chantier backtesteur. Toute conclusion sous 30 passes n'est pas significative.
    </p>

    <!-- ═══ Rangée 1 : vécu | paramètres ═══ -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <section class="glass-card p-4 flex flex-col gap-3">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">1 · Résultats en direct — vécu officiel</h2>
        <div v-if="vecu" class="grid grid-cols-3 gap-2">
          <div class="kpi"><p class="kpi-label">Capital</p>
            <p class="kpi-valeur" :class="rendement >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(vecu.capital_actuel) }}</p>
            <p class="kpi-sous">{{ fmtPct(rendement) }}</p></div>
          <div class="kpi"><p class="kpi-label">Σ R distance</p>
            <p class="kpi-valeur" :class="vecu.r_total >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR2(vecu.r_total) }}</p>
            <p class="kpi-sous">{{ vecu.nb_trades }} passes</p></div>
          <div class="kpi"><p class="kpi-label">WR ($ &gt; 0)</p>
            <p class="kpi-valeur text-white">{{ (vecu.taux_reussite * 100).toFixed(0) }} %</p>
            <p class="kpi-sous">pire creux {{ fmtDollars(creuxVecu) }}</p></div>
        </div>
        <p v-else class="text-xs text-white/60 py-4 text-center">Chargement du vécu…</p>
      </section>

      <section class="glass-card p-4 flex flex-col gap-3">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">2 · Paramètres (virtuels)</h2>
        <div class="grid grid-cols-2 gap-2 text-xs">
          <label class="flex flex-col gap-1">Trailing après TP1 (× ATR)
            <input v-model.number="params.trailing_atr" type="number" step="0.1" min="0.1" max="5" class="champ" /></label>
          <label class="flex flex-col gap-1">Time-stop (minutes)
            <input v-model.number="params.time_stop_min" type="number" step="5" min="5" max="240" class="champ" /></label>
        </div>
        <div class="flex gap-2 flex-wrap">
          <button class="btn-action bg-teal-500/20 text-teal-300 hover:bg-teal-500/30 disabled:opacity-40"
                  :disabled="enCours" @click="lancer">
            {{ enCours ? '⏳ Re-jeu en cours…' : '▶ Lancer la simulation' }}</button>
          <button class="btn-action bg-emerald-600/20 text-emerald-300 hover:bg-emerald-600/30 disabled:opacity-40"
                  :disabled="!modifie" :title="modifie ? 'Écrit le trailing dans les réglages moteur réels (futurs signaux)' : 'Aucune modification'"
                  @click="appliquer">✅ Appliquer ces réglages</button>
        </div>
        <p v-if="messageApplique" class="text-[11px] text-emerald-300">{{ messageApplique }}</p>
      </section>
    </div>

    <!-- ═══ 3 — Comparatif ═══ -->
    <section v-if="sim" class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2 flex-wrap">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">3 · Résultats après re-jeu</h2>
        <span class="text-[10px] text-white/60">calculé en {{ (sim.duree_ms / 1000).toFixed(1) }} s</span>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
              <th class="text-left py-1.5 pr-3">Métrique</th>
              <th class="text-right py-1.5 px-2">Vécu officiel</th>
              <th class="text-right py-1.5 px-2">Simulation</th>
              <th class="text-right py-1.5 pl-2">Écart</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in comparatif" :key="l.label" class="border-b border-white/5">
              <td class="py-1.5 pr-3 text-white">{{ l.label }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white/80">{{ l.vecu }}</td>
              <td class="py-1.5 px-2 text-right font-mono font-bold" :class="l.classe">{{ l.sim }}</td>
              <td class="py-1.5 pl-2 text-right font-mono" :class="l.classe">{{ l.ecart }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div>
        <p class="text-[10px] uppercase tracking-wider text-white/60 mb-1">Verdicts de la simulation</p>
        <div class="flex flex-wrap gap-2">
          <span v-for="v in sim.verdicts" :key="v.label"
                class="text-[11px] px-2 py-1 rounded-full border font-mono"
                :class="v.n < 30 ? 'border-white/10 bg-white/5 text-white/60' : 'border-white/20 bg-white/10 text-white'"
                :title="v.n < 30 ? `${v.n} passes — non significatif (règle des 30)` : `${v.n} passes`">
            {{ v.label }} × {{ v.n }} <span :class="v.r >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR2(v.r) }}</span>
          </span>
        </div>
      </div>
    </section>

    <!-- ═══ 4 — Bibliothèque ═══ -->
    <section class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">4 · Bibliothèque des essais</h2>
        <span class="ml-auto text-[10px] text-white/60">clic = recharge les paramètres · 🗑 = supprime</span>
      </div>
      <div v-if="essais.length" class="flex flex-col gap-1">
        <div v-for="e in essais" :key="e.id"
             class="flex items-center gap-3 text-xs rounded-lg px-3 py-2 bg-white/5 border border-white/10 hover:border-teal-400/40 cursor-pointer"
             @click="rechargerParams(e.params)">
          <span class="text-white/60 font-mono shrink-0">{{ dateCourte(e.cree_le) }}</span>
          <span class="text-white truncate">trailing {{ e.params.trailing_atr }} × ATR · time-stop {{ e.params.time_stop_min }} min</span>
          <span class="ml-auto font-mono shrink-0" :class="(e.resultat?.capital_actuel ?? 0) >= (e.resultat?.capital_depart ?? 1) ? 'text-emerald-400' : 'text-red-400'">
            {{ fmtDollars(e.resultat?.capital_actuel ?? 0) }} · {{ fmtR2(e.resultat?.r_total_pondere ?? 0) }}
          </span>
          <button class="text-white/50 hover:text-red-400 shrink-0" title="Supprimer l'essai"
                  @click.stop="supprimerEssai(e.id)">🗑</button>
        </div>
      </div>
      <p v-else class="text-xs text-white/60 py-2 text-center">Aucun essai encore — lance une simulation, elle sera conservée ici.</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'
import { chargerAnalyse, type AnalyseStrategie } from '@/composables/useAnalyses'

interface ParamsStraddle { trailing_atr: number; time_stop_min: number }
interface VerdictSim { label: string; n: number; r: number }
interface ResultatSim {
  nb_trades: number; taux_reussite: number; r_total: number; r_total_pondere: number
  capital_depart: number; capital_actuel: number; capital_minimum: number
  verdicts: VerdictSim[]; duree_ms: number
}
interface Essai { id: string; params: ParamsStraddle; resultat: ResultatSim; cree_le: number }

const vecu = ref<AnalyseStrategie | null>(null)
const creuxVecu = ref(0)
const params = ref<ParamsStraddle>({ trailing_atr: 1.5, time_stop_min: 60 })
const paramsActuels = ref<ParamsStraddle>({ trailing_atr: 1.5, time_stop_min: 60 })
const sim = ref<ResultatSim | null>(null)
const essais = ref<Essai[]>([])
const enCours = ref(false)
const messageApplique = ref('')

const rendement = computed(() =>
  vecu.value && vecu.value.capital_depart > 0 ? vecu.value.capital_actuel / vecu.value.capital_depart - 1 : 0)

const modifie = computed(() =>
  params.value.trailing_atr !== paramsActuels.value.trailing_atr
  || params.value.time_stop_min !== paramsActuels.value.time_stop_min)

async function chargerVecu() {
  vecu.value = await chargerAnalyse('straddle')
  try {
    const c = await http.get<{ capital_depart: number; points: { capital_apres: number }[] }>('/api/strategies/straddle/capital')
    creuxVecu.value = c.data.points.reduce((m, p) => Math.min(m, p.capital_apres), c.data.capital_depart)
  } catch { creuxVecu.value = vecu.value?.capital_depart ?? 0 }
}

interface StraddleParams { [cle: string]: number | boolean }

async function chargerReglages() {
  try {
    const p = await http.get<StraddleParams>('/api/straddle/params')
    const tr = Number(p.data?.trailing_atr ?? 0.9)
    params.value = { trailing_atr: tr, time_stop_min: 60 }
    paramsActuels.value = { ...params.value }
  } catch { /* défauts conservés */ }
}

async function lancer() {
  enCours.value = true
  messageApplique.value = ''
  try {
    const r = await http.post<{ resultat: ResultatSim }>('/api/strategies/straddle/simulation', params.value, { timeout: 150_000 })
    sim.value = r.data.resultat
    await chargerEssais()
  } catch { sim.value = null }
  enCours.value = false
}

async function chargerEssais() {
  try {
    const r = await http.get<{ essais: Essai[] }>('/api/strategies/straddle/simulation/essais')
    essais.value = r.data.essais ?? []
  } catch { essais.value = [] }
}

async function supprimerEssai(id: string) {
  try { await http.delete(`/api/strategies/straddle/simulation/essais/${id}`) } catch { /* liste inchangée */ }
  await chargerEssais()
}

function rechargerParams(p: ParamsStraddle) {
  params.value = { ...p }
  window.scrollTo({ top: 0, behavior: 'smooth' })
}

async function appliquer() {
  const lignes = [`  trailing ATR : ${paramsActuels.value.trailing_atr} → ${params.value.trailing_atr}`]
  if (!window.confirm(`Écrire le trailing dans les réglages moteur RÉELS ?\n\n${lignes.join('\n')}\n\nEffet sur les FUTURES passes (le time-stop canonique 60 n'est pas réglable ici).`)) return
  try {
    // PUT complet exigé par l'API : on lit les réglages réels et ne
    // remplace que le trailing.
    const actuels = await http.get<StraddleParams>('/api/straddle/params')
    await http.put('/api/straddle/params', { ...actuels.data, trailing_atr: params.value.trailing_atr })
    paramsActuels.value = { ...params.value }
    messageApplique.value = '✓ Trailing appliqué aux réglages moteur — effet sur les futures passes.'
  } catch { /* inchangé */ }
}

const comparatif = computed(() => {
  const v = vecu.value
  const s = sim.value
  if (!v || !s) return []
  const simPct = s.capital_depart > 0 ? s.capital_actuel / s.capital_depart - 1 : 0
  return [
    {
      label: 'Capital final', vecu: fmtDollars(v.capital_actuel), sim: fmtDollars(s.capital_actuel),
      ecart: fmtPct(simPct - rendement.value), classe: simPct >= rendement.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Rendement', vecu: fmtPct(rendement.value), sim: fmtPct(simPct),
      ecart: '', classe: simPct >= rendement.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Pire creux', vecu: fmtDollars(creuxVecu.value), sim: fmtDollars(s.capital_minimum),
      ecart: '', classe: s.capital_minimum >= creuxVecu.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Passes', vecu: String(v.nb_trades), sim: String(s.nb_trades),
      ecart: String(s.nb_trades - v.nb_trades), classe: 'text-white',
    },
    {
      label: 'WR', vecu: `${(v.taux_reussite * 100).toFixed(0)} %`, sim: `${(s.taux_reussite * 100).toFixed(0)} %`,
      ecart: `${((s.taux_reussite - v.taux_reussite) * 100).toFixed(0)} pts`, classe: s.taux_reussite >= v.taux_reussite ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Σ R net (composé)', vecu: fmtR2(v.r_total), sim: fmtR2(s.r_total_pondere),
      ecart: '', classe: s.r_total_pondere >= 0 ? 'text-emerald-400' : 'text-red-400',
    },
  ]
})

function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}
function fmtPct(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v * 100).toFixed(1)} %`
}
function fmtR2(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2)} R`
}
function dateCourte(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
}

onMounted(async () => {
  await Promise.all([chargerVecu(), chargerReglages(), chargerEssais()])
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.kpi        { @apply bg-white/5 rounded-lg p-2.5 border border-white/10 text-center; }
.kpi-label  { @apply text-[9px] uppercase tracking-wider text-white/60; }
.kpi-valeur { @apply text-lg font-bold font-mono mt-0.5; }
.kpi-sous   { @apply text-[10px] text-white/50 mt-0.5; }
.champ      { @apply bg-white/10 border border-white/15 rounded-lg px-2 py-1.5 text-white font-mono focus:border-teal-400/50 outline-none; }
.btn-action { @apply text-xs px-3 py-1.5 rounded-lg font-semibold border border-white/10 transition-colors; }
</style>
