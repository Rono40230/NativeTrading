<template>
  <div class="glass-card p-3 pt-2">
    <!-- En-tête -->
    <div class="flex items-center justify-between mb-2 border-b border-white/10 pb-1">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">🌡️ Sentiment de Marché</p>
      <div v-if="chargement" class="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
    </div>

    <p v-if="erreur" class="text-xs text-red-400">Données indisponibles</p>

    <template v-if="data">
      <!-- ══ BANDEAU SENTIMENT : jauges cliquables (ⓘ = définition + lecture) ══ -->
      <div v-if="data.bandeau" class="flex flex-col gap-2.5 pb-2.5 border-b-2 border-white/10">

        <!-- Peur & Appêt crypto (alternative.me, 0-100) -->
        <div
          v-if="data.bandeau.fng"
          class="relative cursor-help"
          @click.stop="survole = survole === 'fng' ? null : 'fng'"
        >
          <div class="flex items-baseline justify-between">
            <span class="text-[10px] uppercase tracking-wider text-white/80">Crypto — Peur &amp; Appêt <span class="text-white/40">ⓘ</span></span>
            <span class="text-xs font-bold tabular-nums" :class="classeFng(data.bandeau.fng.valeur)">
              {{ data.bandeau.fng.valeur }} · {{ traduireFng(data.bandeau.fng.classe) }}
              <span class="text-[9px] font-normal text-white/60">{{ flecheDelta(data.bandeau.fng.delta_veille) }}</span>
            </span>
          </div>
          <div class="mt-1 h-2 rounded-full overflow-hidden flex">
            <div class="h-full" style="width: 25%; background: #ef4444" />
            <div class="h-full" style="width: 20%; background: #f97316" />
            <div class="h-full" style="width: 10%; background: #eab308" />
            <div class="h-full" style="width: 20%; background: #84cc16" />
            <div class="h-full" style="width: 25%; background: #22c55e" />
          </div>
          <div class="relative h-0">
            <span
              class="absolute -top-[7px] w-[3px] h-[16px] rounded-full bg-white shadow"
              :style="{ left: `${Math.min(100, Math.max(0, data.bandeau.fng.valeur))}%`, transform: 'translateX(-50%)' }"
            />
          </div>
          <Transition name="fade">
            <div v-if="survole === 'fng'" class="tooltip-sentiment" @click.stop>
              <p class="text-[10px] font-bold text-white mb-1">Crypto — Peur &amp; Appêt</p>
              <p class="text-[10px] text-white/80 leading-snug">Indice 0-100 agrégeant volatilité, momentum, réseaux sociaux et recherches (alternative.me). 0 = peur extrême, 100 = appêt extrême.</p>
              <p class="text-[10px] mt-1.5 font-medium leading-snug" :class="tonAnalyse(data.bandeau.fng.valeur <= 24 || data.bandeau.fng.valeur > 75)">
                {{ analyseFng(data.bandeau.fng.valeur, data.bandeau.fng.delta_veille) }}
              </p>
            </div>
          </Transition>
        </div>

        <!-- Actions US : VIX en jauge inverse -->
        <div
          v-if="data.vix != null"
          class="relative cursor-help"
          @click.stop="survole = survole === 'vix' ? null : 'vix'"
        >
          <div class="flex items-baseline justify-between">
            <span class="text-[10px] uppercase tracking-wider text-white/80">Actions US — VIX <span class="text-white/40">ⓘ</span></span>
            <span class="text-xs font-bold tabular-nums" :class="classeVix(data.vix)">
              {{ data.vix.toFixed(1) }} · {{ verdictVix(data.vix) }}
            </span>
          </div>
          <div class="mt-1 h-2 rounded-full overflow-hidden flex">
            <div class="h-full" style="width: 37.5%; background: #22c55e" />
            <div class="h-full" style="width: 12.5%; background: #eab308" />
            <div class="h-full" style="width: 25%; background: #f97316" />
            <div class="h-full" style="width: 25%; background: #ef4444" />
          </div>
          <div class="relative h-0">
            <span
              class="absolute -top-[7px] w-[3px] h-[16px] rounded-full bg-white shadow"
              :style="{ left: `${Math.min(100, Math.max(0, (data.vix / 40) * 100))}%`, transform: 'translateX(-50%)' }"
            />
          </div>
          <Transition name="fade">
            <div v-if="survole === 'vix'" class="tooltip-sentiment" @click.stop>
              <p class="text-[10px] font-bold text-white mb-1">Actions US — VIX</p>
              <p class="text-[10px] text-white/80 leading-snug">Volatilité implicite du S&amp;P 500 à 30 jours — le baromètre de la peur des actions. Jauge inversée : gauche = calme, droite = panique.</p>
              <p class="text-[10px] mt-1.5 font-medium leading-snug" :class="tonAnalyse(data.vix >= 20 || data.vix < 13)">
                {{ analyseVix(data.vix) }}
              </p>
            </div>
          </Transition>
        </div>

        <!-- Positioning futures (public Bybit) -->
        <div
          v-if="data.bandeau.positioning.length"
          class="relative cursor-help"
          @click.stop="survole = survole === 'pos' ? null : 'pos'"
        >
          <span class="text-[10px] uppercase tracking-wider text-white/80">Positioning futures <span class="text-white/40">ⓘ</span></span>
          <div v-for="p in data.bandeau.positioning" :key="p.asset" class="flex items-center gap-2 mt-0.5">
            <span class="w-8 text-[10px] font-bold text-white">{{ p.asset }}</span>
            <div class="flex-1 h-2 rounded-full overflow-hidden bg-white/10 flex" :title="`${Math.round(p.ratio_long * 100)} % longs / ${Math.round(p.ratio_short * 100)} % courts`">
              <div class="h-full bg-emerald-500/80" :style="{ width: `${p.ratio_long * 100}%` }" />
              <div class="h-full bg-red-500/80" :style="{ width: `${p.ratio_short * 100}%` }" />
            </div>
            <span class="w-14 text-right text-[10px] font-mono tabular-nums text-white">L/S {{ p.ls.toFixed(2) }}</span>
            <span class="w-[74px] text-right text-[9px] font-mono tabular-nums" :class="p.funding_pct > 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ (p.funding_pct >= 0 ? '+' : '') + p.funding_pct.toFixed(3) }} %
            </span>
          </div>
          <Transition name="fade">
            <div v-if="survole === 'pos'" class="tooltip-sentiment" @click.stop>
              <p class="text-[10px] font-bold text-white mb-1">Positioning futures</p>
              <p class="text-[10px] text-white/80 leading-snug">Perpétuels Bybit : L/S = part des comptes longs/courts (ratio 1 j). Funding = paiement mutuel toutes les 8 h — négatif : les shorts paient les longs ; positif : les longs paient.</p>
              <p v-for="p in data.bandeau.positioning" :key="p.asset" class="text-[10px] mt-1.5 font-medium leading-snug" :class="tonAnalyse(p.ls >= 2 || p.ls <= 0.5)">
                {{ p.asset }} — {{ analysePositioning(p) }}
              </p>
            </div>
          </Transition>
        </div>

        <!-- Breadth maison : part des actifs > MM50 -->
        <div
          v-if="data.bandeau.breadth.length"
          class="relative cursor-help"
          @click.stop="survole = survole === 'breadth' ? null : 'breadth'"
        >
          <span class="text-[10px] uppercase tracking-wider text-white/80">Breadth — &gt; MM50 <span class="text-white/40">ⓘ</span></span>
          <div class="flex flex-wrap gap-x-3 gap-y-0.5 mt-0.5">
            <span v-for="b in data.bandeau.breadth" :key="b.univers" class="text-[10px] tabular-nums text-white">
              {{ b.univers }}
              <span :class="b.au_dessus / b.total >= 0.6 ? 'text-emerald-400' : b.au_dessus / b.total <= 0.4 ? 'text-red-400' : 'text-amber-400'">
                {{ b.au_dessus }}/{{ b.total }}
              </span>
            </span>
          </div>
          <Transition name="fade">
            <div v-if="survole === 'breadth'" class="tooltip-sentiment" @click.stop>
              <p class="text-[10px] font-bold text-white mb-1">Breadth — au-dessus de MM50</p>
              <p class="text-[10px] text-white/80 leading-snug">Part des actifs de chaque univers au-dessus de leur moyenne mobile 50 jours (tendance de fond). Mesure la participation collective, pas l'amplitude. Forex exclu jusqu'à l'historique EA.</p>
              <p v-for="b in data.bandeau.breadth" :key="b.univers" class="text-[10px] mt-1.5 font-medium leading-snug" :class="tonAnalyse(b.au_dessus / b.total >= 0.9 || b.au_dessus / b.total <= 0.1)">
                {{ b.univers }} — {{ analyseBreadth(b) }}
              </p>
            </div>
          </Transition>
        </div>

        <!-- Bias presse IA (notations LLM existantes, 48 h) -->
        <div
          v-if="data.bandeau.presse && totalPresse > 0"
          class="relative cursor-help"
          @click.stop="survole = survole === 'presse' ? null : 'presse'"
        >
          <div class="flex items-baseline justify-between">
            <span class="text-[10px] uppercase tracking-wider text-white/80">Presse IA (48 h) <span class="text-white/40">ⓘ</span></span>
            <span class="text-[10px] font-mono tabular-nums text-white">
              H <span class="text-emerald-400">{{ data.bandeau.presse.haussier }}</span> ·
              N <span class="text-white/60">{{ data.bandeau.presse.neutre }}</span> ·
              B <span class="text-red-400">{{ data.bandeau.presse.baissier }}</span>
            </span>
          </div>
          <div class="mt-1 h-2 rounded-full overflow-hidden flex">
            <div class="h-full bg-emerald-500/80" :style="{ width: `${(data.bandeau.presse.haussier / totalPresse) * 100}%` }" />
            <div class="h-full bg-slate-500/60" :style="{ width: `${(data.bandeau.presse.neutre / totalPresse) * 100}%` }" />
            <div class="h-full bg-red-500/80" :style="{ width: `${(data.bandeau.presse.baissier / totalPresse) * 100}%` }" />
          </div>
          <Transition name="fade">
            <div v-if="survole === 'presse'" class="tooltip-sentiment" @click.stop>
              <p class="text-[10px] font-bold text-white mb-1">Presse IA (48 h)</p>
              <p class="text-[10px] text-white/80 leading-snug">Ton des articles des dernières 48 h notés haussier/neutre/baissier par l'analyste LLM du pipeline presse.</p>
              <p class="text-[10px] mt-1.5 font-medium leading-snug" :class="tonAnalyse(Math.abs(biaisPresse) > 0.25)">
                {{ analysePresse() }}
              </p>
            </div>
          </Transition>
        </div>
      </div>

      <!-- ══ MARCHÉS : tableau historique conservé (direction pure, 15/09) ══ -->
      <div class="space-y-2.5 text-xs">
        <div class="flex items-center gap-2 text-[9px] uppercase tracking-wider text-white border-b border-white/5 pb-0.5">
          <span class="flex-1" />
          <span class="w-[64px] text-right">Cours</span>
          <span class="w-[64px] text-center">Veille</span>
          <span class="w-[64px] text-center">Jour</span>
        </div>
        <GroupeMarche label="🇺🇸 USA" :entites="data.usa" />
        <GroupeMarche label="🇪🇺 EUROPE" :entites="data.europe" />
        <GroupeMarche label="⛏️ MATIÈRES PREMIÈRES" :entites="data.matieres_premieres" />
        <GroupeMarche label="₿ CRYPTOS" :entites="data.cryptos" />
      </div>
    </template>

    <!-- Skeleton si premier chargement -->
    <div v-if="!data" class="space-y-2 animate-pulse">
      <div v-for="i in 8" :key="i" class="h-3 rounded bg-white/10" :style="{ width: `${55 + (i % 3) * 15}%` }" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, ref, type PropType } from 'vue'
