<template>
  <!-- PLANCHE MÉTÉO (cockpit 24/09, dessin owner) : la rangée
       d'instruments de surveillance du marché, AU-DESSUS des jauges de
       stratégies. Jauges LINÉAIRES — ces échelles sont bornées Peur↔Appêt,
       pas des valeurs signées comme les perf des stratégies. Rang 1 : F&G
       crypto, VIX, Breadth MM50, Presse IA (jauge L/S retirée — owner 10/10). Rang 2 :
       les tuiles Marchés (cours + veille + jour). Toutes les données de
       l'ancien bloc SentimentMarche vivent ici — rien n'est perdu. -->
  <div class="relative shrink-0 rounded-xl border border-white/10 bg-white/[0.03] p-2.5 pt-5">

    <span class="onglet">MÉTÉO DU MARCHÉ</span>

    <p v-if="erreur" class="text-xs text-red-400 py-3 text-center">Données sentiment indisponibles</p>
    <div v-else-if="!data" class="h-[92px] animate-pulse rounded-lg bg-white/5" />

    <template v-else>

      <!-- ══ Rang 1 : mini-cadrans arc-en-ciel à curseur (owner 26/09) ══ -->
      <div class="flex flex-wrap items-stretch gap-1">

        <MiniCadran v-if="fng" titre="Crypto — Peur & Appêt" :texte="texteFng"
          libelle="F&G Crypto" :valeur="fng.valeur" :min="0" :max="100"
          :digital="String(fng.valeur)" :couleur-texte="classeFng(fng.valeur)"
          :zones="[
            { debut: 0, fin: 25, couleur: '#ef4444' },
            { debut: 25, fin: 45, couleur: '#f97316' },
            { debut: 45, fin: 55, couleur: '#eab308' },
            { debut: 55, fin: 75, couleur: '#84cc16' },
            { debut: 75, fin: 100, couleur: '#22c55e' },
          ]"
          :interpretations="[
            { seuil: 0, texte: 'Peur extrême', couleur: 'text-red-400' },
            { seuil: 25, texte: 'Peur', couleur: 'text-orange-400' },
            { seuil: 45, texte: 'Neutre', couleur: 'text-amber-400' },
            { seuil: 55, texte: 'Appêt', couleur: 'text-lime-400' },
            { seuil: 75, texte: 'Appêt extrême', couleur: 'text-emerald-400' },
          ]" />

        <MiniCadran v-if="vix !== null" titre="Actions US — VIX" :texte="texteVix"
          libelle="VIX" :valeur="vix" :min="0" :max="40"
          :digital="vix.toFixed(1).replace('.', ',')" :couleur-texte="classeVix(vix)"
          :zones="[
            { debut: 0, fin: 15, couleur: '#22c55e' },
            { debut: 15, fin: 20, couleur: '#eab308' },
            { debut: 20, fin: 30, couleur: '#f97316' },
            { debut: 30, fin: 40, couleur: '#ef4444' },
          ]"
          :interpretations="[
            { seuil: 0, texte: 'Calme', couleur: 'text-emerald-400' },
            { seuil: 37.5, texte: 'Tendu', couleur: 'text-amber-400' },
            { seuil: 50, texte: 'Volatil', couleur: 'text-orange-400' },
            { seuil: 75, texte: 'Panique', couleur: 'text-red-400' },
          ]" />

        <MiniCadran v-for="b in breadth" :key="b.univers"
          titre="Variation du jour" :texte="texteBreadth"
          :libelle="b.univers"
          :valeur="variationUniverse(b.univers) ?? 0" :min="-3" :max="3"
          :digital="fmtVar(variationUniverse(b.univers))"
          :couleur-texte="(variationUniverse(b.univers) ?? 0) >= 0.3 ? 'fill-emerald-400' : (variationUniverse(b.univers) ?? 0) <= -0.3 ? 'fill-red-400' : 'fill-white'"
          :zones="[
            { debut: -3, fin: -1, couleur: '#ef4444' },
            { debut: -1, fin: -0.3, couleur: '#f97316' },
            { debut: -0.3, fin: 0.3, couleur: '#64748b' },
            { debut: 0.3, fin: 1, couleur: '#84cc16' },
            { debut: 1, fin: 3, couleur: '#22c55e' },
          ]"
          :interpretations="[
            { seuil: 0, texte: 'Chute', couleur: 'text-red-400' },
            { seuil: 23, texte: 'Baisse', couleur: 'text-orange-400' },
            { seuil: 45, texte: 'Stable', couleur: 'text-white/60' },
            { seuil: 55, texte: 'Hausse', couleur: 'text-lime-400' },
            { seuil: 67, texte: 'Envol', couleur: 'text-emerald-400' },
          ]" />

        <MiniCadran titre="Presse IA (48 h)" :texte="textePresse"
          libelle="Presse 48h" :valeur="biaisPresse" :min="-1" :max="1"
          :digital="totalPresse > 0 ? `H${presse.haussier} B${presse.baissier}` : '—'"
          :couleur-texte="biaisPresse > 0.25 ? 'text-emerald-400' : biaisPresse < -0.25 ? 'text-red-400' : 'text-white'"
          :zones="[
            { debut: -1, fin: -0.25, couleur: '#ef4444' },
            { debut: -0.25, fin: 0.25, couleur: '#64748b' },
            { debut: 0.25, fin: 1, couleur: '#22c55e' },
          ]"
          :interpretations="[
            { seuil: 0, texte: 'Biais baisse', couleur: 'text-red-400' },
            { seuil: 37.5, texte: 'Équilibré', couleur: 'text-white/60' },
            { seuil: 62.5, texte: 'Biais hausse', couleur: 'text-emerald-400' },
          ]" />

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
import MiniCadran from './MiniCadran.vue'

