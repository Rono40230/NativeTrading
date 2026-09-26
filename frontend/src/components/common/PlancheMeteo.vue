<template>
  <!-- PLANCHE MÉTÉO (cockpit 24/09, dessin owner) : la rangée
       d'instruments de surveillance du marché, AU-DESSUS des jauges de
       stratégies. Jauges LINÉAIRES — ces échelles sont bornées Peur↔Appêt,
       pas des valeurs signées comme les perf des stratégies. Rang 1 : F&G
       crypto, VIX, Positioning futures, Breadth MM50, Presse IA. Rang 2 :
       les tuiles Marchés (cours + veille + jour). Toutes les données de
       l'ancien bloc SentimentMarche vivent ici — rien n'est perdu. -->
  <div class="relative shrink-0 rounded-xl border border-white/10 bg-white/[0.03] p-2.5 pt-5">

    <span class="onglet">MÉTÉO DU MARCHÉ</span>

    <p v-if="erreur" class="text-xs text-red-400 py-3 text-center">Données sentiment indisponibles</p>
    <div v-else-if="!data" class="h-[92px] animate-pulse rounded-lg bg-white/5" />

    <template v-else>

      <!-- ══ Rang 1 : les instruments linéaires ══ -->
      <div class="flex flex-wrap gap-x-4 gap-y-2">

        <!-- Colonne humeur : F&G au-dessus, VIX en dessous (décision
             owner 25/09 — les deux baromètres d'humeur se lisent en pile) -->
        <div class="flex-1 min-w-[190px] flex flex-col gap-2">

          <!-- ── Crypto — Peur & Appêt ── -->
          <div v-if="fng">
            <PopoverInfo titre="Crypto — Peur & Appêt" :texte="texteFng">
              <div class="flex items-baseline justify-between gap-2 cursor-help">
                <span class="grave">CRYPTO — PEUR &amp; APPÊT <span class="text-white/40">ⓘ</span></span>
                <span class="digital tabular-nums" :class="classeFng(fng.valeur)">
                  {{ fng.valeur }} · {{ traduireFng(fng.classe) }}
                  <span class="text-[9px] font-normal text-white/60">{{ flecheDelta(fng.delta_veille) }}</span>
                </span>
              </div>
            </PopoverInfo>
            <div class="relative mt-1">
              <div class="h-2.5 rounded-full overflow-hidden flex ring-1 ring-black/40">
                <div class="h-full" style="width:25%;background:#ef4444" />
                <div class="h-full" style="width:20%;background:#f97316" />
                <div class="h-full" style="width:10%;background:#eab308" />
                <div class="h-full" style="width:20%;background:#84cc16" />
                <div class="h-full" style="width:25%;background:#22c55e" />
              </div>
              <span class="aiguille" :style="{ left: fng.valeur + '%' }" />
              <div class="flex justify-between mt-0.5">
                <span v-for="t in [0, 25, 50, 75, 100]" :key="t" class="tick" />
              </div>
            </div>
          </div>

          <!-- ── Actions US — VIX (jauge inversée) ── -->
          <div v-if="vix !== null">
            <PopoverInfo titre="Actions US — VIX" :texte="texteVix">
              <div class="flex items-baseline justify-between gap-2 cursor-help">
                <span class="grave">ACTIONS US — VIX <span class="text-white/40">ⓘ</span></span>
                <span class="digital tabular-nums" :class="classeVix(vix)">{{ vix.toFixed(1) }} · {{ verdictVix(vix) }}</span>
              </div>
            </PopoverInfo>
            <div class="relative mt-1">
              <div class="h-2.5 rounded-full overflow-hidden flex ring-1 ring-black/40">
                <div class="h-full" style="width:37.5%;background:#22c55e" />
                <div class="h-full" style="width:12.5%;background:#eab308" />
                <div class="h-full" style="width:25%;background:#f97316" />
                <div class="h-full" style="width:25%;background:#ef4444" />
              </div>
              <span class="aiguille" :style="{ left: Math.min(100, (vix / 40) * 100) + '%' }" />
              <div class="flex justify-between mt-0.5">
                <span v-for="t in [0, 25, 50, 75, 100]" :key="t" class="tick" />
              </div>
            </div>
          </div>

        </div>

        <!-- ── Positioning futures (balances L↔S par actif) ── -->
        <div v-if="positioning.length" class="flex-1 min-w-[240px]">
          <PopoverInfo titre="Positioning futures" :texte="textePositioning">
            <div class="flex items-baseline justify-between gap-2 cursor-help">
              <span class="grave">POSITIONING FUTURES <span class="text-white/40">ⓘ</span></span>
              <span class="digital tabular-nums text-white/70">L/S · funding</span>
            </div>
          </PopoverInfo>
          <div v-for="p in positioning" :key="p.asset" class="flex items-center gap-2 mt-1">
            <span class="w-8 text-[10px] font-bold text-white">{{ p.asset }}</span>
            <div class="relative flex-1 h-2 rounded-full overflow-hidden ring-1 ring-black/40">
              <div class="h-full bg-emerald-500/80" :style="{ width: p.ratio_long * 100 + '%' }" />
              <div class="h-full bg-red-500/80" :style="{ width: p.ratio_short * 100 + '%' }" />
              <span class="centre" />
              <span class="aiguille petite" :style="{ left: p.ratio_long * 100 + '%' }" />
            </div>
            <span class="w-[52px] text-right text-[10px] font-mono tabular-nums text-white">{{ p.ls.toFixed(2) }}</span>
            <span class="w-[58px] text-right text-[9px] font-mono tabular-nums" :class="p.funding_pct > 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ (p.funding_pct >= 0 ? '+' : '') + p.funding_pct.toFixed(3) }} %
            </span>
          </div>
        </div>

        <!-- ── Breadth — part des actifs > MM50 ── -->
        <div v-if="breadth.length" class="flex-1 min-w-[170px]">
          <PopoverInfo titre="Breadth — au-dessus de MM50" :texte="texteBreadth">
            <div class="flex items-baseline justify-between gap-2 cursor-help">
              <span class="grave">BREADTH — MM50 <span class="text-white/40">ⓘ</span></span>
              <span class="digital tabular-nums text-white/70">au-dessus / total</span>
            </div>
          </PopoverInfo>
          <div v-for="b in breadth" :key="b.univers" class="flex items-center gap-2 mt-1">
            <span class="w-14 truncate text-[10px] font-bold text-white" :title="b.univers">{{ b.univers }}</span>
            <div class="relative flex-1 h-2 rounded-full bg-white/10 ring-1 ring-black/40 overflow-hidden">
              <div class="h-full" :class="couleurBreadth(b)" :style="{ width: (b.au_dessus / b.total) * 100 + '%' }" />
              <span class="centre" />
            </div>
            <span class="w-[46px] text-right text-[10px] font-mono tabular-nums"
                  :class="b.au_dessus / b.total >= 0.6 ? 'text-emerald-400' : b.au_dessus / b.total <= 0.4 ? 'text-red-400' : 'text-amber-400'">
              {{ b.au_dessus }}/{{ b.total }}
            </span>
          </div>
        </div>

        <!-- ── Presse IA 48 h (biais H/N/B) ── -->
        <div v-if="presse && totalPresse > 0" class="flex-1 min-w-[160px]">
          <PopoverInfo titre="Presse IA (48 h)" :texte="textePresse">
            <div class="flex items-baseline justify-between gap-2 cursor-help">
              <span class="grave">PRESSE IA — 48 H <span class="text-white/40">ⓘ</span></span>
              <span class="digital tabular-nums font-mono text-white">
                H <span class="text-emerald-400">{{ presse.haussier }}</span> ·
                N <span class="text-white/60">{{ presse.neutre }}</span> ·
                B <span class="text-red-400">{{ presse.baissier }}</span>
              </span>
            </div>
          </PopoverInfo>
          <div class="relative mt-1">
            <div class="h-2.5 rounded-full overflow-hidden flex ring-1 ring-black/40">
              <div class="h-full bg-emerald-500/80" :style="{ width: (presse.haussier / totalPresse) * 100 + '%' }" />
              <div class="h-full bg-slate-500/60" :style="{ width: (presse.neutre / totalPresse) * 100 + '%' }" />
              <div class="h-full bg-red-500/80" :style="{ width: (presse.baissier / totalPresse) * 100 + '%' }" />
            </div>
            <span class="aiguille" :style="{ left: ((biaisPresse + 1) / 2) * 100 + '%' }" />
            <div class="flex justify-between mt-0.5">
              <span v-for="t in [0, 25, 50, 75, 100]" :key="t" class="tick" />
            </div>
          </div>
        </div>

      </div>

      <!-- ══ Rang 2 : le ticker des marchés (pleine largeur, séparateurs
           entre catégories, direction dans la 1re ligne) ══ -->
      <div class="mt-2.5 pt-2 border-t border-white/10">
        <div class="flex items-baseline justify-between mb-1.5">
          <span class="grave">TICKER MARCHÉS</span>
        </div>
        <div class="flex flex-wrap items-stretch gap-1.5">
          <template v-for="(g, i) in groupesMarches" :key="g.label">
            <div v-if="i > 0" class="w-px self-stretch bg-white/15 mx-0.5" />
            <span class="self-center text-[9px] font-extrabold uppercase tracking-wider text-white/75 whitespace-nowrap">{{ g.label }}</span>
            <div v-for="e in g.entites" :key="e.nom" class="tuile flex-1 min-w-[200px] max-w-[300px]" :class="fondTuile(e.variation_pct)" :title="`Cours ${e.nom}`">
              <p class="truncate text-[10px] font-bold tabular-nums" :class="classeDirection(e.variation_pct)">
                {{ e.nom }} à {{ prixFmt(e.prix) }}
              </p>
              <p class="text-[9px] tabular-nums leading-tight whitespace-nowrap">
                <span class="text-white/50">Hier :</span>
                <span :class="classeDirection(e.variation_veille)">{{ ' ' + variation(e.variation_veille) }}</span>
                <span class="text-white/25 mx-1">-</span>
                <span class="text-white/50">Aujourd'hui :</span>
                <span :class="classeDirection(e.variation_pct)">{{ ' ' + variation(e.variation_pct) }}</span>
              </p>
            </div>
          </template>
        </div>
      </div>

    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useSentimentStore } from '@/stores/sentiment.store'
