<template>
  <Transition name="fade">
    <div
      v-if="annonce"
      class="eco-cal-tooltip"
      :style="{ left: `${x + 14}px`, top: `${y}px`, transform: 'translateY(-100%) translateY(-12px)' }"
    >
      <!-- En-tête : devise + impact -->
      <div class="flex items-center gap-2 mb-1.5">
        <span class="text-base leading-none">{{ drapeau }}</span>
        <span class="text-xs font-bold text-white leading-none">{{ annonce.devise }}</span>
        <span
          class="ml-auto text-[10px] font-semibold px-1.5 py-0.5 rounded"
          :class="annonce.impact === 'High'
            ? 'bg-red-500/20 text-red-400'
            : 'bg-orange-500/20 text-orange-400'"
        >{{ annonce.impact }}</span>
      </div>

      <!-- Titre -->
      <p class="text-[11px] text-white leading-snug mb-2">{{ annonce.titre }}</p>

      <!-- Heure -->
      <div class="flex items-center gap-2 text-[10px] text-slate-400 mb-1.5">
        <span>🕐 {{ heureLocale }}</span>
        <span class="text-slate-600">·</span>
        <span>{{ heureUTC }} UTC</span>
      </div>

      <!-- Précédent / Prévision -->
      <div class="flex gap-4 text-[10px] text-slate-400">
        <span>Préc: <span class="text-white">{{ annonce.precedent ?? '—' }}</span></span>
        <span>Prévis: <span class="text-white">{{ annonce.prevision ?? '—' }}</span></span>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AnnonceCalendrier } from '@/services/api.types'
import { formatParis } from '@/utils/date'

const DRAPEAUX: Record<string, string> = {
  USD: '🇺🇸', EUR: '🇪🇺', GBP: '🇬🇧', JPY: '🇯🇵',
  CAD: '🇨🇦', AUD: '🇦🇺', CHF: '🇨🇭', NZD: '🇳🇿',
  CNY: '🇨🇳', CNH: '🇨🇳',
}

const props = defineProps<{
  annonce: AnnonceCalendrier | null
  x: number
  y: number
}>()

const drapeau = computed(() =>
  props.annonce ? (DRAPEAUX[props.annonce.devise] ?? props.annonce.devise.slice(0, 2)) : ''
)

const heureLocale = computed(() =>
  props.annonce
    ? formatParis(new Date(props.annonce.date_heure), { hour: '2-digit', minute: '2-digit' })
    : ''
)

const heureUTC = computed(() => {
  if (!props.annonce) return ''
  const d = new Date(props.annonce.date_heure)
  return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`
})
</script>

<style scoped>
.eco-cal-tooltip {
  position: absolute;
  z-index: 50;
  min-width: 200px;
  max-width: 260px;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(10, 14, 39, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(8px);
  pointer-events: none;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