import { storeToRefs } from 'pinia'
import { useSentimentStore } from '@/stores/sentiment.store'
import type { EntiteSentiment, Positioning, Breadth } from '@/services/api.types.marche'

const store = useSentimentStore()
const { data, chargement, erreur } = storeToRefs(store)

/// Tooltip ouvert (cliqué) — une seule analyse à la fois.
const survole = ref<string | null>(null)

// ── Jauges : couleurs et libellés ───────────────────────────────────────────

function traduireFng(classe: string): string {
  const table: Record<string, string> = {
    'Extreme Fear': 'Peur extrême',
    Fear: 'Peur',
    Neutral: 'Neutre',
    Greed: 'Appêt',
    'Extreme Greed': 'Appêt extrême',
  }
  return table[classe] ?? classe
}

function classeFng(v: number): string {
  if (v <= 24) return 'text-red-400'
  if (v <= 44) return 'text-orange-400'
  if (v <= 55) return 'text-amber-300'
  if (v <= 75) return 'text-lime-400'
  return 'text-emerald-400'
}

function flecheDelta(d: number): string {
  if (d > 0) return `▲${d}`
  if (d < 0) return `▼${Math.abs(d)}`
  return '＝'
}

function classeVix(v: number): string {
  if (v >= 30) return 'text-red-400'
  if (v >= 20) return 'text-orange-400'
  if (v >= 15) return 'text-amber-300'
  return 'text-emerald-400'
}

