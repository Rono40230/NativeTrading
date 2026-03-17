<template>
  <Teleport to="body">
    <div
      v-if="signal"
      class="sm-window"
      :style="{ top: posY + 'px', left: posX + 'px' }"
    >
      <!-- Header draggable -->
      <div class="sm-header" @mousedown="demarrerDrag">
        <div class="flex items-center gap-2">
          <span class="text-base leading-none">{{ dirIcon }}</span>
          <span class="font-semibold text-sm" :class="dirColor">{{ dirLabel }}</span>
          <span class="text-gray-500 text-xs">·</span>
          <span class="text-gray-300 text-xs">{{ signal.source }}</span>
          <span class="text-xs font-mono" :class="forceCouleur">{{ FORCE_LABEL[signal.force] }}</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-gray-600 text-xs select-none cursor-move">⠿ déplacer</span>
          <button
            class="text-gray-500 hover:text-white transition-colors text-sm leading-none px-1.5 py-0.5 rounded hover:bg-white/10"
            @click="emit('fermer')"
            title="Fermer"
          >✕</button>
        </div>
      </div>

      <!-- Corps -->
      <div class="sm-body">
        <!-- Horodatage -->
        <div class="text-[10px] text-gray-500 mb-2">{{ heureFormatee }}</div>

        <!-- Description technique -->
        <div class="text-sm text-gray-200 leading-snug mb-1">{{ signal.description }}</div>

        <!-- Conseil de trading -->
        <div v-if="conseil" class="text-xs italic text-gray-400 mb-3">
          💡 {{ conseil }}
        </div>
        <div v-else class="mb-3" />

        <!-- Proposition de prise de position -->
        <template v-if="niveaux">
          <div class="sm-sep" />
          <div class="flex items-center gap-2 mb-3">
            <span class="text-xs uppercase font-semibold tracking-wider" :class="dirColor">
              Proposition {{ dirLabel }}
            </span>
            <span class="text-gray-600 text-xs">(basé sur ATR×2)</span>
          </div>
          <div class="grid grid-cols-2 gap-2 mb-3">
            <div class="sm-niveau">
              <span class="sm-label">Entrée</span>
              <span class="text-white text-sm font-semibold">{{ formatPrix(niveaux.entry) }}</span>
            </div>
            <div class="sm-niveau sm-niveau-sl">
              <span class="sm-label">Stop Loss</span>
              <span class="text-red-400 text-sm font-semibold">{{ formatPrix(niveaux.sl) }}</span>
            </div>
            <div class="sm-niveau sm-niveau-tp1">
              <span class="sm-label">TP1 (×2 ATR)</span>
              <span class="text-emerald-400 text-sm font-semibold">{{ formatPrix(niveaux.tp1) }}</span>
            </div>
            <div class="sm-niveau sm-niveau-tp2">
              <span class="sm-label">TP2 (×3 ATR)</span>
              <span class="text-emerald-300 text-sm font-semibold">{{ formatPrix(niveaux.tp2) }}</span>
            </div>
          </div>
          <!-- Risk / Reward -->
          <div class="flex gap-4 text-[11px] text-gray-400">
            <span>R/R TP1 : <span class="text-gray-200">{{ rrTp1 }}</span></span>
            <span>R/R TP2 : <span class="text-gray-200">{{ rrTp2 }}</span></span>
          </div>
        </template>

        <!-- Pas d'ATR disponible -->
        <template v-else-if="signal.direction !== 'neutre'">
          <div class="sm-sep" />
          <div class="text-xs text-gray-500 italic">
            Activez l'indicateur ATR pour calculer les niveaux SL / TP.
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { FORCE_LABEL } from '@/composables/chartSignauxTypes'
import type { SignalIndicateur } from '@/composables/chartSignauxTypes'
import type { NiveauSlTp } from '@/composables/chartAtrSlTp'

const CONSEILS: Record<string, string> = {
  golden_cross: `Envisagez un BUY sur pullback vers l'EMA.`,
  death_cross: `Réduisez l'exposition, envisagez un SELL sur rebond.`,
  survente_sortie: `Le vendeur s'épuise — cherchez un BUY à confirmation.`,
  surachat_sortie: `Momentum haussier s'affaiblit — serrez le stop ou prenez profit.`,
  mi_ligne_haussiere: `Tendance favorise les acheteurs, renforcez en pullback.`,
  mi_ligne_baissiere: `Tendance favorise les vendeurs, allégez ou basculez en SELL.`,
  croisement_haussier: `MACD croise à la hausse — signal BUY de court terme.`,
  croisement_baissier: `MACD croise à la baisse — signal de sortie ou entrée SELL.`,
  zero_haussier: `MACD passe au-dessus de zéro — tendance haussière confirmée.`,
  zero_baissier: `MACD passe sous zéro — tendance baissière confirmée.`,
  touche_bande_basse: `Prix en zone de survente — possible rebond BUY, attendez confirmation.`,
  touche_bande_haute: `Prix en zone de surachat — possible retournement SELL.`,
  cassure_basse: `Rupture baissière Bollinger — trailing stop sur SELL conseillé.`,
  cassure_haute: `Breakout haussier Bollinger — stop serré sur BUY.`,
  squeeze: `Contraction de volatilité — explosion imminente (BUY ou SELL).`,
  atr_spike: `Volatilité anormale — évitez d'entrer, attendez la clôture.`,
  atr_compression: `ATR au plus bas — explosion imminente avant le breakout.`,
  boll_rsi_bull: `Double confluence : bande basse + oversold — BUY avec stop sous la bande.`,
  boll_rsi_bear: `Double confluence : bande haute + overbought — SELL avec stop au-dessus.`,
  squeeze_macd_bull: `Compression + MACD haussier — breakout BUY imminent.`,
  squeeze_macd_bear: `Compression + MACD baissier — breakout SELL imminent.`,
  atr_macd_bull: `Volatilité + momentum haussiers alignés — trailing stop recommandé.`,
  atr_macd_bear: `Volatilité + momentum baissiers alignés — trailing stop recommandé.`,
  ema_macd_bull: `EMA + MACD tous deux haussiers — BUY en continuation, stop sous l'EMA.`,
  ema_macd_bear: `EMA + MACD tous deux baissiers — SELL en continuation, stop au-dessus de l'EMA.`,
  cross_macd_bull: `Golden Cross confirmé par MACD — signal BUY majeur.`,
  cross_macd_bear: `Death Cross confirmé par MACD — signal SELL majeur.`,
}

