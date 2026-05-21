<template>
  <Teleport to="body">
    <div
      v-if="analyse"
      class="modal-window"
      :style="{ top: posY + 'px', left: posX + 'px' }"
    >
      <!-- Header draggable -->
      <div class="modal-header" @mousedown="demarrerDrag">
        <div class="flex items-center gap-2">
          <span class="text-purple-400 text-sm">🤖</span>
          <span class="text-xs font-semibold text-purple-300 tracking-wide">Analyse visuelle IA</span>
          <span class="text-gray-500 text-xs">—</span>
          <span class="text-gray-400 text-xs">{{ modele }}</span>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="text-xs font-medium text-emerald-400 bg-emerald-900/30 hover:bg-emerald-800/50 hover:text-emerald-300 px-2 py-0.5 rounded border border-emerald-500/20 transition-colors"
            @click="enregistrerImage"
            title="Enregistrer sous forme d'image"
          >
            {{ enSauvegarde ? '⏳' : '💾' }} Enregistrer
          </button>
          <span class="text-gray-600 text-xs select-none cursor-move">⠿ déplacer</span>
          <button
            class="text-gray-500 hover:text-white transition-colors text-sm leading-none px-1.5 py-0.5 rounded hover:bg-white/10"
            @click="emit('fermer')"
            title="Fermer"
          >✕</button>
        </div>
      </div>

      <!-- Contenu rendu bloc par bloc (sûr) -->
      <div class="modal-body">
        <template v-for="(bloc, idx) in blocs" :key="idx">
          <!-- Tableau Markdown → vrai <table> -->
          <div v-if="bloc.type === 'table'" class="ia-table-wrap">
            <table class="ia-table">
              <thead>
                <tr><th v-for="(h, hi) in bloc.entete" :key="hi" v-html="fmtInline(h)" /></tr>
              </thead>
              <tbody>
                <tr v-for="(row, ri) in bloc.corps" :key="ri">
                  <td v-for="(cell, ci) in row" :key="ci" v-html="fmtInline(cell)" />
                </tr>
              </tbody>
            </table>
          </div>
          <!-- Ligne vide -->
          <div v-else-if="bloc.type === 'spacer'" class="ia-spacer" />
          <!-- Texte avec bold -->
          <div v-else v-html="fmtInline(bloc.text)" :class="classeTexte(bloc.text)" />
        </template>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, computed, onUnmounted } from 'vue'

const props = defineProps<{ analyse: string | null; modele: string; asset?: string; timeframe?: string }>()
const emit = defineEmits<{ fermer: [] }>()

// ─── Parsing ──────────────────────────────────────────────────────────────────

interface BlocTable { type: 'table'; entete: string[]; corps: string[][] }
interface BlocTexte { type: 'texte' | 'spacer'; text: string }
type Bloc = BlocTable | BlocTexte

function normTexte(t: string): string {
  return t
    .replace(/\bLONG\b/g, 'BUY').replace(/\bSHORT\b/g, 'SELL')
    .replace(/\bLong\b/g, 'Buy').replace(/\bShort\b/g, 'Sell')
}

