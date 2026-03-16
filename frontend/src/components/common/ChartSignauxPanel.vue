<template>
  <div class="glass-card p-3 text-xs">
    <!-- Barre unique : sources | Min | Afficher | nb derniers | compteur -->
    <div class="flex items-center gap-2 flex-wrap">

      <!-- Sources -->
      <button
        v-for="src in SOURCES"
        :key="src"
        @click="toggleSource(src)"
        :class="[
          'px-2 py-0.5 rounded border transition-colors',
          filtre.sources.includes(src)
            ? 'bg-purple-500/20 border-purple-500/60 text-purple-300'
            : 'border-white/10 text-gray-500',
        ]"
      >{{ src }}</button>

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Min force -->
      <button
        v-for="f in FORCES"
        :key="f"
        @click="filtre.forceMin = f"
        :class="[
          'px-2 py-0.5 rounded border transition-colors',
          filtre.forceMin === f
            ? 'bg-blue-500/30 border-blue-500/60 text-white'
            : 'border-white/10 text-gray-400 hover:border-white/30',
        ]"
      >{{ FORCE_LABEL[f] }} {{ f }}</button>

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Afficher directions -->
      <label class="flex items-center gap-1 cursor-pointer">
        <input type="checkbox" v-model="filtre.afficherBullish" class="accent-emerald-500" />
        <span class="text-emerald-400">Bullish</span>
      </label>
      <label class="flex items-center gap-1 cursor-pointer">
        <input type="checkbox" v-model="filtre.afficherBearish" class="accent-red-500" />
        <span class="text-red-400">Bearish</span>
      </label>
      <label class="flex items-center gap-1 cursor-pointer">
        <input type="checkbox" v-model="filtre.afficherNeutre" class="accent-gray-400" />
        <span class="text-gray-400">Neutre</span>
      </label>

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Nb derniers signaux -->
      <button
        v-for="n in NB_OPTIONS"
        :key="n"
        @click="filtre.nbSignaux = filtre.nbSignaux === n ? 0 : n"
        :class="[
          'px-2 py-0.5 rounded border transition-colors',
          filtre.nbSignaux === n
            ? 'bg-blue-500/30 border-blue-500/60 text-white'
            : 'border-white/10 text-gray-400 hover:border-white/30',
        ]"
      >{{ n }}</button>
      <span class="text-gray-500">{{ filtre.nbSignaux === 0 ? 'tous' : 'derniers' }}</span>

      <!-- Compteur -->
      <span class="ml-auto text-gray-500">{{ signaux_filtres.length }} / {{ signaux.length }}</span>

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Toggle SL/TP -->
      <button
        @click="filtre.afficherSlTp = !filtre.afficherSlTp"
        :class="[
          'px-2 py-0.5 rounded border transition-colors',
          filtre.afficherSlTp
            ? 'bg-amber-500/20 border-amber-500/60 text-amber-300'
            : 'border-white/10 text-gray-500',
        ]"
      >SL/TP</button>
    </div>

    <!-- Signaux (curseur + récents) -->
    <div v-if="signaux_curseur.length > 0 || recents.length > 0" class="border-t border-white/10 mt-2 pt-2 space-y-1">
      <template v-if="signaux_curseur.length > 0">
        <div class="text-yellow-400 font-semibold">📍 Curseur :</div>
        <div
          v-for="s in signaux_curseur"
          :key="s.source + s.type_signal"
          :class="['px-2 py-1 rounded border', classBadge(s.direction)]"
        >
          <div><span class="font-semibold">{{ s.source }}</span> — {{ s.description }}<span class="ml-1 opacity-60">{{ FORCE_LABEL[s.force] }}</span></div>
          <div v-if="conseilSignal(s.type_signal)" class="mt-0.5 text-xs opacity-70 italic">💡 {{ conseilSignal(s.type_signal) }}</div>
        </div>
      </template>
      <template v-if="recents.length > 0">
        <div class="text-gray-500">Récents :</div>
        <div
          v-for="s in recents"
          :key="s.source + s.type_signal + s.timestamp"
          :class="['px-2 py-1 rounded border', classBadge(s.direction)]"
        >
          <div><span class="font-semibold">{{ s.source }}</span> — {{ s.description }}<span class="ml-1 opacity-60">{{ FORCE_LABEL[s.force] }}</span></div>
          <div v-if="conseilSignal(s.type_signal)" class="mt-0.5 text-xs opacity-70 italic">💡 {{ conseilSignal(s.type_signal) }}</div>
        </div>
      </template>
    </div>

    <div v-if="signaux.length === 0" class="text-gray-600 italic mt-1">
      Aucun signal détecté sur cette période
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import {
  FORCE_LABEL,
  filtreDefaut,
  filtrerSignaux,
  type SignalIndicateur,
  type NiveauForce,
  type FiltreSignaux,
} from '@/composables/chartSignauxTypes'

