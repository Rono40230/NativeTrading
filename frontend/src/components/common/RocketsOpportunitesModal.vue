<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-[10000] flex flex-col"
        style="background: #0b0f28"
      >
        <div class="modal-card flex flex-col h-full w-full overflow-hidden">
          <!-- En-tête -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 shrink-0">
            <div>
              <p class="text-sm font-bold text-white tracking-wide">🚀 Opportunités Rockets — Analyse IA</p>
              <p class="text-[10px] text-gray-500 mt-0.5">Top candidats par score · Support / TP · Analyse Ollama</p>
            </div>
            <div class="flex items-center gap-3">
              <!-- Filtres phase -->
              <div class="flex gap-1.5">
                <button
                  v-for="f in filtres"
                  :key="f.val"
                  class="text-[9px] font-semibold px-2 py-0.5 rounded-lg border transition-all"
                  :class="filtre === f.val ? f.classeActif : 'border-white/10 text-gray-500 hover:text-gray-300'"
                  @click="filtre = f.val"
                >{{ f.label }}</button>
              </div>
              <button
                class="text-gray-500 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/10"
                @click="$emit('close')"
              >✕</button>
            </div>
          </div>

          <div class="overflow-y-auto flex-1 px-6 py-5 scroll-zone">
            <!-- Stats -->
            <div class="grid grid-cols-3 gap-3 mb-5">
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Signaux détectés</p>
                <p class="text-2xl font-bold text-white">{{ signaux.length }}</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Score moyen</p>
                <p class="text-2xl font-bold" :class="classeScore(scoreMoyen)">{{ scoreMoyen.toFixed(0) }}/100</p>
              </div>
              <div class="stat-bloc">
                <p class="text-[10px] text-gray-500 mb-0.5">Phase dominante</p>
                <p class="text-lg font-bold text-white">{{ icone(phaseDominante) }} {{ labelPhase(phaseDominante) }}</p>
              </div>
            </div>

            <!-- Tableau top signaux -->
            <RocketsOpportunitesTableau :signaux="top5" />
            <!-- Analyse IA -->
            <div>
              <div class="flex items-center gap-2 mb-4">
                <span class="text-[10px] font-semibold uppercase tracking-widest text-orange-400">Analyse IA (Ollama)</span>
                <div v-if="chargementIA" class="h-2 w-2 animate-pulse rounded-full bg-orange-500" />
                <button v-if="!chargementIA" class="ml-auto text-[9px] text-gray-500 hover:text-gray-300 border border-white/10 rounded px-2 py-0.5" @click="lancerAnalyse">↺ Relancer</button>
              </div>
              <div v-if="chargementIA" class="flex items-center justify-center gap-2 py-10 text-xs text-gray-500">
                <div class="h-4 w-4 animate-spin rounded-full border-2 border-orange-500 border-t-transparent" />Analyse en cours…
              </div>
              <div v-else-if="erreurIA" class="text-xs text-red-400">{{ erreurIA }}</div>
              <div v-else-if="cartesAnalyse.length" class="grid grid-cols-2 gap-3">
                <div
                  v-for="(c, i) in cartesAnalyse"
                  :key="i"
                  class="rounded-xl border p-4"
                  :class="[c.ticker ? classeCarteSignal(c.phase) : 'border-blue-500/30 bg-blue-500/5 col-span-2']"
                >
                  <!-- Header -->
                  <div class="flex items-center gap-2 mb-3">
                    <template v-if="c.ticker">
                      <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full" :class="classeBadgePhase(c.phase)">
                        {{ icone(c.phase!) }} {{ labelPhase(c.phase!) }}
                      </span>
                      <span class="text-sm font-bold text-white">{{ c.ticker }}</span>
                      <span v-if="c.verdict" class="ml-auto text-[10px] font-bold px-2.5 py-0.5 rounded-full" :class="classeBadgeVerdict(c.verdict)">
                        {{ labelVerdict(c.verdict) }}
                      </span>
                    </template>
                    <template v-else>
                      <span class="text-[10px] font-semibold text-blue-300 uppercase tracking-widest">🌐 Conclusion globale</span>
                    </template>
                  </div>
                  <!-- Corps : 2 colonnes si signal, pleine largeur si conclusion globale -->
                  <div v-if="c.signal" class="grid gap-4" style="grid-template-columns: 180px 1fr">
                    <div class="text-sm space-y-1.5 text-gray-400 border-r border-white/10 pr-4">
                      <div>Variation 1h : <span class="font-semibold" :class="c.signal.change1h>=0?'text-emerald-400':'text-red-400'">{{ c.signal.change1h>=0?'+':'' }}{{ c.signal.change1h.toFixed(2) }}%</span></div>
                      <div>Volume : <span class="font-semibold" :class="c.signal.ratioVolume>=2?'text-orange-400':'text-gray-300'">{{ c.signal.ratioVolume.toFixed(2) }}×</span></div>
                      <div>ATR ratio : <span class="font-semibold text-gray-300">{{ c.signal.atrRatio.toFixed(2) }}</span></div>
                      <div>RSI : <span class="font-semibold" :class="c.signal.rsi>70?'text-orange-400':c.signal.rsi<40?'text-blue-400':'text-gray-300'">{{ c.signal.rsi.toFixed(1) }}</span></div>
                    </div>
                    <div class="text-sm leading-relaxed text-gray-300" v-html="c.texte" />
                  </div>
                  <p v-else class="text-sm leading-relaxed text-gray-300" v-html="c.texte" />
                </div>
              </div>
              <p v-else class="text-xs text-gray-600 italic">Aucune analyse pour l'instant.</p>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { SignalRocket, PhaseRocket } from '@/composables/useVeilleRockets'
