<template>
  <!-- PLANCHE MÉTÉO (cockpit 24/09, dessin owner) : la rangée
       d'instruments de surveillance du marché, AU-DESSUS des jauges de
       stratégies. Réorganisation owner 22/10 : UNE SEULE LIGNE — jauges
       linéaires à gauche (F&G, VIX, breadth Crypto/Indices/Forex — les
       jauges Or, Argent et Presse sont retirées) et le ticker des marchés
       en 2 colonnes à droite (« Indices » et « Matières & Crypto »,
       refonte owner 22/10). Tuiles inchangées : seule la géométrie et
       les titres de colonnes changent. -->
  <div class="relative shrink-0 rounded-xl border border-white/10 bg-white/[0.03] p-2.5 pt-5">

    <span class="onglet">MÉTÉO DU MARCHÉ</span>

    <p v-if="erreur" class="text-xs text-red-400 py-3 text-center">Données sentiment indisponibles</p>
    <div v-else-if="!data" class="h-[92px] animate-pulse rounded-lg bg-white/5" />

    <template v-else>

      <!-- ══ La ligne unique : mini-cadrans ┃ tickers en 2 colonnes.
           CONTRAINTE OWNER 22/10 : jauges ET tickers TOUJOURS sur la même
           ligne — la ligne ne casse jamais ; quand la place manque ce
           sont les JAUGES qui se rangent sur plusieurs rangées (leur zone
           est rétrécissable), la zone tickers garde sa largeur mini. ══ -->
      <div class="flex items-stretch gap-3">

        <!-- Zone jauges (owner 22/10 : Or, Argent, Presse supprimées) -->
        <div class="flex flex-wrap items-stretch gap-1 flex-[3] min-w-0">
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

          <MiniCadran v-for="b in breadthFiltre" :key="b.univers"
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
        </div>

        <!-- Zone tickers : les mêmes tuiles, en 2 colonnes — Indices
             (USA + Europe) et Matières & Crypto. RESPONSIVE (owner 22/10) :
             JAMAIS sous les jauges — largeur mini garantie, les colonnes
             ne s'écrasent jamais. Largeur ≈ le quart de la ligne
             (owner 22/10 : « réduire de moitié ») : les jauges prennent
             les 3/4 (flex-[3] vs flex-1). -->
        <div class="flex-1 min-w-[460px] border-l border-white/15 pl-3">
          <div class="grid grid-cols-2 gap-x-3 items-start">
            <div v-for="g in groupesColonnes" :key="g.label" class="flex flex-col gap-1">
              <span class="text-[11px] font-extrabold uppercase tracking-wider text-white/75 whitespace-nowrap">{{ g.label }}</span>
              <div v-for="e in g.entites" :key="e.nom" class="tuile w-full" :class="fondTuile(e.variation_pct)" :title="`Cours ${e.nom}`">
                <p class="truncate text-[10px] font-bold tabular-nums" :class="classeDirection(e.variation_pct)">
                  {{ e.nom }} à {{ prixFmt(e.prix) }}
                </p>
                <p class="text-[10px] tabular-nums leading-tight whitespace-nowrap">
                  <span class="text-white/50">Hier :</span>
                  <span :class="classeDirection(e.variation_veille)">{{ ' ' + variation(e.variation_veille) }}</span>
                  <span class="text-white/25 mx-1">-</span>
                  <span class="text-white/50">Aujourd'hui :</span>
                  <span :class="classeDirection(e.variation_pct)">{{ ' ' + variation(e.variation_pct) }}</span>
                </p>
              </div>
            </div>
          </div>
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
/// Owner 22/10 : les jauges Or et Argent quittent la planche (les cours
/// vivent dans le ticker) — seuls Crypto, Indices et Forex restent.
const breadthFiltre = computed(() =>
  (data.value?.bandeau?.breadth ?? []).filter(b => !['Or', 'Argent'].includes(b.univers)))

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
  else if (univers === 'Indices') ents = [...(d.usa ?? []), ...(d.europe ?? [])]
  else return null
  const valides = ents.filter(e => typeof e.variation_pct === 'number')
  if (!valides.length) return null
  return valides.reduce((s2, e) => s2 + (e.variation_pct ?? 0), 0) / valides.length
}

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

// ── Textes des popovers : définition + lecture ──────────────────────────────

const texteFng = computed(() => fng.value
  ? 'Indice 0-100 agrégeant volatilité, momentum, réseaux sociaux et recherches (alternative.me). 0 = peur extrême, 100 = appêt extrême.\n\n' + analyseFng(fng.value.valeur, fng.value.delta_veille)
  : '')

const texteVix = computed(() => vix.value !== null
  ? 'Volatilité implicite du S&P 500 à 30 jours — le baromètre de la peur des actions. Jauge inversée : gauche = calme, droite = panique.\n\n' + analyseVix(vix.value)
  : '')

  const texteBreadth = computed(() => 'Part des actifs de chaque univers au-dessus de leur moyenne mobile 50 jours (tendance de fond). Mesure la participation collective, pas l\'amplitude. Forex exclu jusqu\'à l\'historique EA.\n\n'
  + breadthFiltre.value.map(b => analyseBreadth(b)).join('\n'))

// ── Ticker marchés : direction pure (décision 15/09) — mêmes tuiles,
//    2 colonnes refondues owner 22/10 : « Indices » (USA + Europe) et
//    « Matières & Crypto ». Cac 40 toujours exclu (owner 26/09). ────────────

const groupesColonnes = computed(() => {
  const d = data.value
  if (!d) return []
  const sansCac = (entites: typeof d.usa) => (entites ?? []).filter(e => !/cac/i.test(e.nom ?? ''))
  return [
    { label: 'Indices', entites: [...sansCac(d.usa), ...sansCac(d.europe)] },
    { label: 'Matières & Crypto', entites: [...sansCac(d.matieres_premieres), ...sansCac(d.cryptos)] },
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
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.25em;
  color: rgba(255, 255, 255, 0.6);
  background: #111827;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

/* Libellé gravé d'instrument */
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
