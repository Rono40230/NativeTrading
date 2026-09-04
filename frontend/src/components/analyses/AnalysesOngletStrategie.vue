<template>
  <!-- Onglet d'analyse d'une stratégie : performance $ par jour/semaine/mois,
       verdicts, assets et TF les plus propices — tout en $ réels (même base
       que le capital des cartes) et en R de la convention du moteur. -->
  <div v-if="a" class="flex flex-col gap-3">
    <!-- En-tête : identité + fenêtre + avertissement statistique -->
    <div class="glass-card p-3 flex flex-wrap items-center gap-2">
      <span class="text-lg leading-none">{{ icone }}</span>
      <span class="font-bold text-white">{{ nom }}</span>
      <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border" :class="badgeClasse">{{ a.etat }}</span>
      <span class="text-[10px] text-white">{{ a.nb_trades }} clôtures · {{ a.source === 'rejeu' ? 're-jeu paramétrique' : 'base vécue' }}</span>
      <span v-if="a.nb_trades" class="text-[10px] text-white">· {{ dateCourte(a.fenetre_debut) }} → {{ dateCourte(a.fenetre_fin) }}</span>
      <span
        v-if="a.nb_trades < 30"
        class="ml-auto text-[10px] px-2 py-0.5 rounded-full border border-amber-500/40 bg-amber-500/10 text-amber-300"
        :title="`Règle des 30 trades : sous cet effectif, aucune conclusion chiffrée n'est significative (anti-overfitting). ${a.nb_trades} clôture(s) analysée(s).`"
      >⚠️ {{ a.nb_trades }}/30 — non significatif</span>
    </div>

    <!-- Chips de tête : capital, R, WR, hier -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-2">
      <div class="glass-card px-3 py-2" title="Capital simulé (composé à chaque clôture — départ → actuel)">
        <p class="text-[9px] uppercase tracking-wider text-white">Capital</p>
        <p class="text-sm font-bold font-mono" :class="a.capital_actuel >= a.capital_depart ? 'text-emerald-400' : 'text-red-400'">
          {{ fmtDollars(a.capital_actuel) }} <span class="text-[10px] text-white">/ {{ fmtDollars(a.capital_depart) }}</span>
        </p>
      </div>
      <div class="glass-card px-3 py-2" title="Σ R de la convention du moteur (pondéré ventes partielles SMC, net straddle, réalisé base)">
        <p class="text-[9px] uppercase tracking-wider text-white">Σ R</p>
        <p class="text-sm font-bold font-mono" :class="a.r_total > 0 ? 'text-emerald-400' : a.r_total < 0 ? 'text-red-400' : 'text-white'">{{ fmtR(a.r_total) }}</p>
      </div>
      <div class="glass-card px-3 py-2" title="WR — part des clôtures gagnantes ($ > 0)">
        <p class="text-[9px] uppercase tracking-wider text-white">WR</p>
        <p class="text-sm font-bold font-mono text-white">{{ (a.taux_reussite * 100).toFixed(0) }} %</p>
      </div>
      <div class="glass-card px-3 py-2" title="Journée d&#39;hier (heure locale) — les données de la veille">
        <p class="text-[9px] uppercase tracking-wider text-white">Hier</p>
        <p v-if="a.hier" class="text-sm font-bold font-mono" :class="a.hier.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">
          {{ fmtDollars(a.hier.dollars) }} <span class="text-[10px] text-white">· {{ a.hier.trades }} trade(s)</span>
        </p>
        <p v-else class="text-sm text-white">—</p>
      </div>
    </div>

    <!-- Performance par période : jour / semaine / mois -->
    <div class="glass-card p-3">
      <div class="flex items-center gap-2 mb-2">
        <span class="text-xs font-semibold text-white">💵 Performance par période</span>
        <div class="ml-auto flex gap-1">
          <button
            v-for="p in PERIODES"
            :key="p.id"
            class="text-[10px] px-2 py-0.5 rounded font-semibold transition-colors"
            :class="periode === p.id ? 'bg-teal-500/30 text-white' : 'bg-white/5 text-white hover:bg-white/10'"
            @click="periode = p.id"
          >{{ p.label }}</button>
        </div>
      </div>
      <div v-if="periodeCourante.length" class="relative h-28">
        <div class="absolute inset-x-0 top-1/2 border-t border-white/15" />
        <div class="flex items-stretch h-full gap-1">
          <div
            v-for="p in periodeCourante"
            :key="p.cle"
            class="flex-1 flex flex-col items-center justify-center min-w-0"
            :title="`${p.label} — ${fmtDollars(p.dollars)} · ${fmtR(p.r)} · ${p.trades} clôture(s), ${p.gagnants} gagnante(s)`"
          >
            <div class="w-full flex-1 flex flex-col justify-end" v-show="p.dollars >= 0">
              <div class="w-full rounded-t bg-emerald-400/80" :style="{ height: `${hauteurPos(p)}%` }" />
            </div>
            <div class="w-full flex-1 flex flex-col justify-start" v-show="p.dollars < 0">
              <div class="w-full rounded-b bg-red-400/80" :style="{ height: `${hauteurPos(p)}%` }" />
            </div>
            <span class="text-[8px] text-white mt-0.5 truncate w-full text-center">{{ p.label }}</span>
          </div>
        </div>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">Aucune clôture sur cette granularité</p>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
      <!-- Verdicts : où vit l'edge -->
      <div class="glass-card p-3">
        <p class="text-xs font-semibold text-white mb-2">⚖️ Verdicts — où vit l'edge</p>
        <div v-if="a.verdicts.length" class="flex flex-col gap-1">
          <div class="flex items-center gap-2 text-[9px] uppercase tracking-wider text-white/60 px-0.5">
            <span class="w-16 shrink-0">Verdict</span>
            <span class="flex-1">Poids ($)</span>
            <span class="w-20 text-right shrink-0">$</span>
            <span class="w-14 text-right shrink-0">R</span>
            <span class="w-14 text-right shrink-0">Nb trades</span>
            <span class="w-12 text-right shrink-0">WR</span>
          </div>
          <div v-for="v in a.verdicts" :key="v.label" class="flex items-center gap-2 text-[11px]">
            <span class="w-16 shrink-0 font-mono font-bold" :style="{ color: couleurVerdict(v.label) }">{{ v.label }}</span>
            <div class="flex-1 h-4 bg-white/5 rounded overflow-hidden relative">
              <div class="absolute inset-y-0 left-1/2 w-px bg-white/15" />
              <div
                class="absolute inset-y-0.5 rounded"
                :class="v.dollars >= 0 ? 'bg-emerald-400/70 left-1/2' : 'bg-red-400/70 right-1/2'"
                :style="{ width: `${largeurCat(v)}%` }"
              />
            </div>
            <span class="w-20 text-right font-mono shrink-0" :class="v.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(v.dollars) }}</span>
            <span class="w-14 text-right text-white shrink-0">{{ fmtR(v.r) }}</span>
            <span class="w-14 text-right text-white shrink-0">{{ v.n }}</span>
            <span class="w-12 text-right text-white shrink-0">{{ (v.wr * 100).toFixed(0) }} %</span>
          </div>
        </div>
        <p v-else class="text-xs text-white py-4 text-center">Aucune clôture</p>
      </div>

      <!-- Assets les plus propices -->
      <div class="glass-card p-3">
        <p class="text-xs font-semibold text-white mb-2">🥇 Assets — contribution au capital</p>
        <div v-if="a.assets.length" class="flex flex-col gap-1">
          <div class="flex items-center gap-2 text-[9px] uppercase tracking-wider text-white/60 px-0.5">
            <span class="w-16 shrink-0">Asset</span>
            <span class="flex-1">Poids ($)</span>
            <span class="w-20 text-right shrink-0">$</span>
            <span class="w-14 text-right shrink-0">R</span>
            <span class="w-14 text-right shrink-0">Nb trades</span>
            <span class="w-12 text-right shrink-0">WR</span>
          </div>
          <div v-for="s in a.assets" :key="s.label" class="flex items-center gap-2 text-[11px]">
            <span class="w-16 shrink-0 font-mono font-bold text-white">{{ s.label }}</span>
            <div class="flex-1 h-4 bg-white/5 rounded overflow-hidden relative">
              <div class="absolute inset-y-0 left-1/2 w-px bg-white/15" />
              <div
                class="absolute inset-y-0.5 rounded"
                :class="s.dollars >= 0 ? 'bg-emerald-400/70 left-1/2' : 'bg-red-400/70 right-1/2'"
                :style="{ width: `${largeurCat(s)}%` }"
              />
            </div>
            <span class="w-20 text-right font-mono shrink-0" :class="s.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(s.dollars) }}</span>
            <span class="w-14 text-right text-white shrink-0">{{ fmtR(s.r) }}</span>
            <span class="w-14 text-right text-white shrink-0">{{ s.n }}</span>
            <span class="w-12 text-right text-white shrink-0">{{ (s.wr * 100).toFixed(0) }} %</span>
          </div>
        </div>
        <p v-else class="text-xs text-white py-4 text-center">Aucune clôture</p>
      </div>
    </div>

    <!-- Timeframes : contribution au capital, par asset — grille 2 colonnes -->
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2" title="Contribution de chaque TF de chaque asset au capital ($ réels composés)">⏱️ Timeframes — contribution au capital par asset</p>
      <div v-if="a.par_asset_tf.length" class="grid grid-cols-1 md:grid-cols-2 gap-2">
        <div
          v-for="pa in a.par_asset_tf"
          :key="pa.asset"
          class="rounded-lg border px-2.5 py-2 flex flex-col gap-1.5"
          :class="pa.dollars >= 0 ? 'border-emerald-500/25 bg-emerald-500/5' : 'border-red-500/25 bg-red-500/5'"
          :title="`${pa.asset} — ${fmtDollars(pa.dollars)} · ${pa.n} clôture(s)`"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="font-mono font-bold text-white text-xs">{{ pa.asset }}</span>
            <span class="font-mono text-[11px] font-bold" :class="pa.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(pa.dollars) }}</span>
          </div>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="t in pa.tfs"
              :key="pa.asset + t.label"
              class="text-[10px] px-1.5 py-0.5 rounded font-mono"
              :class="t.dollars >= 0 ? 'bg-emerald-500/15 text-emerald-300' : 'bg-red-500/15 text-red-300'"
              :title="`${t.label} — ${fmtDollars(t.dollars)} · ${fmtR(t.r)} · ${t.n} clôture(s) · WR ${(t.wr * 100).toFixed(0)} %`"
            >{{ t.label }} {{ fmtDollars(t.dollars) }}</span>
          </div>
          <span class="text-[9px] text-white">{{ pa.n }} clôture(s)</span>
        </div>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">Aucune clôture</p>
    </div>

    <!-- Évolution jour après jour : snapshots quotidiens + avis IA archivés -->
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2" title="Un snapshot par jour, écrit au premier calcul du rapport — l'avis IA du jour est archivé avec lui (survit aux redémarrages)">📈 Évolution jour après jour</p>
      <div v-if="historique.length" class="flex flex-col gap-2">
        <div class="relative h-16">
          <svg :viewBox="`0 0 100 32`" preserveAspectRatio="none" class="w-full h-full">
            <polyline
              :points="pointsHistorique"
              fill="none" stroke="#60a5fa" stroke-width="1.5"
              vector-effect="non-scaling-stroke" stroke-linejoin="round" stroke-linecap="round"
            />
          </svg>
        </div>
        <div class="overflow-x-auto">
          <table class="w-full text-[10px]">
            <thead>
              <tr class="text-white/60 uppercase tracking-wider">
                <th class="text-left pb-1 pr-2 font-semibold">Jour</th>
                <th class="text-right pb-1 pr-2 font-semibold">Capital</th>
                <th class="text-right pb-1 pr-2 font-semibold">Hier</th>
                <th class="text-right pb-1 pr-2 font-semibold">Σ R</th>
                <th class="text-right pb-1 font-semibold">Avis IA</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="snap in historique.slice(0, 14)" :key="snap.jour" class="border-t border-white/5">
                <td class="py-1 pr-2 text-white font-mono">{{ snap.jour.slice(5) }}</td>
                <td class="py-1 pr-2 text-right font-mono text-white">{{ fmtDollars(snap.capital_actuel) }}</td>
                <td class="py-1 pr-2 text-right font-mono" :class="(snap.hier_dollars ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ snap.hier_dollars === null ? '—' : fmtDollars(snap.hier_dollars) }}</td>
                <td class="py-1 pr-2 text-right font-mono text-white">{{ fmtR(snap.r_total) }}</td>
                <td class="py-1 text-right">
                  <span v-if="avisDuSnapshot(snap)" class="font-mono cursor-help"
                        :class="(avisDuSnapshot(snap)!.confiance ?? 0) >= 70 ? 'text-emerald-400' : (avisDuSnapshot(snap)!.confiance ?? 0) >= 40 ? 'text-amber-300' : 'text-white/60'"
                        :title="avisDuSnapshot(snap)!.etat">{{ avisDuSnapshot(snap)!.confiance }}/100</span>
                  <span v-else class="text-white/40">—</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <p v-else class="text-xs text-white py-3 text-center">Premier snapshot aujourd'hui — l'historique se remplit au fil des jours.</p>
    </div>

    <!-- Analyse IA : avis de l'analyste local (à la demande, cache du jour) -->
    <div class="glass-card p-3">
      <div class="flex items-center gap-2 mb-2 flex-wrap">
        <p class="text-xs font-semibold text-white">🤖 Analyse IA</p>
        <span
          v-if="ia"
          class="text-[10px] px-2 py-0.5 rounded-full border font-mono font-bold"
          :class="ia.confiance >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : ia.confiance >= 40 ? 'border-amber-500/40 bg-amber-500/10 text-amber-300' : 'border-white/20 bg-white/5 text-white'"
          title="Confiance que l'analyste accorde à sa propre analyse"
        >{{ ia.confiance }}/100</span>
        <span v-if="ia" class="text-[10px] text-white">· générée à {{ heure(ia.generee_le) }}{{ iaCache ? ' (cache du jour)' : '' }}</span>
        <button
          class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
          :class="iaEnCours ? 'bg-white/10 text-white' : 'bg-teal-500/20 text-teal-300 hover:bg-teal-500/30'"
          :disabled="iaEnCours"
          @click="genererIa()"
        >{{ iaEnCours ? '⏳ Analyse en cours… (~1 min)' : ia ? '↻ Régénérer' : '⚡ Générer l\u2019avis' }}</button>
      </div>

      <div v-if="ia" class="grid grid-cols-1 md:grid-cols-2 gap-2.5 items-start">
        <!-- État général : encadré distinct, la phrase de synthèse -->
        <div class="rounded-lg border border-teal-500/25 bg-teal-500/10 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-teal-300 mb-1">🧭 État général</p>
          <p class="text-[12px] text-white leading-relaxed">{{ ia.etat }}</p>
        </div>

        <!-- Une section par liste, titre + fond propres -->
        <div v-if="ia.points_forts.length" class="rounded-lg border border-emerald-500/25 bg-emerald-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-emerald-300 mb-1.5">✓ Ce qui marche</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in ia.points_forts" :key="i">{{ p }}</li>
          </ul>
        </div>
        <div v-if="ia.points_faibles.length" class="rounded-lg border border-red-500/25 bg-red-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-red-300 mb-1.5">✕ Ce qui coince</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in ia.points_faibles" :key="i">{{ p }}</li>
          </ul>
        </div>
        <div v-if="ia.corrections.length" class="rounded-lg border border-amber-500/25 bg-amber-500/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-amber-300 mb-1.5">→ À étudier (tu décides seul)</p>
          <ul class="text-[11px] text-white space-y-1 list-disc list-inside leading-snug">
            <li v-for="(p, i) in ia.corrections" :key="i">{{ p }}</li>
          </ul>
        </div>
        <p class="text-[9px] text-white">L'analyste lit et propose — les réglages ne bougent que sur ta décision (constitution du 24/08).</p>
      </div>
      <p v-else-if="iaErreur" class="text-xs text-red-300">{{ iaErreur }}</p>
      <p v-else class="text-xs text-white">L'analyste local (Ollama) lit les métriques ci-dessus et donne son avis — état, points forts/faibles, pistes concrètes. Généré à la demande, conservé pour la journée.</p>
    </div>
  </div>

  <div v-else class="glass-card p-6 text-center text-sm text-white">
    {{ chargement ? 'Calcul de l\u2019analyse…' : 'Analyse indisponible — réessayez dans un instant (le re-jeu démarre à la première demande).' }}
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  chargerAnalyse, chargerHistoriqueAnalyses, fmtDollars, fmtR, couleurVerdict, genererAnalyseIa,
  type AnalyseStrategie, type PeriodeAnalyse, type CategorieAnalyse, type AnalyseIa,
  type SnapshotAnalyse,
} from '@/composables/useAnalyses'