function verdictVix(v: number): string {
  if (v >= 30) return 'Peur'
  if (v >= 20) return 'Volatil'
  if (v >= 15) return 'Tendu'
  return 'Stable'
}

/// Ton de la ligne d'analyse : ambre quand la lecture signale un extrême
/// (prudence contrarienne), bleu-neutre sinon.
function tonAnalyse(extreme: boolean): string {
  return extreme ? 'text-amber-300' : 'text-sky-300'
}

const totalPresse = computed(() => {
  const p = data.value?.bandeau?.presse
  return p ? p.haussier + p.neutre + p.baissier : 0
})

const biaisPresse = computed(() => {
  const p = data.value?.bandeau?.presse
  if (!p || totalPresse.value === 0) return 0
  return (p.haussier - p.baissier) / totalPresse.value
})

// ── Lectures dynamiques (les analyses des tooltips) ─────────────────────────

function analyseFng(v: number, delta: number): string {
  let lecture: string
  if (v <= 24) lecture = 'Zone de peur extrême — historiquement les meilleures zones d\'achat (lecture contrarienne).'
  else if (v <= 44) lecture = 'Peur installée — les vendeurs s\'épuisent souvent dans cette zone.'
  else if (v <= 55) lecture = 'Neutre — pas de signal exploitable.'
  else if (v <= 75) lecture = 'Appêt — l\'optimisme monte, la prudence croît avec la valeur.'
  else lecture = 'Appêt extrême — zone de distribution historique : la foule achète souvent le sommet.'
  if (Math.abs(delta) >= 8) {
    lecture += ` L'humeur ${delta > 0 ? 's\'améliore' : 'se dégrade'} vite (${Math.abs(delta)} pts/j).`
  }
  return lecture
}

