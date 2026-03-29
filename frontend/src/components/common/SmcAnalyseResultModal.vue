<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="modal-wrapper"
      :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
      @mousedown="startDrag"
    >
      <!-- En-tête -->
      <div class="modal-header">
        <div class="flex items-center gap-2">
          <span class="text-xs font-bold uppercase tracking-wider" :class="dirClass">{{ direction }}</span>
          <span class="text-sm font-bold text-white">{{ asset }}</span>
          <span class="text-xs text-gray-400">{{ timeframe }}</span>
          <span class="score-badge" :class="score >= 70 ? 'score-ok' : 'score-low'">{{ score }}/80</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-gray-500">{{ modele }}</span>
          <button class="text-gray-500 hover:text-white text-lg leading-none" @click.stop="$emit('fermer')">✕</button>
        </div>
      </div>

      <!-- Prix clés -->
      <div class="grid grid-cols-5 gap-1.5 px-4 py-3 border-b border-white/5">
        <div class="prix-bloc">
          <span class="prix-label">Entrée</span>
          <span class="prix-val text-white">{{ fmt(prixEntree) }}</span>
        </div>
        <div class="prix-bloc">
          <span class="prix-label">Stop</span>
          <span class="prix-val text-red-400">{{ fmt(stopLoss) }}</span>
        </div>
        <div class="prix-bloc">
          <span class="prix-label">TP1</span>
          <span class="prix-val text-emerald-400">{{ fmt(tp1) }}</span>
        </div>
        <div class="prix-bloc">
          <span class="prix-label">TP2</span>
          <span class="prix-val text-emerald-300">{{ tp2 ? fmt(tp2) : '—' }}</span>
        </div>
        <div class="prix-bloc">
          <span class="prix-label">TP3</span>
          <span class="prix-val text-emerald-200">{{ tp3 ? fmt(tp3) : '—' }}</span>
        </div>
      </div>

      <!-- Analyse formatée -->
      <div class="modal-body">
        <div v-if="!analyse" class="flex items-center justify-center py-6 gap-2 text-gray-500 text-sm">
          <span class="animate-spin">⏳</span> Analyse en cours…
        </div>
        <div v-else class="analyse-content">
          <template v-for="(bloc, i) in blocsFormates" :key="i">
            <h3 v-if="bloc.type === 'titre'" class="bloc-titre">{{ bloc.texte }}</h3>
            <p v-else-if="bloc.type === 'verdict'" class="bloc-verdict" :class="bloc.classe">{{ bloc.texte }}</p>
            <p v-else-if="bloc.type === 'important'" class="bloc-important">{{ bloc.texte }}</p>
            <p v-else class="bloc-texte" v-html="bloc.html"></p>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  visible: boolean
  analyse: string
  modele: string
  asset: string
  timeframe: string
  direction: string
  score: number
  prixEntree: number
  stopLoss: number
  tp1: number
  tp2: number
  tp3: number
}>()

defineEmits<{ fermer: [] }>()

// ── Draggable ─────────────────────────────────────────────────────────────────
const pos = ref({ x: Math.max(0, window.innerWidth - 520), y: 60 })
let dragging = false
let startMouse = { x: 0, y: 0 }
let startPos   = { x: 0, y: 0 }

function startDrag(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('button')) return
  dragging = true
  startMouse = { x: e.clientX, y: e.clientY }
  startPos   = { ...pos.value }
  window.addEventListener('mousemove', onDrag)
  window.addEventListener('mouseup', stopDrag)
}
function onDrag(e: MouseEvent) {
  if (!dragging) return
  pos.value = {
    x: Math.max(0, Math.min(window.innerWidth  - 490, startPos.x + e.clientX - startMouse.x)),
    y: Math.max(0, Math.min(window.innerHeight - 100, startPos.y + e.clientY - startMouse.y)),
  }
}
function stopDrag() {
  dragging = false
  window.removeEventListener('mousemove', onDrag)
  window.removeEventListener('mouseup', stopDrag)
}

// ── Helpers ───────────────────────────────────────────────────────────────────
const dirClass = computed(() => {
  if (props.direction === 'LONG') return 'text-emerald-400'
  if (props.direction === 'SHORT') return 'text-red-400'
  return 'text-blue-400'
})

function fmt(n: number): string {
  if (!n) return '—'
  return n >= 1000
    ? n.toLocaleString('fr-FR', { maximumFractionDigits: 2 })
    : n.toFixed(5)
}

// ── Formatage de l'analyse Ollama ─────────────────────────────────────────────
type Bloc = { type: 'titre' | 'verdict' | 'important' | 'texte'; texte?: string; html?: string; classe?: string }

