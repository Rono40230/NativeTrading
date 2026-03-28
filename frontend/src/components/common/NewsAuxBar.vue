<template>
  <div class="flex items-center gap-2 flex-wrap shrink-0 mb-3">
    <!-- Badge Fear & Greed Index -->
    <span
      v-if="fg"
      class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold cursor-default"
      :class="badgeFg"
      :title="`Fear & Greed Index BTC — ${fg.label}`"
    >
      {{ iconeFg }} Fear &amp; Greed · {{ fg.valeur }}/100
    </span>

    <!-- Prochain événement macro High-impact -->
    <span
      v-if="prochainEvent"
      class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold"
      :class="['border-red-500/30 bg-red-500/10 text-red-400', { 'animate-pulse': dansMoins30min }]"
      :title="prochainEvent.titre"
    >
      ⚠️ {{ prochainEvent.devise }} · {{ countdown(prochainEvent.date_heure) }}
    </span>

    <!-- E.1 — Bandeau suspension SMC Directionnel si événement dans ≤30 min -->
    <span
      v-if="dansMoins30min"
      class="inline-flex items-center gap-1 rounded-full border border-yellow-500/40 bg-yellow-500/10 px-2 py-0.5 text-[10px] font-bold text-yellow-300"
      :title="`SMC Directionnel suspendu automatiquement — ${prochainEvent?.titre}`"
    >
      ⏸ SMC suspendu
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { apiService } from '@/services/api.service'
import { useFearGreed } from '@/composables/useFearGreed'
import type { AnnonceCalendrier } from '@/services/api.types'

const { data: fg } = useFearGreed()

const annonces = ref<AnnonceCalendrier[]>([])

onMounted(async () => {
  annonces.value = await apiService.obtenirCalendrier(2)
})

/** Premier événement USD/EUR High-impact non passé dans les prochaines 24h */
const prochainEvent = computed<AnnonceCalendrier | undefined>(() => {
  const maintenant = Date.now()
  const dans24h = maintenant + 24 * 3_600_000
  return annonces.value.find((a) => {
    const ts = new Date(a.date_heure).getTime()
    return !a.est_passe && ts > maintenant && ts <= dans24h && a.impact === 'High'
  })
})

const dansMoins30min = computed(() => {
  if (!prochainEvent.value) return false
  const diff = new Date(prochainEvent.value.date_heure).getTime() - Date.now()
  return diff > 0 && diff < 30 * 60_000
})

function countdown(iso: string): string {
  const diff = new Date(iso).getTime() - Date.now()
  if (diff <= 0) return 'en cours'
  const min = Math.floor(diff / 60_000)
  if (min < 60) return `dans ${min}min`
  const h = Math.floor(min / 60)
  const m = min % 60
  return `dans ${h}h${m > 0 ? ` ${m}min` : ''}`
}

const badgeFg = computed(() => {
  if (!fg.value) return ''
  switch (fg.value.categorie) {
    case 'extreme_fear': return 'border-red-600/50 bg-red-600/20 text-red-400'
    case 'fear':         return 'border-orange-500/40 bg-orange-500/10 text-orange-400'
    case 'neutral':      return 'border-yellow-500/30 bg-yellow-500/10 text-yellow-400'
    case 'greed':        return 'border-emerald-500/40 bg-emerald-500/10 text-emerald-400'
    case 'extreme_greed': return 'border-emerald-400/60 bg-emerald-400/15 text-emerald-300'
    default:             return 'border-slate-600/30 bg-white/5 text-slate-400'
  }
})

const iconeFg = computed(() => {
  switch (fg.value?.categorie) {
    case 'extreme_fear': return '🩸'
    case 'fear':         return '😨'
    case 'neutral':      return '😐'
    case 'greed':        return '😏'
    case 'extreme_greed': return '🤑'
    default:             return '—'
  }
})
</script>