const props = defineProps<{ id: string }>()

const NOMS: Record<string, { nom: string; icone: string }> = {
  SMC: { nom: 'Stratégie SMC', icone: '📐' },
  straddle: { nom: 'Stratégie Straddle', icone: '⚡' },
  rockets: { nom: 'Stratégie Rockets', icone: '🚀' },
}

const a = ref<AnalyseStrategie | null>(null)
const chargement = ref(true)
const periode = ref<'jour' | 'semaine' | 'mois'>('jour')

const PERIODES = [
  { id: 'jour' as const, label: 'Jour' },
  { id: 'semaine' as const, label: 'Semaine' },
  { id: 'mois' as const, label: 'Mois' },
]

const nom = computed(() => NOMS[props.id]?.nom ?? props.id)
const icone = computed(() => NOMS[props.id]?.icone ?? '📊')
const badgeClasse = computed(() =>
  a.value?.etat === 'Officielle'
    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
    : a.value?.etat === 'Observation'
      ? 'bg-amber-500/10 text-amber-400 border-amber-500/30'
      : 'bg-gray-500/10 text-white border-gray-500/30',
)

const periodeCourante = computed<PeriodeAnalyse[]>(() => {
  if (!a.value) return []
  if (periode.value === 'semaine') return a.value.hebdomadaire
  if (periode.value === 'mois') return a.value.mensuel
  return a.value.journalier
})

