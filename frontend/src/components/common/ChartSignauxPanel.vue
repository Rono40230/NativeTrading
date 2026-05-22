<template>
  <div class="glass-card p-3 text-xs">
    <!-- Barre unique : sources | Min | Afficher | nb derniers | compteur -->
    <div class="flex items-center gap-2 flex-wrap">

      <span class="text-gray-400 whitespace-nowrap">Afficher et trier les signaux</span>
      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Sources -->
      <!-- "Tous" actif quand toutes les sources sont sélectionnées -->
      <button @click="filtre.sources.length === SOURCES.length ? filtre.sources = [] : filtre.sources = [...SOURCES]"
        :class="[
          'px-2 py-0.5 rounded border transition-colors',
          filtre.sources.length === SOURCES.length
            ? 'bg-blue-500/30 border-blue-500/60 text-white'
            : 'border-white/10 text-gray-500 hover:border-white/30',
        ]">Tous</button>
      <button v-for="src in SOURCES" :key="src" @click="toggleSource(src)" :class="[
        'px-2 py-0.5 rounded border transition-colors',
        filtre.sources.includes(src)
          ? 'bg-purple-500/20 border-purple-500/60 text-purple-300'
          : 'border-white/10 text-gray-500',
      ]">{{ src }}</button>

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Min force -->
      <button v-for="f in FORCES" :key="f" @click="filtre.forceMin = f" :class="[
        'px-2 py-0.5 rounded border transition-colors',
        filtre.forceMin === f
          ? 'bg-blue-500/30 border-blue-500/60 text-white'
          : 'border-white/10 text-gray-400 hover:border-white/30',
      ]">{{ FORCE_LABEL[f] }} {{ f }}</button>

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

      <span class="w-px h-4 bg-white/20 mx-1" />

      <!-- Nb derniers signaux -->
      <button v-for="n in NB_OPTIONS" :key="n" @click="filtre.nbSignaux = filtre.nbSignaux === n ? 0 : n" :class="[
        'px-2 py-0.5 rounded border transition-colors',
        filtre.nbSignaux === n
          ? 'bg-blue-500/30 border-blue-500/60 text-white'
          : 'border-white/10 text-gray-400 hover:border-white/30',
      ]">{{ n }}</button>

      <!-- Bouton Analyse SMC (text-first, Phase 2.1) -->
      <div class="ml-auto flex items-center gap-1">
        <button
          class="h-7 px-3 rounded border transition-colors bg-purple-600/20 border-purple-500/30 text-purple-300 hover:bg-purple-600/30 disabled:opacity-40"
          :disabled="analyseEnCours" @click="$emit('analyser')">{{ analyseEnCours ? '🔍 Analyse...' : '🔍 Analyse SMC par l\'IA' }}</button>
      </div>


    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import {
  FORCE_LABEL,
  TOUTES_SOURCES,
  filtreDefaut,
  filtrerSignaux,
  type SignalIndicateur,
  type NiveauForce,
  type FiltreSignaux,
} from '@/composables/chartSignauxTypes'
const FORCES: NiveauForce[] = ['moyen', 'fort']
const SOURCES = [...TOUTES_SOURCES]
const NB_OPTIONS = [5, 10, 40]

const props = defineProps<{
  signaux: SignalIndicateur[]
  analyseEnCours?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:filtre', f: FiltreSignaux): void
  (e: 'analyser'): void
}>()

const filtre = reactive<FiltreSignaux>(filtreDefaut())

const signaux_filtres = computed(() => filtrerSignaux(props.signaux, filtre))

function toggleSource(src: string) {
  const idx = filtre.sources.indexOf(src)
  if (idx >= 0) filtre.sources.splice(idx, 1)
  else filtre.sources.push(src)
}

// Propagation du filtre au parent pour re-rendu des marqueurs
watch(filtre, () => emit('update:filtre', { ...filtre, sources: [...filtre.sources] }), {
  deep: true,
})
</script>
