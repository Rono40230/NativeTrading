<template>
  <table class="w-full text-sm">
    <thead>
      <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
        <th class="px-3 py-3 text-left">#</th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'asset')">Asset <span class="tri-icone">{{ icone('asset') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'timeframe')">TF / Phase <span class="tri-icone">{{ icone('timeframe') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'direction')">Direction <span class="tri-icone">{{ icone('direction') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'score')">Score <span class="tri-icone">{{ icone('score') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'prix_entree')">Entrée <span class="tri-icone">{{ icone('prix_entree') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'stop_loss')">SL <span class="tri-icone">{{ icone('stop_loss') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp1')">TP1 <span class="tri-icone">{{ icone('tp1') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp2')">TP2 <span class="tri-icone">{{ icone('tp2') }}</span></th>
        <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'tp3')">TP3 <span class="tri-icone">{{ icone('tp3') }}</span></th>
        <th v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right">Prix actuel</th>
        <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'prix_verdict')">Sortie <span class="tri-icone">{{ icone('prix_verdict') }}</span></th>
        <th class="px-3 py-3 text-center">IA</th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'r_reference')">Palier max <span class="tri-icone">{{ icone('r_reference') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'strategie')">Stratégie <span class="tri-icone">{{ icone('strategie') }}</span></th>
        <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'cree_le')">Ouvert le <span class="tri-icone">{{ icone('cree_le') }}</span></th>
        <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="$emit('trier-par', 'ferme_le')">Fermé le <span class="tri-icone">{{ icone('ferme_le') }}</span></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="(s, i) in signaux" :key="s.id" class="border-b border-white/5 hover:bg-white/5 transition-colors">
        <td class="px-3 py-3 text-gray-500">{{ i + 1 }}</td>
        <td class="px-3 py-3 font-semibold text-white">{{ s.asset }}</td>
        <td class="px-3 py-3 text-gray-400">{{ s.timeframe }}</td>
        <td class="px-3 py-3">
          <span class="badge" :class="s.direction?.toUpperCase() === 'LONG' ? 'badge-green' : 'badge-red'">{{ s.direction }}</span>
        </td>
        <td class="px-3 py-3 text-right font-mono text-gray-300">{{ s.score.toFixed(0) }}</td>
        <td class="px-3 py-3 text-right font-mono text-white">{{ formatNombre(s.prix_entree) }}</td>
        <td class="px-3 py-3 text-right font-mono text-red-400">{{ formatNombre(s.stop_loss) }}</td>
        <td class="px-3 py-3 text-right font-mono text-emerald-400">{{ formatNombre(s.take_profit[0]) }}</td>
        <td class="px-3 py-3 text-right font-mono text-emerald-300">{{ s.take_profit[1] ? formatNombre(s.take_profit[1]) : '—' }}</td>
        <td class="px-3 py-3 text-right font-mono text-emerald-200">{{ s.take_profit[2] ? formatNombre(s.take_profit[2]) : '—' }}</td>
        <td v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right font-mono" :class="classePrixActuelSignal(s, prixStore.getPrix(s.asset))">{{ prixStore.getPrix(s.asset) !== null ? formatNombre(prixStore.getPrix(s.asset)!) : '—' }}</td>
        <!-- Sortie = information secondaire (gestion d'exécution), le R de
             référence vit dans la colonne Palier max. -->
        <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right">
          <div class="flex flex-col items-end leading-tight">
            <span v-if="calculerR(s) !== null" :class="classeR(calculerR(s))" class="text-xs">{{ formatR(calculerR(s)) }}</span>
            <span class="font-mono text-gray-500 text-xs">{{ s.prix_verdict ? formatNombre(s.prix_verdict) : '—' }}</span>
          </div>
        </td>
        <td class="px-3 py-3 text-center"><span v-if="s.llm_conviction !== null" class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help" :class="classeConviction(s.llm_conviction)" :title="s.llm_raison ?? ''">{{ s.llm_conviction }}</span><span v-else class="text-gray-700 text-xs">—</span></td>
        <td class="px-3 py-3">
          <div class="flex flex-col gap-0.5">
            <div class="flex items-center gap-2">
              <span v-if="palierFerme(s)" class="badge" :class="classePalierMax(palierFerme(s))">{{ labelPalierMax(palierFerme(s)) }}</span>
              <span v-else class="badge" :class="classeEtatSignal(s)" :title="titreEtatSignal(s)">{{ labelEtatSignal(s) }}</span>
              <span v-if="rReference(s) !== null" :class="classeR(rReference(s))" class="text-xs">{{ formatR(rReference(s)) }}</span>
            </div>
            <!-- MFE des perdants : l'excursion favorable avant le SL juge le
                 placement des niveaux (frôler TP1 puis claquer = info clé). -->
            <span v-if="palierFerme(s) === 'SL' && mfeMap[s.id] !== undefined"
                  class="text-amber-400 text-xs cursor-help"
                  title="Excursion favorable maximale avant le SL (calcul sur bougies M1)">{{ formatMfe(mfeMap[s.id]?.mfe_r ?? null) }}</span>
          </div>
        </td>
        <td class="px-3 py-3 text-gray-400 text-xs">{{ s.strategie === 'SMC Directionnel' ? 'SMC' : s.strategie }}</td>
        <td class="px-3 py-3 text-gray-500 text-xs">{{ formatDate(s.cree_le) }}</td>
        <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-gray-500 text-xs">{{ s.ferme_le ? formatDate(s.ferme_le) : '—' }}</td>
      </tr>
    </tbody>
  </table>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Signal } from '@/services/api.service'