import PopoverInfo from './PopoverInfo.vue'

const store = useSentimentStore()
const { data, erreur } = storeToRefs(store)

// ── Raccourcis réactifs ─────────────────────────────────────────────────────
const fng = computed(() => data.value?.bandeau?.fng ?? null)
const vix = computed(() => data.value?.vix ?? null)
const positioning = computed(() => data.value?.bandeau?.positioning ?? [])
const breadth = computed(() => data.value?.bandeau?.breadth ?? [])
const presse = computed(() => data.value?.bandeau?.presse ?? null)

const totalPresse = computed(() => {
  const p = presse.value
  return p ? p.haussier + p.neutre + p.baissier : 0
})
const biaisPresse = computed(() => {
  const p = presse.value
  if (!p || totalPresse.value === 0) return 0
  return (p.haussier - p.baissier) / totalPresse.value
})

// ── Couleurs / libellés des lectures digitales ──────────────────────────────

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

function couleurBreadth(b: { au_dessus: number; total: number }): string {
  const r = b.au_dessus / b.total
  if (r >= 0.6) return 'bg-emerald-500/80'
  if (r <= 0.4) return 'bg-red-500/80'
  return 'bg-amber-500/80'
}

// ── Lectures (les analyses des popovers — texte de l'ancien bloc) ───────────

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