import { useRocketsHelpers } from '@/composables/useRocketsHelpers'
import { apiService } from '@/services/api.service'
import RocketsOpportunitesTableau from './RocketsOpportunitesTableau.vue'

const props = defineProps<{ visible: boolean; signaux: SignalRocket[] }>()
defineEmits<{ close: [] }>()

const {
  icone, labelPhase, classeScore, formatPrix,
  classeCarteSignal, classeBadgePhase, classeBadgeVerdict, labelVerdict,
} = useRocketsHelpers()



type FiltrePhaseTous = 'tous' | PhaseRocket
const filtre = ref<FiltrePhaseTous>('tous')
const filtres = [
  { val: 'tous' as const,        label: 'Tous',         classeActif: 'border-white/30 text-white bg-white/10' },
  { val: 'compression' as const, label: '🌀 Compression', classeActif: 'border-blue-500/60 text-blue-300 bg-blue-500/10' },
  { val: 'prelancement' as const, label: '⚡ Pré-lancement', classeActif: 'border-yellow-500/60 text-yellow-300 bg-yellow-500/10' },
  { val: 'breakout' as const,    label: '🚀 Breakout',   classeActif: 'border-emerald-500/60 text-emerald-300 bg-emerald-500/10' },
]

const signauxFiltres = computed(() =>
  filtre.value === 'tous' ? props.signaux : props.signaux.filter(s => s.phase === filtre.value)
)
const top5 = computed(() => [...signauxFiltres.value].sort((a, b) => b.score - a.score).slice(0, 5))
const scoreMoyen = computed(() => props.signaux.length === 0 ? 0 : props.signaux.reduce((s, r) => s + r.score, 0) / props.signaux.length)
const phaseDominante = computed<PhaseRocket>(() => {
  const counts = { breakout: 0, prelancement: 0, compression: 0 }
  props.signaux.forEach(s => counts[s.phase]++)
  return (Object.entries(counts).sort((a, b) => b[1] - a[1])[0][0]) as PhaseRocket
})

const analyseIA = ref(''), chargementIA = ref(false), erreurIA = ref('')

