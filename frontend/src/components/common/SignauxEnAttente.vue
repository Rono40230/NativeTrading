<template>
  <div class="flex flex-col gap-1">
    <div class="text-[10px] uppercase text-white font-semibold tracking-wider mb-0.5">
      Ordres posés ({{ enAttente.length }})
    </div>
    <div v-if="!enAttente.length" class="text-[11px] text-white">
      Aucun ordre en attente de remplissage
    </div>
    <div
      v-for="s in enAttente"
      :key="s.id"
      class="flex items-center gap-1.5 bg-white/5 rounded px-1.5 py-1 text-[11px]"
      :title="titreLigne(s)"
    >
      <span :class="s.direction?.toUpperCase() === 'LONG' ? 'text-emerald-400' : 'text-red-400'">
        {{ s.direction?.toUpperCase() === 'LONG' ? '📈' : '📉' }}
      </span>
      <span class="font-semibold text-white truncate">{{ s.asset }}</span>
      <span class="text-white">{{ s.timeframe }}</span>
      <span class="ml-auto font-mono text-amber-300 whitespace-nowrap">{{ formatNombre(s.prix_entree) }}</span>
      <span v-if="compteARebours(s) !== null" class="text-white font-mono text-[10px] whitespace-nowrap">
        {{ compteARebours(s) }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Signal } from '@/services/api.service'
import { formatNombre } from '@/composables/useSignalFormat'

const props = defineProps<{
  /** Signaux actifs (non clôturés) de la stratégie — le composant filtre. */
  signaux: Signal[]
  strategie: 'SMC' | 'straddle' | 'Rockets'
}>()

/// Ordres posés jamais remplis : SMC = entrée non touchée (heure_entree
/// vide) ; Straddle = annonce pas encore arrivée (heure E future) ;
/// Rockets = aucun (position ouverte dès le signal).
const enAttente = computed(() =>
  props.signaux.filter(s => {
    if (s.statut === 'Fermé' || s.verdict !== null) return false
    if (props.strategie === 'SMC') return s.heure_entree === null || s.heure_entree === undefined
    if (props.strategie === 'straddle')
      return (s.heure_entree ?? 0) > Math.floor(Date.now() / 1000)
    return false
  }),
)

function titreLigne(s: Signal): string {
  if (props.strategie === 'straddle') {
    return `Passe armée — entrée des 2 jambes à l'heure de l'annonce (${new Date((s.heure_entree ?? 0) * 1000).toLocaleString('fr-FR')})`
  }
  return 'Annoncé : ordre en limite — se remplit si le prix revient toucher l\'entrée'
}

function compteARebours(s: Signal): string | null {
  if (props.strategie !== 'straddle' || !s.heure_entree) return null
  const d = s.heure_entree - Math.floor(Date.now() / 1000)
  if (d <= 0) return null
  const h = Math.floor(d / 3600)
  const m = Math.floor((d % 3600) / 60)
  return h > 0 ? `E dans ${h}h${String(m).padStart(2, '0')}` : `E dans ${m} min`
}
</script>
