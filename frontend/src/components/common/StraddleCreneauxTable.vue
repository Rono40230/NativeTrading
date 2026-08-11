<template>
  <div class="glass-card overflow-x-auto">
    <div class="p-4 border-b border-white/10 flex items-center justify-between">
      <h2 class="text-sm font-semibold text-white">📊 Créneaux identifiés</h2>
      <span class="text-xs text-gray-400">{{ creneauxFiltres.length }} créneau(x)</span>
    </div>

    <!-- Filtres statut -->
    <div class="p-3 border-b border-white/10 flex gap-2 flex-wrap">
      <button
        v-for="f in FILTRES_STATUT"
        :key="f.val"
        class="px-3 py-1 text-xs rounded-full border transition-all"
        :class="filtreStatut === f.val
          ? 'bg-yellow-600/30 border-yellow-500/50 text-yellow-300'
          : 'border-white/10 text-gray-400 hover:border-white/30'"
        @click="$emit('update:filtreStatut', f.val)"
      >{{ f.label }}</button>
    </div>

    <div v-if="chargementListe" class="p-8 text-center text-gray-500">Chargement…</div>

    <div v-else-if="creneauxFiltres.length === 0" class="p-10 text-center text-gray-500">
      <p class="text-3xl mb-2">⚡</p>
      <p class="text-sm">Aucun créneau — lancez une analyse LLM pour démarrer.</p>
    </div>

    <table v-else class="w-full text-sm">
      <thead>
        <tr class="border-b border-white/10 text-xs text-gray-400 uppercase">
          <th class="text-left px-4 py-3">Asset</th>
          <th class="text-left px-4 py-3">Jour</th>
          <th class="text-center px-4 py-3">Pic volatilité</th>
          <th class="text-center px-4 py-3">ATR ×</th>
          <th class="text-center px-4 py-3">Fréquence</th>
          <th class="text-center px-4 py-3">Conviction</th>
          <th class="text-left px-4 py-3">Raison LLM</th>
          <th class="text-center px-4 py-3">Statut</th>
          <th class="px-4 py-3"></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="c in creneauxFiltres"
          :key="c.id"
          class="border-b border-white/5 hover:bg-white/5 transition-colors"
        >
          <td class="px-4 py-3 font-bold text-white">{{ c.asset }}</td>
          <td class="px-4 py-3 text-gray-300">{{ nomJour(c.jour_semaine) }}</td>
          <td class="px-4 py-3 text-center text-xs">
            <template v-if="chargementPrecision[c.id]">⏳</template>
            <template v-else-if="c.timing_optimal">
              <span class="text-orange-400 font-mono font-bold text-sm">{{ c.timing_optimal }}</span>
              <br /><span class="text-gray-500">{{ c.fenetre_entree }}</span>
              <br /><button class="text-gray-600 hover:text-blue-400 text-xs mt-0.5" @click="$emit('chargerPrecision', c)">↻</button>
            </template>
            <button v-else class="text-blue-400 hover:underline" @click="$emit('chargerPrecision', c)">⏱ Analyser</button>
          </td>
          <td class="px-4 py-3 text-center">
            <span :class="couleurAtr(c.atr_moyen)" class="font-semibold">
              {{ c.atr_moyen != null ? c.atr_moyen.toFixed(2) + '×' : '—' }}
            </span>
          </td>
          <td class="px-4 py-3 text-center text-gray-300">
            {{ c.frequence != null ? (c.frequence * 100).toFixed(0) + '%' : '—' }}
          </td>
          <td class="px-4 py-3 text-center">
            <span :class="couleurConviction(c.llm_conviction)" class="font-bold">
              {{ c.llm_conviction ?? '—' }}
            </span>
          </td>
          <td class="px-4 py-3 text-gray-400 text-xs max-w-sm truncate" :title="c.llm_raison ?? ''">
            {{ c.llm_raison ?? '—' }}
          </td>
          <td class="px-4 py-3 text-center">
            <AppSelect
              :model-value="c.statut"
              :options="OPTIONS_STATUT"
              @update:model-value="(v) => $emit('changerStatut', c, String(v))"
            />
          </td>
          <td class="px-4 py-3 text-center">
            <RouterLink
              :to="`/pnl?asset=${c.asset}&timing=${c.timing_optimal ?? ''}&jour=${c.jour_semaine ?? ''}&id=${c.id}`"
              class="text-xs text-blue-400 hover:underline"
              :class="!c.timing_optimal ? 'opacity-40 pointer-events-none' : ''"
              :title="c.timing_optimal ? '' : 'Lancez ⏱ Analyser pour obtenir le timing précis'"
            >🧪 Tester</RouterLink>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import AppSelect from '@/components/common/AppSelect.vue'
import type { StraddleCreneau } from '@/services/api.types'

type FiltreStatut = 'tous' | 'a_tester' | 'valide' | 'invalide'

const props = defineProps<{
  creneaux: StraddleCreneau[]
  asset: string
  filtreStatut: FiltreStatut
  chargementListe: boolean
  chargementPrecision: Record<number, boolean>
}>()

defineEmits<{
  'update:filtreStatut': [val: FiltreStatut]
  'changerStatut': [creneau: StraddleCreneau, statut: string]
  'chargerPrecision': [creneau: StraddleCreneau]
}>()

const FILTRES_STATUT = [
  { val: 'tous' as FiltreStatut,     label: 'Tous' },
  { val: 'a_tester' as FiltreStatut, label: '🔍 À tester' },
  { val: 'valide' as FiltreStatut,   label: '✅ Validés' },
  { val: 'invalide' as FiltreStatut, label: '❌ Invalides' },
]

const OPTIONS_STATUT = [
  { label: '🔍 À tester', value: 'a_tester' },
  { label: '✅ Validé',   value: 'valide' },
  { label: '❌ Invalide', value: 'invalide' },
]

const JOURS = ['Lundi', 'Mardi', 'Mercredi', 'Jeudi', 'Vendredi', 'Samedi', 'Dimanche']

const creneauxFiltres = computed(() => {
  const liste = props.creneaux.filter(c => c.asset === props.asset)
  return props.filtreStatut === 'tous' ? liste : liste.filter(c => c.statut === props.filtreStatut)
})

function nomJour(jour: number | null): string {
  if (jour == null) return 'Tous'
  return JOURS[jour] ?? `J${jour}`
}

function couleurAtr(v: number | null): string {
  if (v == null) return 'text-gray-500'
  if (v >= 1.8) return 'text-red-400'
  if (v >= 1.4) return 'text-yellow-400'
  return 'text-gray-400'
}

function couleurConviction(v: number | null): string {
  if (v == null) return 'text-gray-500'
  if (v >= 80) return 'text-emerald-400'
  if (v >= 65) return 'text-yellow-400'
  return 'text-red-400'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