function rendreMd(texte: string): string {
  return texte
    .replace(/^#{1,4}\s+/gm, '')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
    .replace(/«([^\u00bb]+)»/g, '<span class="font-semibold text-white">«$1»</span>')    // Saut de ligne après chaque phrase (. suivi d'espace + majuscule)
    .replace(/\.\s+(?=[A-ZÀ-Ü])/g, '.<br>')    .replace(/\n/g, '<br>')
}

interface CarteAnalyse { ticker: string | null; phase: PhaseRocket | null; texte: string; verdict: 'long' | 'attendre' | 'eviter' | null; signal: SignalRocket | null }
const cartesAnalyse = computed<CarteAnalyse[]>(() => {
  if (!analyseIA.value) return []
  const cartes: CarteAnalyse[] = []
  const blocs = analyseIA.value.split(/\n{2,}/).map(b => b.trim()).filter(Boolean)
  for (const bloc of blocs) {
    // SIGNAL: TICKER | VERDICT: xxx\n<analyse>
    const mSignal = bloc.match(/^SIGNAL\s*:\s*(\w+)\s*\|\s*VERDICT\s*:\s*([^\n]+)/im)
    if (mSignal) {
      const ticker = mSignal[1].toUpperCase()
      const verdictBrut = mSignal[2].trim()
      const signal = top5.value.find(s => s.ticker.toUpperCase() === ticker) ?? null
      const analyse = bloc.replace(/^SIGNAL[^\n]+\n?/im, '').trim()
      const verdict: 'long' | 'attendre' | 'eviter' =
        /long|imminent/i.test(verdictBrut) ? 'long' :
        /épuis/i.test(verdictBrut) ? 'eviter' : 'attendre'
      cartes.push({ ticker, phase: signal?.phase ?? null, texte: rendreMd(analyse), verdict, signal })
      continue
    }
    // CONCLUSION: <texte>
    const mConclusion = bloc.match(/^CONCLUSION\s*:\s*([\s\S]+)/im)
    if (mConclusion) {
      cartes.push({ ticker: null, phase: null, texte: rendreMd(mConclusion[1].trim()), verdict: null, signal: null })
    }
  }
  return cartes
})

async function lancerAnalyse() {
  if (top5.value.length === 0) return
  chargementIA.value = true; erreurIA.value = ''; analyseIA.value = ''
  try {
    const liste = top5.value.map((s, i) => {
      return `${i + 1}. ${s.ticker} — Phase: ${labelPhase(s.phase)} | Variation 1h: ${s.change1h >= 0 ? '+' : ''}${s.change1h.toFixed(2)}% | Vol×: ${s.ratioVolume.toFixed(2)} | ATR ratio: ${s.atrRatio.toFixed(2)} | RSI: ${s.rsi.toFixed(1)} | E.Limite: ${formatPrix(s.entreeLimite)}$ | E.Stop: ${formatPrix(s.entreeStop)}$ | Invalidation: ${formatPrix(s.niveauInvalidation)}$ | Entrée idéale: ${s.typeEntreeRec} | SL: ${formatPrix(s.sl)}$ | TP1 (1.5R): ${formatPrix(s.tp1)}$ | TP2 (2.5R): ${formatPrix(s.tp2)}$ | Trailing trigger (3.5R): ${formatPrix(s.tp3Trigger)}$ | Coef trailing: ${s.trailingCoeff.toFixed(1)}× | Score: ${s.score}/100`
    }).join('\n')
    const res = await apiService.chatIA([
      {
        role: 'system',
        contenu: `Tu es un trader algorithmique spécialisé en stratégie Rocket (compression volatilité → breakout LONG, sortie pyramidale TP1/TP2/TP3 avec BreakEven).

FORMAT DE RÉPONSE STRICT — respecte exactement cette structure :

SIGNAL: TICKER | VERDICT: <verdict>
<2 phrases d'analyse max, sans répéter les chiffres du prompt>

SIGNAL: TICKER2 | VERDICT: <verdict>
<2 phrases d'analyse max>

CONCLUSION: <synthèse globale en 2 phrases : phase de marché, meilleur setup, R-multiple>

Où <verdict> est EXACTEMENT l'un de ces trois textes : "LONG imminent" ou "Attendre confirmation" ou "Signal épuisé"

Règles :
- Réponds TOUJOURS en français
- INTERDIT ABSOLU : citer un chiffre (%, ×, $, ratio, RSI, score, prix) — les données sont déjà affichées, parle uniquement de leur signification qualitative
- Ne mets aucun titre, aucun markdown, aucune liste à puces`
      },
      { role: 'user', contenu: `Signaux Rocket à analyser :\n\n${liste}` }
    ])
    analyseIA.value = res.reponse
  } catch (err) {
    erreurIA.value = `Erreur IA : ${err instanceof Error ? err.message : String(err)}`
  } finally {
    chargementIA.value = false
  }
}

watch(() => props.visible, async (ouvert) => {
  if (!ouvert) return
  analyseIA.value = ''; erreurIA.value = ''
  await lancerAnalyse()
})
</script>

<style scoped>
.modal-card { background: #0b0f28; }
.stat-bloc { @apply rounded-xl border border-white/10 bg-white/5 p-4; }
.scroll-zone { scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.1) transparent; }
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
.modal-enter-active, .modal-leave-active { transition: opacity 0.18s, transform 0.18s; }
.modal-enter-from, .modal-leave-to { opacity: 0; transform: scale(0.96); }
.modal-enter-to, .modal-leave-from { opacity: 1; transform: scale(1); }
</style>
