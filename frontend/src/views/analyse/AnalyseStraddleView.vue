<template>
  <AnalysePageShell
    titre="⚡ Analyse Straddle"
    retour-label="Straddle"
    retour-route="/straddle"
    :synthese="bandeau"
  >
    <template #actions>
      <button
        class="text-[10px] px-2.5 py-1.5 rounded-lg bg-blue-600/30 text-blue-300 hover:bg-blue-600/50 border border-blue-500/30 transition-colors disabled:opacity-40"
        :disabled="rafraichissement"
        title="Force le recalcul de l'analyse LLM du jour (~2 min)"
        @click="raffraichirAnalyste"
      >{{ rafraichissement ? '⏳ Recalcul…' : '↻ Refaire analyser' }}</button>
    </template>

    <!-- ═══ RANGÉE 1 : dossier + avis, deux colonnes ═══ -->
    <div class="grid grid-cols-2 gap-4">
    <section class="rounded-xl border border-blue-500/30 bg-blue-950/20 p-4 flex flex-col gap-4">
      <h2 class="text-sm font-bold text-blue-300 uppercase tracking-wider">🎯 Le dossier de décision</h2>

      <!-- Les 4 chiffres, typographie XL -->
      <div class="grid grid-cols-4 gap-3">
        <div class="text-center">
          <div class="text-3xl font-bold" :class="(rapport?.somme_r ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ rapport?.somme_r != null ? `${rapport.somme_r >= 0 ? '+' : ''}${rapport.somme_r.toFixed(2)}R` : '—' }}
          </div>
          <div class="text-[10px] uppercase tracking-wide text-white mt-1">ΣR nets</div>
        </div>
        <div class="text-center">
          <div class="text-3xl font-bold text-white">{{ rapport?.n_passes ?? '—' }}<span class="text-base text-white">/30</span></div>
          <div class="text-[10px] uppercase tracking-wide text-white mt-1">passes closes</div>
        </div>
        <div class="text-center">
          <div class="text-3xl font-bold" :class="(stats.winPct ?? 0) >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ stats.winPct || '—' }}%</div>
          <div class="text-[10px] uppercase tracking-wide text-white mt-1">win rate</div>
        </div>
        <div class="text-center">
          <div class="text-3xl font-bold text-amber-300">{{ Math.max(30 - (rapport?.n_passes ?? 30), 0) }}</div>
          <div class="text-[10px] uppercase tracking-wide text-white mt-1">passes restantes avant décision</div>
        </div>
      </div>

      <!-- LA donnée qui décide : par source -->
      <table v-if="parSource.length" class="w-full text-sm border-collapse">
        <thead>
          <tr class="text-white border-b border-white/15">
            <th class="py-2 text-left text-xs uppercase tracking-wide">Source</th>
            <th class="py-2 text-right text-xs uppercase tracking-wide">Passes</th>
            <th class="py-2 text-right text-xs uppercase tracking-wide">ΣR nets</th>
            <th class="py-2 text-right text-xs uppercase tracking-wide">Win rate</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="src in parSource" :key="src.source" class="border-b border-white/5">
            <td class="py-2 text-white">{{ src.source }}</td>
            <td class="py-2 text-right text-white">{{ src.n }}</td>
            <td class="py-2 text-right text-lg font-bold" :class="src.r >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ src.r >= 0 ? '+' : '' }}{{ src.r.toFixed(2) }}R</td>
            <td class="py-2 text-right" :class="src.wr >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ src.wr }}%</td>
          </tr>
        </tbody>
      </table>

      <!-- La phrase de l'analyste -->
      <p v-if="analyseLlm?.synthese" class="text-sm text-white leading-relaxed font-medium border-l-2 border-blue-400/60 pl-3">
        « {{ analyseLlm.synthese }} »
      </p>
      <div v-else-if="enCalcul" class="text-xs text-white italic">⏳ Analyse du jour en calcul…</div>
    </section>

    <!-- ═══ ZONE 2 · L'AVIS DE L'ANALYSTE ═══ -->
    <section class="rounded-xl border border-purple-500/25 bg-purple-950/10 p-4 flex flex-col gap-3">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold text-purple-300 uppercase tracking-wider">🤖 L'avis de l'analyste</h2>
        <span v-if="analyseLlm?.confiance != null" class="text-xs text-white">confiance <span class="font-bold text-purple-300">{{ analyseLlm.confiance }}/100</span></span>
      </div>

      <div v-if="enCalcul" class="text-xs text-white italic px-1 py-3">
        ⏳ Analyse du jour en calcul (~2 min) — <button class="underline hover:text-white" @click="chargerAnalyste">réessayer</button>
      </div>
      <div v-else-if="!analyseLlm" class="text-xs text-white italic px-1 py-3">{{ messageAnalyste || 'Aucun avis — utilise ↻ pour lancer l\'analyse.' }}</div>

      <template v-else>
        <div v-if="pointsForts.length || pointsFaibles.length" class="grid grid-cols-2 gap-3">
          <div v-if="pointsForts.length" class="rounded-lg border border-emerald-500/20 bg-emerald-950/20 px-3 py-2.5">
            <div class="text-[9px] text-emerald-400 font-semibold uppercase tracking-widest mb-1.5">✅ Ce qui marche</div>
            <ul class="flex flex-col gap-1.5">
              <li v-for="(p, i) in pointsForts" :key="i" class="text-xs text-white leading-relaxed">· {{ p }}</li>
            </ul>
          </div>
          <div v-if="pointsFaibles.length" class="rounded-lg border border-red-500/20 bg-red-950/20 px-3 py-2.5">
            <div class="text-[9px] text-red-400 font-semibold uppercase tracking-widest mb-1.5">❌ Ce qui coince</div>
            <ul class="flex flex-col gap-1.5">
              <li v-for="(p, i) in pointsFaibles" :key="i" class="text-xs text-white leading-relaxed">· {{ p }}</li>
            </ul>
          </div>
        </div>

        <div v-if="recommandations.length">
          <h3 class="text-[10px] text-white font-semibold uppercase tracking-widest mb-2">
            Propositions pour la décision ({{ recommandations.length }}) — toi seul décides
          </h3>
          <div class="flex flex-col gap-2">
            <div
              v-for="(r, i) in recommandations" :key="i"
              class="rounded-lg border px-3 py-2.5 flex gap-3 items-start"
              :class="r.priorite === 'haute' ? 'border-amber-500/30 bg-amber-950/20' : 'border-white/10 bg-white/5'"
            >
              <span class="shrink-0 text-[9px] font-bold uppercase tracking-widest px-1.5 py-0.5 rounded-full mt-0.5"
                    :class="r.priorite === 'haute' ? 'bg-amber-500/20 text-amber-300' : 'bg-white/10 text-white'">{{ r.priorite }}</span>
              <div class="flex-1 min-w-0">
                <p class="text-xs text-white leading-relaxed">{{ r.description }}</p>
                <p v-if="r.impact_estime" class="text-[10px] text-white mt-1 italic">Impact estimé : {{ r.impact_estime }}</p>
              </div>
            </div>
          </div>
        </div>
      </template>
    </section>

    </div>

    <!-- ═══ RANGÉE 2 : détails + créneaux, deux colonnes ═══ -->
    <div class="grid grid-cols-2 gap-4">
    <section class="rounded-xl border border-white/10 bg-white/[0.03] p-4 flex flex-col gap-4">
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-sm font-bold text-white uppercase tracking-wider">📋 Détails de performance</span>
        <span class="ml-auto text-xs text-white">{{ stats.total }} passes · R moyen {{ stats.rMoyen }}R · loss rate {{ stats.tauxSL }}%</span>
      </div>
      <!-- Cartes 1 ligne par asset + verdicts en chips -->
      <div class="flex flex-col gap-3">
        <div class="grid grid-cols-3 gap-2">
          <div v-for="a in straddleStats.parAsset.value" :key="a.asset"
               class="kpi-card flex items-center gap-2 px-2.5 py-1.5 text-xs whitespace-nowrap">
            <span class="font-bold text-yellow-300">{{ a.asset }}</span>
            <span class="text-white">{{ a.total }}p</span>
            <span :class="a.winPct >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ a.winPct }}%</span>
            <span class="font-bold" :class="a.rMoyen >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ a.rMoyen >= 0 ? '+' : '' }}{{ a.rMoyen }}R</span>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <span class="text-xs font-semibold px-2 py-1 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">tp2 × {{ stats.tp2 }}</span>
          <span v-if="stats.tp1" class="text-xs font-semibold px-2 py-1 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">tp1 × {{ stats.tp1 }}</span>
          <span v-if="stats.tp3" class="text-xs font-semibold px-2 py-1 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">tp3 × {{ stats.tp3 }}</span>
          <span v-if="stats.ts" class="text-xs font-semibold px-2 py-1 rounded-full border bg-emerald-500/10 text-emerald-400 border-emerald-500/30">ts × {{ stats.ts }}</span>
          <span v-if="nbBe" class="text-xs font-semibold px-2 py-1 rounded-full border bg-white/5 text-white border-white/20">be × {{ nbBe }}</span>
          <span class="text-xs font-semibold px-2 py-1 rounded-full border bg-red-500/10 text-red-400 border-red-500/30">sl × {{ stats.sl }}</span>
          <span v-if="stats.expire" class="text-xs font-semibold px-2 py-1 rounded-full border bg-white/5 text-white border-white/20">expire × {{ stats.expire }}</span>
        </div>
      </div>
    </section>

    <!-- ═══ TIROIR · Créneaux armés (résumé visible replié) ═══ -->
    <section class="rounded-xl border border-white/10 bg-white/[0.03] p-4 flex flex-col gap-3">
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-sm font-bold text-white uppercase tracking-wider">🕐 Créneaux IA armés</span>
        <span v-if="creneauxArmés.length" class="ml-auto text-xs text-white truncate">
          {{ creneauxArmés.length }} armé(s) · {{ resumeCreneaux }}
        </span>
        <span v-else class="ml-auto text-xs text-white italic">aucun créneau armé</span>
      </div>
      <div class="flex flex-col gap-2">
        <div v-for="c in creneauxArmés" :key="c.asset + c.jour + c.heure"
             class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 flex items-center gap-3 flex-wrap text-xs">
          <span class="font-bold text-white">{{ c.asset }} · {{ JOURS[c.jour - 1] }} {{ c.heure }}h</span>
          <span class="font-mono text-white/80">vol ×{{ c.ratio.toFixed(2) }}</span>
          <span class="font-mono" :class="c.somme_r >= 0 ? 'text-emerald-300' : 'text-red-300'">
            {{ c.occurrences }}/{{ cible(c) }} tirages · Σ{{ c.somme_r >= 0 ? '+' : '' }}{{ c.somme_r.toFixed(2) }}R
          </span>
          <span v-if="c.verdict_test === 'valide'" class="text-[9px] font-bold px-1.5 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">✅ VALIDÉ — pilier</span>
          <span v-else-if="c.verdict_test === 'incertain'" class="text-[9px] px-1.5 py-0.5 rounded-full bg-amber-500/10 text-amber-300 border border-amber-500/30">⏳ prolongé</span>
          <span class="ml-auto text-white/70">prochain tirage : {{ prochainTirage(c) }}</span>
        </div>
        <RouterLink
          to="/straddle"
          class="self-start text-[10px] px-2.5 py-1.5 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition"
        >→ Agenda complet, file et armement sur la page Straddle</RouterLink>
      </div>
    </section>

    </div>

    <!-- ═══ PARAMÈTRES (pleine largeur, toujours déroulé) ═══ -->
    <section class="rounded-xl border border-white/10 bg-white/[0.03] p-4 flex flex-col gap-3">
      <div class="flex items-center gap-3">
        <span class="text-sm font-bold text-white uppercase tracking-wider">⚙️ Paramètres</span>
        <span class="ml-auto text-xs text-white">réglages du moteur straddle</span>
      </div>
      <div>
        <StraddleParamsPanel
          v-model="strParams"
          :has-resultats="false"
          :chargement-llm="false"
          :suggestion="null"
          @relancer="() => {}"
          @optimiser="() => {}"
          @params-saved="() => {}"
        />
      </div>
    </section>
  </AnalysePageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { Ref } from 'vue'