function analyseVix(v: number): string {
  if (v < 13) return 'Calme profond — parfois de la complaisance : les sommets naissent dans l\'indifférence.'
  if (v < 15) return 'Calme — volatilité faible, marché détendu.'
  if (v < 20) return 'Tension normale-haute — volatilité présente sans stress.'
  if (v < 30) return 'Stress — mouvements amples : resserrer les tailles de position.'
  return 'Panique — zones de repli historiques (lecture contrarienne).'
}

function analysePositioning(p: Positioning): string {
  const longs = Math.round(p.ratio_long * 100)
  let lecture: string
  if (p.ls >= 2) lecture = `Foule très longue (${longs} % des comptes) — extrême contrarien : le risque est à la baisse.`
  else if (p.ls >= 1.4) lecture = `Majorité de longs (${longs} %) — biais haussier de la foule.`
  else if (p.ls <= 0.5) lecture = `Foule très courte (${100 - longs} % shorts) — extrême contrarien : risque de short squeeze.`
  else lecture = 'Positioning équilibré — pas de signal.'
  if (p.funding_pct < 0 && p.ls > 1.2) {
    lecture += ' Divergence : comptes longs mais funding négatif — les grosses positions (en valeur) sont courtes et paient les longs. Tension avant mouvement violent.'
  } else if (p.funding_pct > 0.01 && p.ls > 1.5) {
    lecture += ' Les longs dominent ET paient — euphorie coûteuse, sommets fragiles.'
  }
  return lecture
}