/// Hauteur (%, demi-zone) d'une barre de période — maxi absolu = 100 %.
function hauteurPos(p: PeriodeAnalyse): number {
  const maxAbs = Math.max(...periodeCourante.value.map(x => Math.abs(x.dollars)), 1)
  return Math.max(3, (Math.abs(p.dollars) / maxAbs) * 100)
}

/// Largeur (% de la demi-zone) d'une barre de catégorie (verdict/asset).
function largeurCat(c: CategorieAnalyse): number {
  const maxAbs = Math.max(
    ...[...(a.value?.verdicts ?? []), ...(a.value?.assets ?? []), ...(a.value?.tfs ?? [])]
      .map(x => Math.abs(x.dollars)), 1,
  )
  return Math.max(2, (Math.abs(c.dollars) / maxAbs) * 50)
}

/// Date courte JJ/MM/AA depuis un epoch secondes.
function dateCourte(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit', year: '2-digit' })
}

/// Heure HH:MM depuis un epoch secondes.
function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

// ── Analyse IA (à la demande, cache du jour côté backend) ────────────────────
const ia = ref<AnalyseIa | null>(null)
const iaCache = ref(false)
const iaEnCours = ref(false)
const iaErreur = ref('')

async function genererIa() {
  iaEnCours.value = true
  iaErreur.value = ''
  const res = await genererAnalyseIa(props.id)
  iaEnCours.value = false
  if (res) {
    ia.value = res.analyse
    iaCache.value = res.en_cache
  } else {
    iaErreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
  }
}

