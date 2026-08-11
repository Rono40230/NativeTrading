<template>
  <div class="rocket-bar px-4 py-2.5 flex flex-col gap-3 h-full min-h-0 overflow-hidden"
    v-bind="$attrs">
    <!-- Header -->
    <div class="flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-orange-400">
        🚀 ROCKETS ÉLIGIBLES
        <span v-if="signaux.length > 0"
          class="ml-1 text-[9px] font-normal text-orange-300 normal-case tracking-normal">{{ signaux.length }}
          positions éligibles</span>
      </p>
      <span class="text-[9px] text-gray-600">{{ countdown }}s ▸</span>
    </div>

    <!-- Chargement -->
    <template v-if="chargement">
      <span class="text-[10px] text-orange-400 animate-pulse">Scan {{ progression }}%</span>
    </template>

    <!-- Cartes éligibles -->
    <div v-else-if="signaux.length > 0" class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-2 relative z-20 pr-1">
      <RocketCard v-for="s in signaux" :key="s.symbol" :s="s" @click.stop="onCardClick($event, s)" />
    </div>

    <!-- Vide -->
    <template v-else>
      <span class="text-[10px] text-gray-500 italic">Aucun candidat</span>
    </template>

    <!-- Footer -->
    <div class="mt-auto shrink-0">
    </div>
  </div>

  <RocketTooltip
    :signal="hovered"
    :pos="pos"
    :selected-tf="selectedTF"
    :sparkline="sparklineActive"
    :couleur="couleurSparkline"
    :variation="variationTF"
    :tf-configs="TF_CONFIGS"
    @choose-tf="choisirTF"
  />
  <RocketsOpportunitesModal :visible="modalOuverte" :signaux="signaux" @close="modalOuverte = false" />
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'
import RocketTooltip from '@/components/common/RocketTooltip.vue'
import RocketsOpportunitesModal from '@/components/common/RocketsOpportunitesModal.vue'
import RocketCard from '@/components/common/RocketCard.vue'

const props = defineProps<{
  signaux: SignalRocket[]
  totalCandidats: number
  chargement: boolean
  erreur: boolean
  progression: number
  derniereMAJ: number
}>()

const labelCandidats = computed(() =>
  props.chargement
    ? `Scan en cours… ${props.progression}%`
    : props.signaux.length > 0
      ? `${props.signaux.length} signaux · ${props.totalCandidats} paires analysées`
      : `${props.totalCandidats > 0 ? props.totalCandidats : '…'} paires Binance`
)

function icone(phase: PhaseRocket): string {
  if (phase === 'breakout') return '🚀'
  if (phase === 'prelancement') return '⚡'
  return '🌀'
}



const TF_CONFIGS = [
  { label: '1H', interval: null as string | null, limit: 0 },
  { label: '4H', interval: '5m' as string | null, limit: 48 },
  { label: 'D1', interval: '1h' as string | null, limit: 24 },
  { label: 'W1', interval: '4h' as string | null, limit: 42 },
]

const hoveredSymbol = ref<string | null>(null)
const hovered = computed(() => hoveredSymbol.value ? props.signaux.find(s => s.symbol === hoveredSymbol.value) ?? null : null)
const modalOuverte = ref(false)
const topSignal = computed(() =>
  props.signaux.length > 0 ? [...props.signaux].reduce((best, s) => s.score > best.score ? s : best) : null
)
const top5 = computed(() =>
  [...props.signaux].sort((a, b) => b.score - a.score).slice(0, 5)
)
const pos = ref({ x: 0, y: 0 })
const selectedTF = ref('1H')
const sparklineTF = ref<number[]>([])

const sparklineActive = computed(() =>
  selectedTF.value === '1H' ? (hovered.value?.closes ?? []) : sparklineTF.value
)
const couleurSparkline = computed(() => {
  const s = sparklineActive.value
  if (s.length < 2) return '#10b981'
  return s.at(-1)! >= s[0] ? '#10b981' : '#ef4444'
})

const variationTF = computed(() => {
  const s = sparklineActive.value
  if (s.length < 2) return hovered.value?.change1h ?? 0
  return ((s.at(-1)! - s[0]) / s[0]) * 100
})

async function fetchSparklineTF(ticker: string, interval: string, limit: number) {
  sparklineTF.value = []
  try {
    const data = await apiService.getMarcheKlines(ticker, interval, limit)
    sparklineTF.value = data.map(k => parseFloat(k[4] as string))
  } catch { /* silencieux */ }
}

function choisirTF(tf: { label: string; interval: string | null; limit: number }) {
  selectedTF.value = tf.label
  if (hovered.value && tf.interval) fetchSparklineTF(hovered.value.ticker, tf.interval, tf.limit)
}

function onCardClick(event: MouseEvent, s: SignalRocket) {
  if (hoveredSymbol.value === s.symbol) { hoveredSymbol.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = rect.left + rect.width / 2
  const clampX = Math.max(124, Math.min(window.innerWidth - 124, rawX))
  pos.value = { x: clampX, y: rect.top - 8 }
  selectedTF.value = '1H'
  sparklineTF.value = []
  hoveredSymbol.value = s.symbol
}

function fermerTooltip() { hoveredSymbol.value = null }

// Countdown 30s : se réinitialise à chaque fin de poll (silencieux ou non)
const SCAN_S = 30
const countdown = ref(SCAN_S)
let tickInterval: ReturnType<typeof setInterval> | null = null
watch(() => props.derniereMAJ, () => { countdown.value = SCAN_S })

onMounted(() => {
  document.addEventListener('click', fermerTooltip)
  tickInterval = setInterval(() => { if (countdown.value > 0) countdown.value-- }, 1000)
})
onUnmounted(() => {
  document.removeEventListener('click', fermerTooltip)
  if (tickInterval) clearInterval(tickInterval)
})
</script>

<style scoped>
.rocket-bar {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