const FORCES: NiveauForce[] = ['faible', 'moyen', 'fort']
const SOURCES = ['EMA', 'RSI', 'MACD', 'Bollinger', 'ATR', 'Combiné']
const NB_OPTIONS = [5, 10, 40]

const props = defineProps<{
  signaux: SignalIndicateur[]
  timestampCurseur: number | null
}>()

const emit = defineEmits<{
  (e: 'update:filtre', f: FiltreSignaux): void
}>()

const filtre = reactive<FiltreSignaux>(filtreDefaut())

const signaux_filtres = computed(() => filtrerSignaux(props.signaux, filtre))

const signaux_curseur = computed(() =>
  props.timestampCurseur === null
    ? []
    : signaux_filtres.value.filter((s) => s.timestamp === props.timestampCurseur),
)

const recents = computed(() =>
  [...signaux_filtres.value].sort((a, b) => b.timestamp - a.timestamp).slice(0, 5),
)

function toggleSource(src: string) {
  const idx = filtre.sources.indexOf(src)
  if (idx >= 0) filtre.sources.splice(idx, 1)
  else filtre.sources.push(src)
}

const CONSEILS: Record<string, string> = {
  golden_cross: `Envisagez un buy sur pullback vers l'EMA.`,
  death_cross: `Réduisez l'exposition buy, envisagez un sell sur rebond.`,
  survente_sortie: `Le vendeur s'épuise — cherchez un buy à confirmation.`,
  surachat_sortie: `Momentum haussier s'affaiblit — serrez le stop ou prenez profit sur le buy.`,
  mi_ligne_haussiere: `Tendance favorise les acheteurs, renforcez le buy en pullback.`,
  mi_ligne_baissiere: `Tendance favorise les vendeurs, allégez ou basculez en sell.`,
  croisement_haussier: `MACD croise à la hausse — signal d'entrée buy de court terme.`,
  croisement_baissier: `MACD croise à la baisse — signal de sortie buy ou d'entrée sell.`,
  zero_haussier: `MACD passe au-dessus de zéro — tendance haussière confirmée, favorisez le buy.`,
  zero_baissier: `MACD passe sous zéro — tendance baissière confirmée, favorisez le sell.`,
  touche_bande_basse: `Prix en zone de survente — possible rebond buy, attendez confirmation.`,
  touche_bande_haute: `Prix en zone de surachat — possible retournement sell, gérez le risque.`,
  cassure_basse: `Rupture baissière Bollinger — volatilité en hausse, trailing stop sur sell conseillé.`,
  cassure_haute: `Breakout haussier Bollinger — momentum fort sur buy, suivez avec stop serré.`,
  squeeze: `Contraction de volatilité — anticipez un mouvement directionnel imminent (buy ou sell).`,
  atr_spike: `Volatilité anormale — évitez d'entrer en position, attendez que la bougie se ferme.`,
  atr_compression: `ATR au plus bas — explosion imminente, positionnez-vous avant le breakout.`,
  boll_rsi_bull: `Double confluence : bande basse + oversold — buy avec stop sous la bande basse.`,
  boll_rsi_bear: `Double confluence : bande haute + overbought — sell avec stop au-dessus de la bande haute.`,
  squeeze_macd_bull: `Compression + MACD haussier — le breakout buy est imminent, entrez sur confirmation.`,
  squeeze_macd_bear: `Compression + MACD baissier — le breakout sell est imminent, entrez sur confirmation.`,
  atr_macd_bull: `Volatilité + momentum haussier alignés — tendance forte buy, trailing stop recommandé.`,
  atr_macd_bear: `Volatilité + momentum baissier alignés — tendance forte sell, trailing stop recommandé.`,
  ema_macd_bull: `EMA + MACD tous deux haussiers — buy en continuation, stop sous l'EMA.`,
  ema_macd_bear: `EMA + MACD tous deux baissiers — sell en continuation, stop au-dessus de l'EMA.`,
  cross_macd_bull: `Golden Cross confirmé par le MACD — signal buy majeur, taille de position normale.`,
  cross_macd_bear: `Death Cross confirmé par le MACD — signal sell majeur, taille de position normale.`,
}

function conseilSignal(type_signal: string): string {
  return CONSEILS[type_signal] ?? ''
}

function classBadge(direction: string): string {
  if (direction === 'bullish') return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300'
  if (direction === 'bearish') return 'border-red-500/30 bg-red-500/10 text-red-300'
  return 'border-gray-600/30 bg-gray-600/10 text-gray-400'
}

// Propagation du filtre au parent pour re-rendu des marqueurs
watch(filtre, () => emit('update:filtre', { ...filtre, sources: [...filtre.sources] }), {
  deep: true,
})
</script>
