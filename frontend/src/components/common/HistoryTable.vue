<template>
  <table class="w-full text-sm">
    <thead>
      <tr class="text-white text-xs uppercase border-b border-white/10">
        <th class="px-3 py-3 text-left">#</th>
        <th class="px-1 py-3 text-center" title="Journal de bord du trade (notes du propriétaire)">📝</th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'asset')">Asset <span class="tri-icone">{{ icone('asset') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'timeframe')">TF / Phase <span class="tri-icone">{{ icone('timeframe') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'direction')">Direction <span class="tri-icone">{{ icone('direction') }}</span></th>
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'score')">Score <span class="tri-icone">{{ icone('score') }}</span></th>
        <th class="px-3 py-3 text-right" :title="variante === 'kdj'
          ? 'Position en unités au moment de l\u2019émission — risque % du capital composé, quantité = risque ÷ distance au SL'
          : 'Taille de position au moment de l\u2019émission — recalculée : capital composé de la stratégie × risque % / (stop en pips × valeur du pip)'">{{ variante === 'kdj' ? 'Position' : 'Lot' }}</th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'prix_entree')">Entrée <span class="tri-icone">{{ icone('prix_entree') }}</span></th>
        <!-- Variante KDJ (10/10) : pas de colonnes SL/TP (niveaux figés, sans
             valeur de relecture — ils vivent sur le graphique et la table des
             positions en cours) ni Score ni Stratégie (redondants page dédiée). -->
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'stop_loss')">SL <span class="tri-icone">{{ icone('stop_loss') }}</span></th>
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp1')">TP1 <span class="tri-icone">{{ icone('tp1') }}</span></th>
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp2')">TP2 <span class="tri-icone">{{ icone('tp2') }}</span></th>
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp3')">TP3 <span class="tri-icone">{{ icone('tp3') }}</span></th>
        <th v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right">Prix actuel</th>
        <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'prix_verdict')">Sortie <span class="tri-icone">{{ icone('prix_verdict') }}</span></th>
        <th class="px-3 py-3 text-center">IA</th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'r_reference')">Palier max <span class="tri-icone">{{ icone('r_reference') }}</span></th>
        <!-- Variante KDJ (10/10) : la voix $ en colonnes dédiées — Gain/Perte
             du trade puis capital composé après clôture (décision owner). -->
        <th v-if="variante === 'kdj'" class="px-3 py-3 text-right" title="$ réellement encaissé sur ce trade (voix résultat — ventes partielles comprises)">Gain/Perte</th>
        <th v-if="variante === 'kdj'" class="px-3 py-3 text-right" title="Capital de la stratégie APRÈS la clôture de ce trade (composé, ordre chronologique)">Évolution du capital</th>
        <th v-if="variante !== 'kdj'" class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'strategie')">Stratégie <span class="tri-icone">{{ icone('strategie') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'cree_le')">Ouvert le <span class="tri-icone">{{ icone('cree_le') }}</span></th>
        <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'ferme_le')">Fermé le <span class="tri-icone">{{ icone('ferme_le') }}</span></th>
        <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" title="Vie de la position : du remplissage de l'ordre à la fermeture (l'attente de l'ordre en attente n'est pas comptée)" @click="$emit('trier-par', 'duree')">Durée <span class="tri-icone">{{ icone('duree') }}</span></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="(s, i) in signaux" :key="s.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
        <td class="px-3 py-3 text-white">{{ i + 1 }}</td>
        <td class="px-1 py-3 text-center">
          <button
            class="text-[11px] font-mono rounded px-1 transition-colors"
            :class="journalComptes?.[s.id] ? 'bg-teal-500/20 text-teal-300 hover:bg-teal-500/30' : 'text-white/30 hover:text-white'"
            :title="journalComptes?.[s.id]
              ? `${journalComptes[s.id]} note(s) — ouvrir le journal`
              : 'Ouvrir le journal de bord du trade'"
            @click="journalSignal = s"
          >{{ journalComptes?.[s.id] ?? '+' }}</button>
        </td>
        <td class="px-3 py-3 font-semibold text-white">{{ s.asset }}</td>
        <td class="px-3 py-3 text-white">{{ s.timeframe }}</td>
        <td class="px-3 py-3">
          <span class="badge" :class="s.direction?.toUpperCase() === 'LONG' ? 'badge-green' : 'badge-red'">{{ s.direction }}</span>
        </td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-white">{{ s.score.toFixed(0) }}</td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-white" title="Lot recalculé (capital composé de la stratégie au moment de l'émission)">{{ formatLot(lotMap[s.id]) }}</td>
        <!-- Variante KDJ : position en UNITÉS + risque $ (décision owner 10/10). -->
        <td v-if="variante === 'kdj'" class="px-3 py-3 text-right leading-tight">
          <template v-if="positionKdjHisto(s)">
            <div class="font-mono font-bold text-yellow-300">{{ fmtUnites(positionKdjHisto(s)!.unites) }} unités</div>
            <div class="text-[10px] text-white">{{ positionKdjHisto(s)!.risque.toFixed(0) }} $ risqués</div>
            <div class="text-[9px] text-white/40">≈ {{ Math.round(positionKdjHisto(s)!.engage) }} $ engagés</div>
          </template>
          <span v-else class="text-white text-xs">—</span>
        </td>
        <td class="px-3 py-3 text-right font-mono text-white">{{ formatNombre(s.prix_entree) }}</td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-red-400">{{ formatNombre(s.stop_loss) }}</td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-emerald-400">{{ formatNombre(s.take_profit[0]) }}</td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-emerald-300">{{ s.take_profit[1] ? formatNombre(s.take_profit[1]) : '—' }}</td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-right font-mono text-emerald-200">{{ s.take_profit[2] ? formatNombre(s.take_profit[2]) : '—' }}</td>
        <td v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right font-mono" :class="classePrixActuelSignal(s, prixStore.getPrix(s.asset))">{{ prixStore.getPrix(s.asset) !== null ? formatNombre(prixStore.getPrix(s.asset)!) : '—' }}</td>
        <!-- Sortie = information secondaire (gestion d'exécution), le R de
             référence vit dans la colonne Palier max. -->
        <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right">
          <span class="font-mono text-white text-xs">{{ s.prix_verdict ? formatNombre(s.prix_verdict) : '—' }}</span>
        </td>
        <td class="px-3 py-3 text-center"><span v-if="s.llm_conviction !== null" class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help" :class="classeConviction(s.llm_conviction)" :title="s.llm_raison ?? ''">{{ s.llm_conviction }}</span><span v-else class="text-white text-xs">—</span></td>
        <td class="px-3 py-3">
          <div class="flex flex-col gap-0.5">
            <div class="flex items-center gap-2">
              <span v-if="palierFerme(s)" class="badge" :class="classePalierMax(palierFerme(s))">{{ labelPalierMax(palierFerme(s)) }}</span>
              <span v-else class="badge" :class="classeEtatSignal(s)" :title="titreEtatSignal(s)">{{ labelEtatSignal(s) }}</span>
              <span v-if="rReference(s) !== null" :class="classeR(rReference(s))" class="text-xs" title="R distance : niveau le plus lointain atteint (juge l'entrée et les TP)">{{ formatR(rReference(s)) }}</span>
              <span v-if="variante !== 'kdj' && profitMap[s.id] !== undefined" class="text-[10px] font-mono shrink-0" :class="profitMap[s.id] >= 0 ? 'text-emerald-300/80' : 'text-red-300/80'" title="$ réellement encaissé (ventes partielles comprises)">{{ formatDollarsTrade(s.id) }}</span>
              <span v-if="pointsPalier(s)" class="text-[10px] font-mono" :class="classeR(rReference(s))" title="Gain/perte en points MT5 (R de référence × risque en points — unité du broker)">{{ pointsPalier(s) }}</span>
            </div>
            <!-- MFE des perdants : l'excursion favorable avant le SL juge le
                 placement des niveaux (frôler TP1 puis claquer = info clé). -->
            <span v-if="palierFerme(s) === 'SL' && mfeMap[s.id] !== undefined"
                  class="text-amber-400 text-xs cursor-help"
                  title="Excursion favorable maximale avant le SL (calcul sur bougies M1)">{{ formatMfe(mfeMap[s.id]?.mfe_r ?? null) }}</span>
          </div>
        </td>
        <!-- Variante KDJ : $ du trade puis capital composé après clôture. -->
        <td v-if="variante === 'kdj'" class="px-3 py-3 text-right font-mono text-xs font-semibold" :class="(profitMap[s.id] ?? 0) >= 0 ? 'text-emerald-300' : 'text-red-300'">
          {{ profitMap[s.id] !== undefined ? formatDollarsTrade(s.id) : '—' }}
        </td>
        <td v-if="variante === 'kdj'" class="px-3 py-3 text-right font-mono text-xs" :class="(profitMap[s.id] ?? 0) >= 0 ? 'text-white' : 'text-white'" title="Capital de la stratégie après ce trade (composé)">
          {{ capitaux?.[s.id] !== undefined ? capitaux[s.id].toLocaleString('fr-FR', { maximumFractionDigits: 0 }) + ' $' : '—' }}
        </td>
        <td v-if="variante !== 'kdj'" class="px-3 py-3 text-white text-xs">{{ s.strategie === 'SMC Directionnel' ? 'SMC' : s.strategie }}</td>
        <td class="px-3 py-3 text-white text-xs cursor-help" :title="titreOuverture(s)">{{ formatDate(s.heure_entree ?? s.cree_le) }}</td>
        <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-white text-xs">{{ s.ferme_le ? formatDate(s.ferme_le) : '—' }}</td>
        <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 font-mono text-white text-xs">{{ formatDuree(s.heure_entree, s.ferme_le) }}</td>
      </tr>
    </tbody>
  </table>

  <JournalBordModal
    :ouvert="journalSignal !== null"
    :signal-id="journalSignal?.id ?? ''"
    :titre="journalSignal ? `${journalSignal.asset} · ${journalSignal.timeframe} · ${journalSignal.direction} — fermé le ${formatDate(journalSignal.ferme_le ?? journalSignal.cree_le)}` : ''"
    @fermer="journalSignal = null"
    @note="emit('journal-maj')"
  />
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import type { Signal } from '@/services/api.service'
import { usePrixStore } from '@/stores/prix.store'
import { ref } from 'vue'
import JournalBordModal from './JournalBordModal.vue'
import { useAssetParamsStore } from '@/stores/assetParams.store'
import {
  formatDate, formatNombre, classeEtatSignal, labelEtatSignal, titreEtatSignal,
  formatR, classeR, formatDuree,
  palierMax, labelPalierMax, classePalierMax, formatMfe,
  calculerPositionKdj, fmtUnites,
  type PalierMax,
} from '@/composables/useSignalFormat'

const props = defineProps<{
  signaux: Signal[]
  filtreStatut: 'en_cours' | 'cloturees' | ''
  triColonne: string
  triDir: 'asc' | 'desc'
  /**
   * Variante de rendu (09/10) : 'kdj' = page KDJ — pas de colonnes
   * SL/TP/Stratégie (niveaux figés sans valeur de relecture, stratégie
   * redondante sur la page dédiée), verdict moteur en badge.
   */
  variante?: 'standard' | 'kdj'
  /** MFE des trades SL : { [id]: { mfe_r, meilleur_prix } } */
  mfe?: Record<string, { mfe_r: number | null; meilleur_prix: number | null }>
  /** Lot recalculé par trade : { [id]: lot } — vide si non chargé. */
  lots?: Record<string, number>
  /** Nombre de notes du journal par trade : { [id]: n }. */
  journalComptes?: Record<string, number>
  /** $ réel par trade (re-jeu capital) : { [id]: dollars }. */
  profits?: Record<string, number>
  /** Capital composé APRÈS chaque clôture : { [id]: capital } — variante kdj. */
  capitaux?: Record<string, number>
}>()

const emit = defineEmits<{
  'trier-par': [col: string]
  'journal-maj': []
}>()

/** Trade dont le journal est ouvert (null = fermé). */
const journalSignal = ref<Signal | null>(null)

const assetParams = useAssetParamsStore()
onMounted(() => { if (!assetParams.liste.length) void assetParams.charger() })

/// Position KDJ (variante kdj) : unités + risque $ + engagé, depuis le lot
/// backend (capital composé à l'émission) et la distance réelle du trade.
function positionKdjHisto(s: Signal): { unites: number; risque: number; engage: number } | null {
  const p = assetParams.liste.find(x => x.asset === s.asset)
  if (!p) return null
  const calc = calculerPositionKdj(lotMap.value[s.id], s.prix_entree, s.stop_loss, p.taille_pip, p.valeur_pips)
  if (!calc) return null
  return { ...calc, engage: calc.unites * s.prix_entree }
}

/** Gain/perte en pips du palier max : R de référence × risque en pips
 *  (|entrée − SL| / taille du pip de l'asset). '' si params absents. */
/// Gain/perte du palier en POINTS MT5 (unité du broker : taille du point =
/// taille_pip / pip_to_points — décision 04/09 : MT5 raisonne en points).
function pointsPalier(s: Signal): string {
  const r = rReference(s)
  if (r === null) return ''
  const p = assetParams.liste.find(x => x.asset === s.asset)
  if (!p || p.taille_pip <= 0 || !p.pip_to_points || p.pip_to_points <= 0) return ''
  const risque = Math.abs(s.prix_entree - s.stop_loss)
  if (risque <= 0) return ''
  const pts = r * (risque / (p.taille_pip / p.pip_to_points))
  return `${pts >= 0 ? '+' : '−'}${Math.abs(Math.round(pts))} pts`
}

const prixStore = usePrixStore()

function icone(col: string): string {
  if (props.triColonne !== col) return '\u21c5'
  return props.triDir === 'asc' ? '\u2191' : '\u2193'
}

/** MFE des perdants : { [id]: { mfe_r, meilleur_prix } } — vide si non chargé. */
const mfeMap = computed<Record<string, { mfe_r: number | null; meilleur_prix: number | null }>>(() => props.mfe ?? {})
const profitMap = computed<Record<string, number>>(() => props.profits ?? {})

/** $ réel du trade (re-jeu capital) — la voix « résultat », à côté du R. */
function formatDollarsTrade(id: string): string {
  const d = profitMap.value[id]
  if (d === undefined) return ''
  return `${d >= 0 ? '+' : '−'}${Math.abs(d).toFixed(2)} $`
}

/** Lots recalculés : { [id]: lot } — vide si non chargé. */
const lotMap = computed<Record<string, number>>(() => props.lots ?? {})

/// Lot formaté : toujours 2 décimales maximum (règle 04/09).
function formatLot(v: number | undefined): string {
  if (v === undefined || v <= 0) return '—'
  return v.toFixed(2)
}

/** Palier max d'un trade clôturé (null si encore ouvert). Un ordre JAMAIS
 *  REMPLI n'a pas de palier — le trade n'a pas existé (l'ancien worker v1
 *  lui collait parfois un verdict au prix courant : on neutralise l'affichage
 *  du R, la ligne reste pour la traçabilité). */
function palierFerme(s: Signal): PalierMax['palier'] {
  if ((s.statut ?? '') !== 'Fermé') return null
  if (s.heure_entree === null || s.heure_entree === undefined) return 'Non rempli'
  return palierMax(s).palier
}

/** R DISTANCE du trade (niveau le plus lointain atteint — décision 23/09) :
 *  LE R affiché, il juge l'entrée et le placement des TP, indépendant de
 *  tout réglage de sortie. Le $ réel du trade vit juste à côté (profitMap).
 *  Null hors clôtures remplies. */
function rReference(s: Signal): number | null {
  if ((s.statut ?? '') !== 'Fermé') return null
  if (s.heure_entree === null || s.heure_entree === undefined) return null
  return s.r_distance ?? null
}

/** « Ouvert le » = REMPLISSAGE de l'ordre — la position n'existe qu'à partir
 *  de là (un signal émis à 21:04 et rempli à 23:52 est « ouvert » à 23:52,
 *  l'attente du retest n'est pas de la vie en position). L'émission part en
 *  info-bulle ; un ordre jamais rempli garde l'émission comme seule date. */
function titreOuverture(s: Signal): string {
  if (s.heure_entree === null || s.heure_entree === undefined) {
    return `Signal émis le ${formatDate(s.cree_le)} — ordre jamais rempli`
  }
  if (s.heure_entree === s.cree_le) return ''
  return `Émis le ${formatDate(s.cree_le)} · position ouverte le ${formatDate(s.heure_entree)}`
}

function classeConviction(c: number | null): string {
  if (c === null) return 'bg-gray-700 text-white'
  if (c >= 70) return 'bg-emerald-900 text-emerald-300 border border-emerald-600'
  if (c >= 50) return 'bg-yellow-900 text-yellow-300 border border-yellow-600'
  return 'bg-red-900 text-red-300 border border-red-600'
}

function classePrixActuelSignal(s: Signal, prix: number | null): string {
  if (!prix) return 'text-white'
  const long = s.direction === 'LONG'
  if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'text-red-400'
  if (s.take_profit[2] && (long ? prix >= s.take_profit[2] : prix <= s.take_profit[2])) return 'text-emerald-200'
  if (s.take_profit[1] && (long ? prix >= s.take_profit[1] : prix <= s.take_profit[1])) return 'text-emerald-300'
  return (long ? prix >= s.take_profit[0] : prix <= s.take_profit[0]) ? 'text-emerald-400' : 'text-blue-300'
}
</script>

<style scoped>
/* En-tête collant : la ligne de titres reste visible pendant le scroll
   (le conteneur qui défile est la section Historique de StrategyShell).
   Fond opaque = composite glass-card sur le bg-gray-900 de l'app ; filet
   inférieur en box-shadow — fiable avec border-collapse. */
thead th {
  position: sticky;
  top: 0;
  z-index: 10;
  background: #1d2332;
  box-shadow: inset 0 -1px 0 0 rgba(255, 255, 255, 0.1);
}

/* Badges (verdict KDJ, état) — mêmes teintes que SignauxTableau. */
.badge { @apply text-xs font-bold px-2 py-0.5 rounded-full; }
.badge-green { @apply bg-emerald-900/60 text-emerald-300; }
.badge-red   { @apply bg-red-900/60 text-red-300; }
.badge-blue  { @apply bg-blue-900/60 text-blue-300; }
.badge-gray  { @apply bg-gray-700/60 text-white; }
.badge-yellow { @apply bg-yellow-900/60 text-yellow-300; }
</style>
