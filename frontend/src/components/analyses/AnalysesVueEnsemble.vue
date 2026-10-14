<template>
  <!-- Vue d'ensemble du rapport d'activité (owner 11/10) : LES QUATRE
       stratégies (liste dynamique), ligne TOTALES, barres de capital, et
       synthèse IA du portefeuille (prompt analyse_portefeuille). Les onglets
       par stratégie ont disparu : l'accès se fait par les cartes du
       dashboard (clic sur une ligne pour ouvrir le rapport détaillé). -->
  <div class="flex flex-col gap-3">
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2">🧭 Les stratégies</p>
      <div v-if="liste.length" class="overflow-x-auto">
        <table class="w-full text-[11px]">
          <thead>
            <tr class="text-white text-left">
              <th class="py-1 pr-2 font-semibold">Stratégie</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Clôtures analysées">Clôtures</th>
              <th class="py-1 pr-2 font-semibold text-right" title="WR — part des clôtures gagnantes ($ > 0)">WR</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Σ R de la convention du moteur">Σ R</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Capital simulé départ → actuel">Capital</th>
              <th class="py-1 font-semibold text-right" title="Journée d'hier (heure locale)">Hier</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="s in liste"
              :key="s.strategie"
              class="cursor-pointer border-t border-white/5 hover:bg-white/5 transition-colors"
              :title="`Ouvrir l'analyse ${s.strategie}`"
              @click="$emit('ouvrir', s.strategie)"
            >
              <td class="py-1.5 pr-2">
                <span class="mr-1">{{ icone(s.strategie) }}</span>
                <span class="font-semibold text-white">{{ s.strategie }}</span>
                <span class="ml-1.5 text-[9px] px-1.5 py-0.5 rounded-full border" :class="badgeClasse(s.etat)">{{ s.etat }}</span>
                <span class="ml-1 text-[9px] text-white">{{ s.source === 'rejeu' ? '· re-jeu' : '' }}</span>
              </td>
              <td class="py-1.5 pr-2 text-right text-white">{{ s.nb_trades }}</td>
              <td class="py-1.5 pr-2 text-right text-white">{{ (s.taux_reussite * 100).toFixed(0) }} %</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="s.r_distance_total > 0 ? 'text-emerald-400' : s.r_total < 0 ? 'text-red-400' : 'text-white'">{{ fmtR(s.r_distance_total) }}</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="s.capital_actuel >= s.capital_depart ? 'text-emerald-400' : 'text-red-400'">
                {{ fmtDollars(s.capital_actuel) }}
                <span class="text-white/60 text-[10px]">/ {{ fmtDollars(s.capital_depart) }}</span>
              </td>
              <td class="py-1.5 text-right font-mono" :class="!s.hier ? 'text-white/50' : s.hier.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ s.hier ? fmtDollars(s.hier.dollars) : '—' }}
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr class="border-t border-white/15 font-bold" @click="" style="cursor: default">
              <td class="py-1.5 pr-2 text-white">TOTALES</td>
              <td class="py-1.5 pr-2 text-right text-white">{{ totalClotures }}</td>
              <td class="py-1.5 pr-2 text-right text-white">{{ wrGlobal }} %</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="totalR > 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR(totalR) }}</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="totalDollars >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ fmtDollars(totalCapital) }}
                <span class="text-white/60 text-[10px]">/ {{ fmtDollars(totalDepart) }}</span>
              </td>
              <td class="py-1.5 text-right font-mono" :class="totalHier >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ fmtDollars(totalHier) }}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">Analyses indisponibles — le backend calcule à la première demande.</p>
    </div>

    <!-- Contributions cumulées : qui pèse combien sur le capital total -->
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2">💰 Capital par stratégie (départ → actuel)</p>
      <div v-if="liste.length" class="flex flex-col gap-1.5">
        <div v-for="s in liste" :key="s.strategie" class="flex items-center gap-2 text-[11px]">
          <span class="w-20 shrink-0 text-white truncate">{{ icone(s.strategie) }} {{ s.strategie }}</span>
          <div class="flex-1 h-4 bg-white/5 rounded overflow-hidden relative">
            <div class="absolute inset-y-0 left-1/2 w-px bg-white/15" />
            <div
              class="absolute inset-y-0.5 rounded"
              :class="s.capital_actuel - s.capital_depart >= 0 ? 'bg-emerald-400/70 left-1/2' : 'bg-red-400/70 right-1/2'"
              :style="{ width: `${largeur(s)}%` }"
            />
          </div>
          <span class="w-24 text-right font-mono shrink-0" :class="s.capital_actuel - s.capital_depart >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ fmtDollars(s.capital_actuel - s.capital_depart) }}
          </span>
        </div>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">—</p>
    </div>

    <!-- Synthèse IA du portefeuille (owner 11/10) -->
    <div class="glass-card p-3">
      <div class="flex items-center gap-2 flex-wrap">
        <p class="text-xs font-semibold text-white">🤖 Synthèse IA du portefeuille</p>
        <span v-if="synthese" class="text-[10px] font-mono px-1.5 py-0.5 rounded-full border font-bold"
              :class="synthese.confiance >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : 'border-white/20 bg-white/5 text-white'">{{ synthese.confiance }}/100</span>
        <span v-if="synthese && syntheseCache" class="text-[9px] text-white/40">cache du jour</span>
        <button class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
                :class="syntheseEnCours ? 'bg-white/10 text-white' : 'bg-teal-500/20 text-teal-300 hover:bg-teal-500/30'"
                :disabled="syntheseEnCours" @click="genererSynthese()">
          {{ syntheseEnCours ? '⏳ Analyse… (~1 min)' : synthese ? '↻ Régénérer' : '⚡ Générer' }}</button>
      </div>
      <div v-if="synthese" class="grid grid-cols-1 md:grid-cols-3 gap-2.5 items-start mt-2">
        <div class="rounded-lg border border-white/15 bg-white/5 p-2.5 md:col-span-3">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">🧭 État du portefeuille</p>
          <p class="text-xs text-white leading-relaxed">{{ synthese.etat }}</p>
        </div>
        <div v-if="synthese.points_forts.length" class="rounded-lg border border-emerald-500/25 bg-emerald-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-emerald-300 mb-1.5">✓ Ce qui porte</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in synthese.points_forts" :key="i">{{ p }}</li>
          </ul>
        </div>
        <div v-if="synthese.points_faibles.length" class="rounded-lg border border-red-500/25 bg-red-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-red-300 mb-1.5">✕ Ce qui coince</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in synthese.points_faibles" :key="i">{{ p }}</li>
          </ul>
        </div>
        <div v-if="synthese.recommandations.length" class="rounded-lg border border-amber-500/25 bg-amber-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-amber-300 mb-1.5">→ Où concentrer l'attention</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in synthese.recommandations" :key="i">{{ p }}</li>
          </ul>
        </div>
        <p class="text-[9px] text-white md:col-span-3">L'analyste lit les métriques consolidées des 4 stratégies — il propose, le propriétaire décide (constitution 24/08).</p>
      </div>
      <p v-else-if="syntheseErreur" class="text-xs text-red-300 mt-2">{{ syntheseErreur }}</p>
      <p v-else class="text-[11px] text-white/60 mt-2">L'analyste local (Ollama) lit les 4 stratégies et dit qui porte, qui coince, où concentrer l'attention — généré à la demande, conservé pour la journée.</p>
    </div>

    <!-- 🧾 Autopsie de la semaine (owner 13/10 — KTA #10/#26) : les clôtures
         des 7 derniers jours décrites factuellement, descriptif jamais
         prédictif. Compteurs servis par l'app, synthèse par le LLM. -->
    <div class="glass-card p-3">
      <div class="flex items-center gap-2 flex-wrap">
        <p class="text-xs font-semibold text-white">🧾 Autopsie de la semaine</p>
        <span v-if="autopsie?.confiance" class="text-[10px] font-mono px-1.5 py-0.5 rounded-full border font-bold"
              :class="autopsie.confiance >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : 'border-white/20 bg-white/5 text-white'">{{ autopsie.confiance }}/100</span>
        <span v-if="autopsieCache" class="text-[9px] text-white/40">cache du jour</span>
        <button class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
                :class="autopsieEnCours ? 'bg-white/10 text-white' : 'bg-violet-500/20 text-violet-300 hover:bg-violet-500/30'"
                :disabled="autopsieEnCours" @click="genererAutopsie()">
          {{ autopsieEnCours ? '⏳ Analyse… (~1 min)' : autopsie ? '↻ Régénérer' : '⚡ Générer' }}</button>
      </div>
      <div v-if="autopsie" class="mt-2 flex flex-col gap-2">
        <div class="rounded-lg border border-white/15 bg-white/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">🗓 La semaine en clôtures</p>
          <p class="text-xs text-white leading-relaxed">{{ autopsie.etat }}</p>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-2.5 items-start">
          <div v-if="autopsie.gagnants.length" class="rounded-lg border border-emerald-500/25 bg-emerald-500/5 p-2.5">
            <p class="text-[10px] font-bold uppercase tracking-wider text-emerald-300 mb-1.5">✓ Côté gagnants</p>
            <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug"><li v-for="(g, i) in autopsie.gagnants" :key="i">{{ g }}</li></ul>
          </div>
          <div v-if="autopsie.perdants.length" class="rounded-lg border border-red-500/25 bg-red-500/5 p-2.5">
            <p class="text-[10px] font-bold uppercase tracking-wider text-red-300 mb-1.5">✕ Côté perdants</p>
            <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug"><li v-for="(x, i) in autopsie.perdants" :key="i">{{ x }}</li></ul>
          </div>
          <div v-if="autopsie.remarques.length" class="rounded-lg border border-amber-500/25 bg-amber-500/5 p-2.5">
            <p class="text-[10px] font-bold uppercase tracking-wider text-amber-300 mb-1.5">→ Remarques</p>
            <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug"><li v-for="(r, i) in autopsie.remarques" :key="i">{{ r }}</li></ul>
          </div>
        </div>
        <details class="text-[10px] text-white/60">
          <summary class="cursor-pointer select-none hover:text-white">Compteurs factuels de la semaine (calcul app)</summary>
          <pre class="mt-1 p-2 rounded bg-black/30 text-[10px] whitespace-pre-wrap">{{ autopsie.compteurs }}</pre>
        </details>
        <p class="text-[9px] text-white">Descriptif — jamais prédictif (leçon conviction). Effectif hebdo faible : l'analyste peut dire « semaine non concluante ».</p>
      </div>
      <p v-else class="text-[11px] text-white/60 mt-2">Les clôtures des 7 derniers jours, toutes stratégies : ce qui a distingué gagnants et perdants — généré à la demande, conservé pour la journée.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { chargerAnalyses, fmtDollars, fmtR, type ResumeStrategie } from '@/composables/useAnalyses'
import { http } from '@/services/http.client'

defineEmits<{ ouvrir: [strategie: string] }>()

const liste = ref<ResumeStrategie[]>([])

const ICONES: Record<string, string> = { SMC: '📐', straddle: '⚡', rockets: '🚀', kdj_halftrend: '📈' }
function icone(id: string): string {
  return ICONES[id] ?? '📊'
}

function badgeClasse(etat: string): string {
  if (etat === 'Officielle') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (etat === 'Observation') return 'bg-amber-500/10 text-amber-400 border-amber-500/30'
  return 'bg-gray-500/10 text-white border-gray-500/30'
}

// ── Totaux (owner 11/10) ────────────────────────────────────────────────────
const totalClotures = computed(() => liste.value.reduce((n, s) => n + s.nb_trades, 0))
const totalR = computed(() => liste.value.reduce((r, s) => r + s.r_distance_total, 0))
const totalCapital = computed(() => liste.value.reduce((c, s) => c + s.capital_actuel, 0))
const totalDepart = computed(() => liste.value.reduce((c, s) => c + s.capital_depart, 0))
const totalDollars = computed(() => totalCapital.value - totalDepart.value)
const totalHier = computed(() => liste.value.reduce((h, s) => h + (s.hier?.dollars ?? 0), 0))
const wrGlobal = computed(() =>
  totalClotures.value > 0
    ? Math.round(liste.value.reduce((n, s) => n + s.taux_reussite * s.nb_trades, 0) / totalClotures.value * 100)
    : 0)

// ── Synthèse IA du portefeuille (owner 11/10) ──────────────────────────────
interface SyntheseIa { etat: string; points_forts: string[]; points_faibles: string[]; recommandations: string[]; confiance: number }
const synthese = ref<SyntheseIa | null>(null)
const syntheseEnCours = ref(false)
const syntheseErreur = ref('')
const syntheseCache = ref(false)

async function genererSynthese() {
  syntheseEnCours.value = true
  syntheseErreur.value = ''
  try {
    const res = await http.post<{ en_cache: boolean; analyse: SyntheseIa }>('/api/analyses/portefeuille/ia', null, { timeout: 180_000 })
    synthese.value = res.data.analyse
    syntheseCache.value = res.data.en_cache
  } catch {
    syntheseErreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
  }
  syntheseEnCours.value = false
}

// ── 🧾 Autopsie de la semaine (owner 13/10) ─────────────────────────────────
interface AutopsieIa {
  etat: string
  gagnants: string[]
  perdants: string[]
  remarques: string[]
  confiance: number
  compteurs: string
}
const autopsie = ref<AutopsieIa | null>(null)
const autopsieEnCours = ref(false)
const autopsieCache = ref(false)

async function genererAutopsie() {
  autopsieEnCours.value = true
  try {
    const res = await http.post<{ en_cache: boolean; autopsie: AutopsieIa }>('/api/analyses/autopsie', null, { timeout: 180_000 })
    autopsie.value = res.data.autopsie
    autopsieCache.value = res.data.en_cache
  } catch {
    autopsie.value = null
  }
  autopsieEnCours.value = false
}

/// Largeur (% de la demi-zone) de la barre de variation de capital.
function largeur(s: ResumeStrategie): number {
  const variations = liste.value.map(x => Math.abs(x.capital_actuel - x.capital_depart))
  const maxAbs = Math.max(...variations, 1)
  return Math.max(2, (Math.abs(s.capital_actuel - s.capital_depart) / maxAbs) * 50)
}

onMounted(async () => {
  liste.value = await chargerAnalyses()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