import { RouterLink } from 'vue-router'
import { http } from '@/services/http.client'
import { apiService } from '@/services/api.service'
import type { Signal } from '@/services/api.service'
import { useStraddleStats } from '@/composables/useStraddleStats'
import AnalysePageShell from '@/components/common/AnalysePageShell.vue'
import StraddleParamsPanel from '@/components/common/StraddleParamsPanel.vue'
import type { StraddleParams } from '@/components/common/StraddleParamsPanel.vue'

// ── Signaux (stats du tiroir détail) ──
const signaux = ref<Signal[]>([])
onMounted(async () => {
  try {
    const data = await apiService.getSignaux(500)
    signaux.value = data.filter(s => s.strategie?.toLowerCase() === 'straddle')
  } catch { signaux.value = [] }
})

const straddleStats = useStraddleStats(computed(() => signaux.value) as Ref<Signal[]>)
const stats = computed(() => straddleStats.stats.value)
const nbBe = computed(() => Math.max(stats.value.total - stats.value.tp1 - stats.value.tp2 - stats.value.tp3 - stats.value.ts - stats.value.sl - stats.value.expire, 0))

// ── Analyste des passes (cache du jour — instantané) ──
interface RecoAnalyste { priorite: string; description: string; impact_estime?: string }
interface ReponseAnalyste {
  n_passes?: number; calcule_le?: number; somme_r?: number; gagnantes?: number; en_calcul?: boolean; message?: string
  analyse: { synthese?: string; points_forts?: string[]; points_faibles?: string[]; recommandations?: RecoAnalyste[]; confiance?: number | null } | null
}
const rapport = ref<ReponseAnalyste | null>(null)
const rafraichissement = ref(false)