function analysePositioning(p: { ratio_long: number; ls: number; funding_pct: number }): string {
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

function analyseBreadth(b: { univers: string; au_dessus: number; total: number }): string {
  const ratio = b.au_dessus / b.total
  if (ratio >= 0.9) return `${b.univers} : participation totale (${b.au_dessus}/${b.total}) — tendance installée, mais un tel extrême est fragile aux retournements.`
  if (ratio >= 0.6) return `${b.univers} : majorité au-dessus (${b.au_dessus}/${b.total}) — tendance de fond haussière.`
  if (ratio <= 0.1) return `${b.univers} : participation nulle (${b.au_dessus}/${b.total}) — tendance baissière installée.`
  if (ratio <= 0.4) return `${b.univers} : majorité en dessous (${b.au_dessus}/${b.total}) — tendance de fond baissière.`
  return `${b.univers} : mixte (${b.au_dessus}/${b.total}) — phase de transition.`
}

function analysePresse(): string {
  const biais = biaisPresse.value
  if (biais > 0.25) return `Presse orientée hausse (+${Math.round(biais * 100)} % de biais) — l'optimisme médiatique sert de contraste aux jauges de marché.`
  if (biais < -0.25) return `Presse orientée baisse (${Math.round(biais * 100)} % de biais) — pessimisme médiatique marqué.`
  return 'Presse équilibrée — pas de biais médiatique net.'
}

// ── Textes des popovers : définition + lecture ──────────────────────────────

const texteFng = computed(() => fng.value
  ? 'Indice 0-100 agrégeant volatilité, momentum, réseaux sociaux et recherches (alternative.me). 0 = peur extrême, 100 = appêt extrême.\n\n' + analyseFng(fng.value.valeur, fng.value.delta_veille)
  : '')

const texteVix = computed(() => vix.value !== null
  ? 'Volatilité implicite du S&P 500 à 30 jours — le baromètre de la peur des actions. Jauge inversée : gauche = calme, droite = panique.\n\n' + analyseVix(vix.value)
  : '')

const textePositioning = computed(() => 'Perpétuels Bybit : L/S = part des comptes longs/courts (ratio 1 j), aiguille = part des comptes longs. Funding = paiement mutuel toutes les 8 h — négatif : les shorts paient les longs ; positif : les longs paient.\n\n'
  + positioning.value.map(p => `${p.asset} — ${analysePositioning(p)}`).join('\n'))

const texteBreadth = computed(() => 'Part des actifs de chaque univers au-dessus de leur moyenne mobile 50 jours (tendance de fond). Mesure la participation collective, pas l\'amplitude. Forex exclu jusqu\'à l\'historique EA.\n\n'
  + breadth.value.map(b => analyseBreadth(b)).join('\n'))

const textePresse = computed(() => 'Ton des articles des dernières 48 h notés haussier/neutre/baissier par l\'analyste LLM du pipeline presse.\n\n' + analysePresse())

// ── Ticker marchés : direction pure (décision 15/09) ────────────────────────

const groupesMarches = computed(() => {
  const d = data.value
  if (!d) return []
  return [
    { label: '🇺🇸 USA', entites: d.usa ?? [] },
    { label: '🇪🇺 EUROPE', entites: d.europe ?? [] },
    { label: '⛏️ MATIÈRES', entites: d.matieres_premieres ?? [] },
    { label: '₿ CRYPTOS', entites: d.cryptos ?? [] },
  ].filter(g => g.entites.length > 0)
})

function classeDirection(v: number | null | undefined): string {
  if (v === null || v === undefined) return 'text-white/30'
  if (v > 0.05) return 'text-emerald-400'
  if (v < -0.05) return 'text-red-400'
  return 'text-slate-400'
}

function variation(v: number | null | undefined): string {
  if (v === null || v === undefined) return '—'
  return (v > 0 ? '▲' : v < 0 ? '▼' : '◆') + (v > 0 ? '+' : '') + v.toFixed(2) + ' %'
}

/// Fond pastel de la tuile : vert en hausse, rouge en baisse (décision
/// owner 24/09) — même seuil « direction pure » que le texte (±0,05 %).
function fondTuile(v: number | null | undefined): string {
  if (v === null || v === undefined) return 'bg-white/[0.04] border-white/10'
  if (v > 0.05) return 'bg-emerald-500/[0.12] border-emerald-500/30'
  if (v < -0.05) return 'bg-red-500/[0.12] border-red-500/30'
  return 'bg-white/[0.04] border-white/10'
}

/// Prix compact façon ticker : groupé façon FR, sans unité (indices en
/// points, paires sans devise — le « $ » des cryptos vit ailleurs).
function prixFmt(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(v)
  return new Intl.NumberFormat('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v)
}
</script>

<style scoped>
/* Onglet gravé du panneau (même langage que le bloc STRATÉGIES). À
   l'INTÉRIEUR du panneau : les blocs clippés (overflow-hidden pour la
   carte monde, parents scrollables) coupaient l'onglet à cheval. */
.onglet {
  position: absolute;
  top: 4px;
  left: 10px;
  padding: 0 8px;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.25em;
  color: rgba(255, 255, 255, 0.6);
  background: #111827;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

/* Libellé gravé d'instrument */
.grave {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.12em;
  color: rgba(255, 255, 255, 0.8);
  text-transform: uppercase;
  white-space: nowrap;
}
.digital { font-size: 11px; font-weight: 700; }

/* Aiguille des jauges linéaires : curseur blanc gravé */
.aiguille {
  position: absolute;
  top: -4px;
  width: 3px;
  height: 16px;
  border-radius: 9999px;
  background: #f8fafc;
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
  transform: translateX(-50%);
  transition: left 600ms cubic-bezier(0.2, 0.8, 0.3, 1);
}
.aiguille.petite { top: -2px; height: 12px; }

/* Graduation gravée sous les rails */
.tick {
  width: 1px;
  height: 4px;
  background: rgba(255, 255, 255, 0.3);
}

/* Repère central des balances bipolaires */
.centre {
  position: absolute;
  left: 50%;
  top: 0;
  bottom: 0;
  width: 1px;
  background: rgba(255, 255, 255, 0.45);
  transform: translateX(-50%);
}

/* Tuile du ticker marchés : deux lignes — identité+direction, puis
   Hier/Aujourd'hui. Forme seule ici : fond ET couleur de bordure viennent
   des classes fondTuile() (pastel directionnelle) — ne rien remettre en
   background/border-color ici, ça écraserait les utilitaires Tailwind. */
.tuile {
  padding: 3px 6px;
  border-width: 1px;
  border-style: solid;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  justify-content: center;
}
</style>