const props = defineProps<{
  signal: SignalIndicateur | null
  niveaux: NiveauSlTp | null
}>()

const emit = defineEmits<{ fermer: [] }>()

// ─── Computed UI ──────────────────────────────────────────────────────────────

const dirIcon = computed(() =>
  props.signal?.direction === 'bullish' ? '🟢'
  : props.signal?.direction === 'bearish' ? '🔴'
  : '⚪',
)

const dirLabel = computed(() =>
  props.signal?.direction === 'bullish' ? 'BUY'
  : props.signal?.direction === 'bearish' ? 'SELL'
  : 'Neutre',
)

const dirColor = computed(() =>
  props.signal?.direction === 'bullish' ? 'text-emerald-400'
  : props.signal?.direction === 'bearish' ? 'text-red-400'
  : 'text-gray-400',
)

const forceCouleur = computed(() =>
  props.signal?.force === 'fort' ? 'text-amber-400'
  : props.signal?.force === 'moyen' ? 'text-blue-400'
  : 'text-gray-500',
)

const heureFormatee = computed(() => {
  if (!props.signal) return ''
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'Europe/Paris',
    day: '2-digit', month: '2-digit', year: '2-digit',
    hour: '2-digit', minute: '2-digit',
  }).format(new Date(props.signal.timestamp * 1000))
})

const conseil = computed(() =>
  props.signal ? (CONSEILS[props.signal.type_signal] ?? '') : '',
)

const rrTp1 = computed(() => {
  if (!props.niveaux) return '—'
  const risk = Math.abs(props.niveaux.entry - props.niveaux.sl)
  if (risk === 0) return '—'
  return (Math.abs(props.niveaux.tp1 - props.niveaux.entry) / risk).toFixed(1)
})

const rrTp2 = computed(() => {
  if (!props.niveaux) return '—'
  const risk = Math.abs(props.niveaux.entry - props.niveaux.sl)
  if (risk === 0) return '—'
  return (Math.abs(props.niveaux.tp2 - props.niveaux.entry) / risk).toFixed(1)
})

function formatPrix(v: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency', currency: 'USD',
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v)
}

// ─── Position ─────────────────────────────────────────────────────────────────

const posX = ref(0)
const posY = ref(0)

function centrer() {
  posX.value = Math.max(0, window.innerWidth / 2 - 160)
  posY.value = Math.max(40, window.innerHeight * 0.15)
}

watch(() => props.signal, (val) => { if (val) centrer() })
centrer()

// ─── Drag ─────────────────────────────────────────────────────────────────────

let dragging = false
let startMouseX = 0, startMouseY = 0, startPosX = 0, startPosY = 0

function demarrerDrag(e: MouseEvent) {
  dragging = true
  startMouseX = e.clientX; startMouseY = e.clientY
  startPosX = posX.value; startPosY = posY.value
  e.preventDefault()
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', arretDrag)
}

function onMouseMove(e: MouseEvent) {
  if (!dragging) return
  posX.value = startPosX + (e.clientX - startMouseX)
  posY.value = startPosY + (e.clientY - startMouseY)
}

function arretDrag() {
  dragging = false
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', arretDrag)
}

onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', arretDrag)
})
</script>

<style scoped>
.sm-window {
  position: fixed; z-index: 9998;
  width: 340px; max-width: calc(100vw - 32px);
  display: flex; flex-direction: column;
  border-radius: 14px;
  border: 1px solid rgba(16, 185, 129, 0.25);
  background: rgba(10, 14, 39, 0.96);
  backdrop-filter: blur(16px);
  box-shadow: 0 8px 32px rgba(0,0,0,0.65), 0 0 40px rgba(16,185,129,0.05);
}
.sm-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px; border-bottom: 1px solid rgba(255,255,255,0.08);
  cursor: move; user-select: none; flex-shrink: 0;
  background: linear-gradient(135deg, rgba(16,185,129,0.06), rgba(59,130,246,0.04));
  border-radius: 13px 13px 0 0;
}
.sm-body { padding: 14px 16px; }
.sm-sep { height: 1px; background: rgba(255,255,255,0.08); margin: 10px 0 12px; }
.sm-niveau {
  display: flex; flex-direction: column; gap: 2px;
  padding: 8px 10px; border-radius: 8px;
  border: 1px solid rgba(255,255,255,0.07);
  background: rgba(255,255,255,0.03);
}
.sm-niveau-sl  { border-color: rgba(239,68,68,0.2); }
.sm-niveau-tp1 { border-color: rgba(16,185,129,0.2); }
.sm-niveau-tp2 { border-color: rgba(52,211,153,0.15); }
.sm-label { font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.04em; }
</style>