const store = useSentimentStore()
const { data, erreur } = storeToRefs(store)

// ── Raccourcis réactifs ─────────────────────────────────────────────────────
const fng = computed(() => data.value?.bandeau?.fng ?? null)
const vix = computed(() => data.value?.vix ?? null)
const breadth = computed(() => data.value?.bandeau?.breadth ?? [])
const presse = computed(() => data.value?.bandeau?.presse ?? { haussier: 0, neutre: 0, baissier: 0 })

/// Formate une variation : « +0,5 % » / « −1,2 % ».
function fmtVar(v: number | null): string {
  if (v === null) return '—'
  return (v >= 0 ? '+' : '−') + Math.abs(v).toFixed(1).replace('.', ',') + ' %'
}

/// Variation MOYENNE (% du jour) d'un univers — depuis les entités du ticker
/// (owner 26/09 : l'aiguille doit refléter la VIOLENCE du mouvement, pas
/// juste sa direction ; ±3 % = échelle symétrique).
const variationUniverse = (univers: string): number | null => {
  const d = data.value
  if (!d) return null
  let ents: { variation_pct?: number }[] = []
  if (univers === 'Crypto') ents = d.cryptos ?? []
  else if (univers === 'Or') ents = (d.matieres_premieres ?? []).filter(e => /or/i.test(e.nom ?? ''))
  else if (univers === 'Argent') ents = (d.matieres_premieres ?? []).filter(e => /argent/i.test(e.nom ?? ''))
  else if (univers === 'Indices') ents = [...(d.usa ?? []), ...(d.europe ?? [])]
  else return null
  const valides = ents.filter(e => typeof e.variation_pct === 'number')
  if (!valides.length) return null
  return valides.reduce((s2, e) => s2 + (e.variation_pct ?? 0), 0) / valides.length
}

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

  const texteBreadth = computed(() => 'Part des actifs de chaque univers au-dessus de leur moyenne mobile 50 jours (tendance de fond). Mesure la participation collective, pas l\'amplitude. Forex exclu jusqu\'à l\'historique EA.\n\n'
  + breadth.value.map(b => analyseBreadth(b)).join('\n'))

const textePresse = computed(() => {
  const base = 'Ton des articles des dernières 48 h notés haussier/neutre/baissier par l\'analyste LLM du pipeline presse.\n\n'
  if (totalPresse.value === 0) {
    return base + 'Aucun article noté depuis 48 h — une notation naît à l\'ouverture d\'un article dans la Revue de presse.'
  }
  return base + analysePresse()
})

// ── Ticker marchés : direction pure (décision 15/09) ────────────────────────

const groupesMarches = computed(() => {
  const d = data.value
  if (!d) return []
  // CAC40 exclu du ticker (owner 26/09) — l'univers trading ne le suit pas.
  const sansCac = (entites: typeof d.usa) => (entites ?? []).filter(e => !/cac/i.test(e.nom ?? ''))
  return [
    { label: '🇺🇸 USA', entites: sansCac(d.usa) },
    { label: '🇪🇺 EUROPE', entites: sansCac(d.europe) },
    { label: '⛏️ MATIÈRES', entites: sansCac(d.matieres_premieres) },
    { label: '₿ CRYPTOS', entites: sansCac(d.cryptos) },
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
  if (v >= 1000) return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(v) + ' $'
  return new Intl.NumberFormat('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v) + ' $'
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

/* Aiguille des jauges linéaires : curseur blanc gravé */

/* Graduation gravée sous les rails */

/* Repère central des balances bipolaires */

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