const analyseLlm = computed(() => rapport.value?.analyse ?? null)
const pointsForts = computed(() => analyseLlm.value?.points_forts ?? [])
const pointsFaibles = computed(() => analyseLlm.value?.points_faibles ?? [])
const recommandations = computed(() => analyseLlm.value?.recommandations ?? [])
const enCalcul = computed(() => !!rapport.value?.en_calcul && !analyseLlm.value)
const messageAnalyste = computed(() => (!enCalcul.value && !analyseLlm.value ? (rapport.value?.message ?? '') : ''))

/// Performance par source (chiffres moteur) — LA donnée de la décision §1.
const parSource = computed(() => {
  const ps = (rapport.value as any)?.kpis?.par_source ?? {}
  return Object.entries(ps)
    .map(([source, v]: [string, any]) => ({ source, n: v.n as number, r: v.r as number, wr: v.n > 0 ? Math.round(100 * v.g / v.n) : 0 }))
    .sort((a, b) => b.r - a.r)
})

const horodatageAnalyse = computed(() => {
  const le = rapport.value?.calcule_le
  if (!le) return '—'
  const d = new Date(le * 1000)
  return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}h${String(d.getMinutes()).padStart(2, '0')}`
})

const bandeau = computed(() => [
  { label: 'analyse du', valeur: horodatageAnalyse.value, classe: 'text-blue-300' },
])

async function chargerAnalyste() {
  try {
    const res = await http.get<ReponseAnalyste>('/api/straddle/analyste')
    rapport.value = res.data
  } catch { rapport.value = { n_passes: 0, analyse: null, message: 'Analyste injoignable' } }
}
async function raffraichirAnalyste() {
  rafraichissement.value = true
  try {
    const res = await http.post<ReponseAnalyste>('/api/straddle/analyste/rafraichir', null, { timeout: 240_000 })
    rapport.value = res.data
  } catch { /* le cache existant reste affiché */ }
  rafraichissement.value = false
}
onMounted(chargerAnalyste)

// ── Créneaux armés (§16-b) — résumé sur le tiroir, détail à l'ouverture ──
interface CreneauArme { asset: string; jour: number; heure: number; ratio: number; occurrences: number; somme_r: number; verdict_test: string | null }
const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']
const creneauxArmés = ref<CreneauArme[]>([])
const seuilTirages = ref(4)
function cible(c: CreneauArme): number {
  return c.verdict_test === 'incertain' ? seuilTirages.value + 2 : seuilTirages.value
}
function prochainTirage(c: CreneauArme): string {
  const d = new Date()
  const delta = (c.jour - (d.getDay() === 0 ? 7 : d.getDay()) + 7) % 7
  const t = new Date(d)
  t.setDate(d.getDate() + (delta === 0 && d.getHours() * 60 + d.getMinutes() >= c.heure * 60 ? 7 : delta))
  return `${JOURS[c.jour - 1]}. ${String(t.getDate()).padStart(2, '0')}/${String(t.getMonth() + 1).padStart(2, '0')} ${c.heure}h00`
}
const resumeCreneaux = computed(() => {
  if (!creneauxArmés.value.length) return ''
  return creneauxArmés.value.map(c => `${c.asset} ${JOURS[c.jour - 1]} ${c.heure}h (tirage ${prochainTirage(c)})`).join(' · ')
})
onMounted(async () => {
  try {
    const res = await http.get<{ slots: CreneauArme[]; seuils: { min: number } }>('/api/straddle/creneaux-ia')
    creneauxArmés.value = res.data.slots ?? []
    seuilTirages.value = res.data.seuils?.min ?? 4
  } catch { creneauxArmés.value = [] }
})

const strParams = ref<StraddleParams>({
  atr_periode: 14, seuil_atr: 1.5,
  tp_mult_1: 2.0, tp_mult_2: 3.0, tp_mult_3: 5.0,
  sl_mult: 0.5, trailing_atr: 1.5, vente_partielle: 1, pct_cloture_tp1: 0.33, pct_cloture_tp2: 0.33,
})
</script>

<style scoped>
.kpi-card     { @apply bg-white/5 rounded-lg p-3 border border-white/10; }
.section-title { @apply text-xs font-semibold text-white mb-2 uppercase tracking-wide; }
</style>