import { usePrixStore } from '@/stores/prix.store'
import {
  formatDate, formatNombre, classeEtatSignal, labelEtatSignal, titreEtatSignal,
  calculerR, formatR, classeR,
  palierMax, labelPalierMax, classePalierMax, formatMfe,
  type PalierMax,
} from '@/composables/useSignalFormat'

const props = defineProps<{
  signaux: Signal[]
  filtreStatut: 'en_cours' | 'cloturees' | ''
  triColonne: string
  triDir: 'asc' | 'desc'
  /** MFE des trades SL : { [id]: { mfe_r, meilleur_prix } } */
  mfe?: Record<string, { mfe_r: number | null; meilleur_prix: number | null }>
}>()

const emit = defineEmits<{
  'trier-par': [col: string]
}>()

const prixStore = usePrixStore()

function icone(col: string): string {
  if (props.triColonne !== col) return '\u21c5'
  return props.triDir === 'asc' ? '\u2191' : '\u2193'
}

/** MFE des perdants : { [id]: { mfe_r, meilleur_prix } } — vide si non chargé. */
const mfeMap = computed<Record<string, { mfe_r: number | null; meilleur_prix: number | null }>>(() => props.mfe ?? {})

/** Palier max d'un trade clôturé (null si encore ouvert). */
function palierFerme(s: Signal): PalierMax['palier'] {
  if ((s.statut ?? '') !== 'Fermé') return null
  return palierMax(s).palier
}

/** R de référence : palier → R (colonnes dominantes de la lecture d'entrée). */
function rReference(s: Signal): number | null {
  if ((s.statut ?? '') !== 'Fermé') return null
  return palierMax(s).rReference
}

function classeConviction(c: number | null): string {
  if (c === null) return 'bg-gray-700 text-gray-400'
  if (c >= 70) return 'bg-emerald-900 text-emerald-300 border border-emerald-600'
  if (c >= 50) return 'bg-yellow-900 text-yellow-300 border border-yellow-600'
  return 'bg-red-900 text-red-300 border border-red-600'
}

function classePrixActuelSignal(s: Signal, prix: number | null): string {
  if (!prix) return 'text-gray-400'
  const long = s.direction === 'LONG'
  if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'text-red-400'
  if (s.take_profit[2] && (long ? prix >= s.take_profit[2] : prix <= s.take_profit[2])) return 'text-emerald-200'
  if (s.take_profit[1] && (long ? prix >= s.take_profit[1] : prix <= s.take_profit[1])) return 'text-emerald-300'
  return (long ? prix >= s.take_profit[0] : prix <= s.take_profit[0]) ? 'text-emerald-400' : 'text-blue-300'
}
</script>