function echapper(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function fmtInline(s: string): string {
  return echapper(s).replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
}

function classeTexte(t: string): string {
  if (!t) return 'ia-spacer'
  if (t.startsWith('⚠️') || t.startsWith('⭐')) return 'ia-warn'
  if (t.startsWith('• ') || t.startsWith('- ')) return 'ia-bullet'
  return 'ia-line'
}

function estSepMd(l: string): boolean {
  return /^\|[\s\-|:]+\|$/.test(l.trim())
}

function parserAnalyse(texte: string): Bloc[] {
  const lignes = normTexte(texte).split('\n')
  const result: Bloc[] = []
  let i = 0
  while (i < lignes.length) {
    const l = lignes[i].trim()
    if (l.startsWith('|')) {
      const rows: string[] = []
      while (i < lignes.length && lignes[i].trim().startsWith('|')) {
        rows.push(lignes[i]); i++
      }
      const rangees = rows
        .filter(r => !estSepMd(r))
        .map(r => r.split('|').slice(1, -1).map(c => c.trim()))
      if (rangees.length >= 2) {
        result.push({ type: 'table', entete: rangees[0], corps: rangees.slice(1) })
      }
    } else if (!l) {
      result.push({ type: 'spacer', text: '' }); i++
    } else {
      result.push({ type: 'texte', text: l }); i++
    }
  }
  return result
}

const blocs = computed((): Bloc[] => props.analyse ? parserAnalyse(props.analyse) : [])

// ─── Position ─────────────────────────────────────────────────────────────────

const posX = ref(0)
const posY = ref(0)

const enSauvegarde = ref(false)
const erreurSauvegarde = ref<string | null>(null)

async function enregistrerImage() {
  if (enSauvegarde.value) return
  enSauvegarde.value = true

  try {
    if (!(window as any).html2canvas) {
      await new Promise((resolve, reject) => {
        const script = document.createElement('script')
        script.src = 'https://cdn.jsdelivr.net/npm/html2canvas@1.4.1/dist/html2canvas.min.js'
        script.onload = resolve
        script.onerror = reject
        document.head.appendChild(script)
      })
    }

    const el = document.querySelector('.modal-window') as HTMLElement
    const bodyEl = document.querySelector('.modal-body') as HTMLElement
    if (!el || !bodyEl) throw new Error('Modale introuvable')

    // Sauvegarde des styles initiaux pour les remettre après
    const baseWindowMaxHeight = el.style.maxHeight
    const baseWindowHeight = el.style.height
    const baseBodyOverflow = bodyEl.style.overflowY

    // Forcer le déploiement complet en hauteur pour capturer tout le scroll
    el.style.maxHeight = 'none'
    el.style.height = 'auto'
    bodyEl.style.overflowY = 'visible'

    // Laisser un tick au navigateur pour recalculer la mise en page
    await new Promise(r => setTimeout(r, 50))

    // @ts-ignore
    const canvas = await window.html2canvas(el, {
      backgroundColor: '#0a0e27',
      scale: 2,
      scrollY: -window.scrollY // Empêche les décalages de capture si la page était scrollée
    })

    // Restauration de la vue scrollable
    el.style.maxHeight = baseWindowMaxHeight
    el.style.height = baseWindowHeight
    bodyEl.style.overflowY = baseBodyOverflow

    const base64 = canvas.toDataURL('image/png').split(',')[1]

    const res = await fetch('http://localhost:8080/api/ia/save-analysis', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        image_base64: base64,
        asset: props.asset || 'UNKNOWN',
        timeframe: props.timeframe || 'UNKNOWN',
      })
    })

    if (!res.ok) throw new Error('Erreur HTTP ' + res.status)

    // Optionnel : un petit effet visuel ou flash de réussite
  } catch (err) {
    console.error('Echec capture:', err)
    erreurSauvegarde.value = (err as Error).message
  } finally {
    enSauvegarde.value = false
  }
}

function centrer() {
  posX.value = Math.max(0, window.innerWidth / 2 - 550)
  posY.value = Math.max(0, window.innerHeight * 0.01)
}

watch(() => props.analyse, (val) => { if (val) centrer() })
centrer()

// ─── Drag ──────────────────────────────────────────────────────────────────────

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
.modal-window {
  position: fixed; z-index: 9999;
  width: 1100px; max-width: calc(100vw - 32px); height: 98vh; max-height: 98vh;
  display: flex; flex-direction: column;
  border-radius: 14px;
  border: 1px solid rgba(168, 85, 247, 0.35);
  background: rgba(10, 14, 39, 0.94);
  backdrop-filter: blur(16px);
  box-shadow: 0 0 0 1px rgba(168,85,247,0.08), 0 8px 32px rgba(0,0,0,0.6), 0 0 60px rgba(168,85,247,0.08);
}
.modal-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px; border-bottom: 1px solid rgba(168,85,247,0.2);
  cursor: move; user-select: none; flex-shrink: 0;
  background: linear-gradient(135deg, rgba(168,85,247,0.08), rgba(59,130,246,0.05));
  border-radius: 13px 13px 0 0;
}
.modal-body {
  padding: 14px 16px; overflow-y: auto; flex: 1;
  scrollbar-width: thin; scrollbar-color: rgba(168,85,247,0.3) transparent;
}
.modal-body::-webkit-scrollbar { width: 5px; }
.modal-body::-webkit-scrollbar-track { background: transparent; }
.modal-body::-webkit-scrollbar-thumb { background: rgba(168,85,247,0.3); border-radius: 99px; }

.ia-line   { font-size: 0.8125rem; color: #d1d5db; line-height: 1.55; margin-bottom: 2px; }
.ia-bullet { font-size: 0.8125rem; color: #d1d5db; line-height: 1.55; padding-left: 12px; margin-bottom: 2px; }
.ia-warn   { font-size: 0.8125rem; color: #fcd34d; line-height: 1.55; margin-bottom: 3px; }
.ia-spacer { height: 8px; }
:deep(strong) { color: #e9d5ff; font-weight: 600; }

.ia-table-wrap { overflow-x: auto; margin: 10px 0 14px; border-radius: 8px; border: 1px solid rgba(168,85,247,0.25); }
.ia-table      { width: 100%; border-collapse: collapse; font-size: 0.78rem; }
.ia-table thead tr { background: rgba(168,85,247,0.18); }
.ia-table th   { padding: 7px 10px; text-align: left; font-weight: 600; color: #c4b5fd; white-space: nowrap; border-bottom: 1px solid rgba(168,85,247,0.25); }
.ia-table td   { padding: 6px 10px; color: #e5e7eb; border-bottom: 1px solid rgba(255,255,255,0.05); }
.ia-table tbody tr:last-child td { border-bottom: none; }
.ia-table tbody tr:nth-child(even) { background: rgba(255,255,255,0.03); }
.ia-table tbody tr:hover           { background: rgba(168,85,247,0.07); }
</style>