function analyseBreadth(b: Breadth): string {
  const ratio = b.au_dessus / b.total
  if (ratio >= 0.9) return `Participation totale (${b.au_dessus}/${b.total}) — tendance haussière installée, mais un tel extrême est fragile aux retournements.`
  if (ratio >= 0.6) return `Majorité au-dessus (${b.au_dessus}/${b.total}) — tendance de fond haussière.`
  if (ratio <= 0.1) return `Participation nulle (${b.au_dessus}/${b.total}) — tendance baissière installée.`
  if (ratio <= 0.4) return `Majorité en dessous (${b.au_dessus}/${b.total}) — tendance de fond baissière.`
  return `Mixte (${b.au_dessus}/${b.total}) — phase de transition.`
}

function analysePresse(): string {
  const biais = biaisPresse.value
  if (biais > 0.25) return `Presse orientée hausse (+${Math.round(biais * 100)} % de biais) — l'optimisme médiatique sert de contraste aux jauges de marché.`
  if (biais < -0.25) return `Presse orientée baisse (${Math.round(biais * 100)} % de biais) — pessimisme médiatique marqué.`
  return 'Presse équilibrée — pas de biais médiatique net.'
}

// ── Tableau Marchés : DIRECTION PURE (décision 15/09) ───────────────────────
// Vert si > 0, rouge si < 0, gris autour de zéro — la sévérité vit dans le
// bandeau (VIX, jauges), le tableau ne porte plus que la direction.

function classeDirection(v: number): string {
  if (v > 0.05) return 'text-emerald-400'
  if (v < -0.05) return 'text-red-400'
  return 'text-slate-400'
}

function flecheDirection(v: number): string {
  if (v > 0.05) return '▲'
  if (v < -0.05) return '▼'
  return '◆'
}

function pct(v: number): string {
  return (v > 0 ? '+' : '') + v.toFixed(2) + '%'
}

function prixFmt(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(v) + ' $'
  return new Intl.NumberFormat('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v) + ' $'
}

/// Une cellule de variation : ▲ +2,10 % (couleur = direction). Veille absente
/// → « — » (plus de repli trompeur sur la variation du jour).
const CelluleVariation = defineComponent({
  props: {
    valeur: { type: Number as PropType<number | null>, required: true },
  },
  setup(props) {
    return () => {
      if (props.valeur === null || props.valeur === undefined) {
        return h('span', { class: 'w-[64px] flex justify-end text-white/30' }, '—')
      }
      return h('span', { class: 'w-[64px] flex items-center justify-end gap-1 tabular-nums' }, [
        h('span', { class: `text-[10px] leading-none ${classeDirection(props.valeur)}` }, flecheDirection(props.valeur)),
        h('span', { class: classeDirection(props.valeur) }, pct(props.valeur)),
      ])
    }
  },
})

/// Groupe du tableau (USA / Europe / MCP / Cryptos) — lignes conservées à
/// l'identique, couleurs en direction pure.
const GroupeMarche = defineComponent({
  props: {
    label: { type: String, required: true },
    entites: { type: Array as PropType<EntiteSentiment[]>, required: true },
  },
  setup(props) {
    return () => h('div', [
      h('p', { class: 'text-white mb-0.5' }, props.label),
      ...props.entites.map(e => h('div', { class: 'flex items-center gap-2 py-0.5' }, [
        h('span', { class: 'flex-1 min-w-0 truncate text-white', title: e.nom }, e.nom),
        h('span', {
          class: `w-[64px] text-right tabular-nums text-[11px] font-medium ${classeDirection(e.variation_pct)}`,
          title: `Cours ${e.nom}`,
        }, prixFmt(e.prix)),
        h(CelluleVariation, { valeur: e.variation_veille ?? null }),
        h(CelluleVariation, { valeur: e.variation_pct }),
      ])),
    ])
  },
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }

.tooltip-sentiment {
  @apply absolute top-full left-0 z-50 mt-1.5
         rounded-xl border border-white/15 bg-[#0f1629]
         p-3 shadow-2xl;
  width: 21rem;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