watch(() => props.id, () => {
  ia.value = null
  iaErreur.value = ''
})

// ── Historique quotidien (§14) ────────────────────────────────────────────────
const historique = ref<SnapshotAnalyse[]>([])

/// Avis IA désérialisé d'un snapshot (null si absent/corrompu).
function avisDuSnapshot(snap: SnapshotAnalyse): AnalyseIa | null {
  if (!snap.avis_ia) return null
  try {
    return JSON.parse(snap.avis_ia) as AnalyseIa
  } catch {
    return null
  }
}

/// Courbe du capital par jour (chrono croissant pour la polyline).
const pointsHistorique = computed(() => {
  const serie = [...historique.value].reverse()
  if (serie.length < 2) return ''
  const valeurs = serie.map(s => s.capital_actuel)
  const min = Math.min(...valeurs)
  const max = Math.max(...valeurs)
  const amplitude = max - min || 1
  return serie
    .map((s, i) => `${((i / (serie.length - 1)) * 100).toFixed(2)},${(30 - ((s.capital_actuel - min) / amplitude) * 28).toFixed(2)}`)
    .join(' ')
})

async function charger() {
  chargement.value = true
  const [analyse, histo] = await Promise.all([
    chargerAnalyse(props.id),
    chargerHistoriqueAnalyses(props.id),
  ])
  a.value = analyse
  historique.value = histo
  chargement.value = false
}

watch(() => props.id, () => { void charger() }, { immediate: true })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