const blocsFormates = computed((): Bloc[] => {
  if (!props.analyse) return []
  const lignes = props.analyse.split('\n')
  const blocs: Bloc[] = []

  for (const ligne of lignes) {
    const l = ligne.trim()
    if (!l) continue

    // Titres markdown ## ou **TITRE**
    if (/^#{1,3}\s/.test(l)) {
      blocs.push({ type: 'titre', texte: l.replace(/^#{1,3}\s*/, '') })
      continue
    }
    if (/^\*\*[^*]{3,40}\*\*\s*:?\s*$/.test(l)) {
      blocs.push({ type: 'titre', texte: l.replace(/\*\*/g, '') })
      continue
    }

    // Verdict VALIDE / INVALIDE / RISQUÉ
    const lUp = l.toUpperCase()
    if (/(✅|VALIDE|SIGNAL VALIDE)/.test(lUp) && !/(NON|IN)VALIDE/.test(lUp)) {
      blocs.push({ type: 'verdict', texte: l.replace(/\*\*/g, ''), classe: 'verdict-ok' })
      continue
    }
    if (/(❌|INVALIDE|REJETÉ|RISQUÉ)/.test(lUp)) {
      blocs.push({ type: 'verdict', texte: l.replace(/\*\*/g, ''), classe: 'verdict-ko' })
      continue
    }
    if (/(⚠️|ATTENTION|RISQUE|PRUDENCE)/.test(lUp)) {
      blocs.push({ type: 'verdict', texte: l.replace(/\*\*/g, ''), classe: 'verdict-warn' })
      continue
    }

    // Lignes avec **gras** → important
    if (/\*\*/.test(l) && l.length < 150) {
      blocs.push({ type: 'important', texte: l.replace(/\*\*/g, '') })
      continue
    }

    // Ligne normale — formatage **bold** inline
    const html = l.replace(/\*\*(.+?)\*\*/g, '<strong class="text-white">$1</strong>')
      .replace(/^[-–•]\s*/, '<span class="text-blue-400 mr-1">›</span> ')
    blocs.push({ type: 'texte', html })
  }

  return blocs
})
</script>

<style scoped>
.modal-wrapper {
  position: fixed;
  z-index: 9998;
  width: 490px;
  max-height: 80vh;
  border-radius: 0.875rem;
  background: rgba(8, 12, 30, 0.97);
  border: 1px solid rgba(99, 102, 241, 0.35);
  backdrop-filter: blur(24px);
  box-shadow: 0 24px 64px rgba(0,0,0,0.8), 0 0 0 1px rgba(99,102,241,0.15);
  display: flex;
  flex-direction: column;
  user-select: none;
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  cursor: grab;
  flex-shrink: 0;
}
.modal-header:active { cursor: grabbing; }
.modal-body {
  overflow-y: auto;
  padding: 0.75rem 1rem 1rem;
  flex: 1;
}
.score-badge {
  font-size: 0.65rem;
  font-weight: 700;
  padding: 0.15rem 0.5rem;
  border-radius: 9999px;
}
.score-ok  { background: rgba(16,185,129,0.15); color: #34d399; border: 1px solid rgba(16,185,129,0.3); }
.score-low { background: rgba(239,68,68,0.15);  color: #f87171; border: 1px solid rgba(239,68,68,0.3); }
.prix-bloc { display: flex; flex-direction: column; align-items: center; background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.07); border-radius: 0.4rem; padding: 0.3rem 0.4rem; }
.prix-label { font-size: 0.6rem; color: #6b7280; text-transform: uppercase; letter-spacing: 0.05em; }
.prix-val   { font-size: 0.75rem; font-family: monospace; font-weight: 700; margin-top: 0.1rem; }
.analyse-content { display: flex; flex-direction: column; gap: 0.4rem; }
.bloc-titre { font-size: 0.75rem; font-weight: 700; color: #a5b4fc; text-transform: uppercase; letter-spacing: 0.06em; margin-top: 0.5rem; margin-bottom: 0.1rem; border-bottom: 1px solid rgba(165,180,252,0.15); padding-bottom: 0.25rem; }
.bloc-verdict { font-size: 0.8rem; font-weight: 700; padding: 0.4rem 0.75rem; border-radius: 0.4rem; }
.verdict-ok   { background: rgba(16,185,129,0.12); color: #34d399; border: 1px solid rgba(16,185,129,0.25); }
.verdict-ko   { background: rgba(239,68,68,0.12);  color: #f87171; border: 1px solid rgba(239,68,68,0.25); }
.verdict-warn { background: rgba(245,158,11,0.12); color: #fbbf24; border: 1px solid rgba(245,158,11,0.25); }
.bloc-important { font-size: 0.78rem; color: #e2e8f0; font-weight: 600; }
.bloc-texte { font-size: 0.76rem; color: #94a3b8; line-height: 1.55; }
</style>
