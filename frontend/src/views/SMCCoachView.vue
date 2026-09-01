<template>
  <div class="flex flex-col" style="height: calc(100vh - 5.5rem);">
    <div class="flex items-center justify-between mb-3 flex-shrink-0">
      <div>
        <h1 class="text-2xl font-bold">💬 Coach Trading IA</h1>
        <p class="text-xs text-white mt-0.5">Posez vos questions sur le trading SMC — {{ modeleActif }}</p>
      </div>
      <div class="flex gap-2 items-center">
        <span
          class="text-xs px-2 py-1 rounded-full font-semibold"
          :class="ollamaOk ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/50 text-red-300'"
        >
          {{ ollamaOk ? '🟢 Ollama actif' : '🔴 Ollama hors ligne' }}
        </span>
        <button
          class="text-xs px-2 py-1 rounded-full font-semibold transition-all cursor-pointer select-none"
          :class="{
            'bg-emerald-900/50 text-emerald-300 hover:bg-emerald-800/60': anthropicStatut === 'ok' && anthropicActif,
            'bg-gray-700/50 text-white hover:bg-gray-600/50 line-through': anthropicStatut === 'ok' && !anthropicActif,
            'bg-red-900/50 text-red-300': anthropicStatut === 'credits-insuffisants',
            'bg-gray-800 text-white': anthropicStatut === 'non-configure',
          }"
          :disabled="anthropicStatut !== 'ok'"
          @click="anthropicStatut === 'ok' && toggleAnthropic()"
        >
          <span v-if="anthropicStatut === 'ok' && anthropicActif">🔑 Anthropic ON</span>
          <span v-else-if="anthropicStatut === 'ok' && !anthropicActif">🔑 Anthropic OFF</span>
          <span v-else-if="anthropicStatut === 'credits-insuffisants'">⚠️ Crédits épuisés</span>
          <span v-else>🔑 Pas de clé</span>
        </button>
        <button class="btn-sm text-white hover:text-white" title="Effacer la conversation" @click="effacer">🗑</button>
      </div>
    </div>

    <div class="flex-1 min-h-0 grid grid-cols-2 gap-4">

      <div ref="zoneChat" class="glass-card p-4 space-y-4 overflow-y-auto h-full min-h-0 flex flex-col">
        <div v-if="messages.length === 0" class="text-center text-white py-12 space-y-2 flex-1 flex flex-col items-center justify-center">
          <p class="text-3xl">🤖</p>
          <p class="text-sm">Je suis votre coach trading SMC IA.</p>
          <p class="text-xs">Posez-moi une question sur vos signaux, stratégies ou l'analyse de marché.</p>
          <div class="flex flex-wrap gap-2 justify-center mt-4">
            <button
              v-for="q in questionsRapides"
              :key="q"
              class="text-xs bg-gray-700 hover:bg-gray-600 text-white px-3 py-1.5 rounded-full transition"
              @click="envoyerRapide(q)"
            >{{ q }}</button>
          </div>
        </div>

        <!-- Messages (texte uniquement) -->
        <div
          v-for="(msg, i) in messages"
          :key="i"
          class="flex"
          :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
        >
          <div
            class="rounded-2xl px-4 py-3 text-sm leading-relaxed"
            :class="msg.role === 'user'
              ? 'max-w-[85%] bg-blue-600/30 text-blue-100 rounded-br-sm'
              : 'w-full bg-white/5 text-white rounded-bl-sm'"
          >
            <span v-if="msg.role === 'assistant'" class="text-xs text-purple-400 block mb-1">🤖 Coach IA</span>
            <span v-if="msg.role === 'user'" class="whitespace-pre-wrap">{{ msg.contenu }}</span>
              <template v-else>
              <span
                v-for="(part, pi) in parseContent(msg.contenu)"
                :key="pi"
              >
                <span v-if="part.type === 'text'" class="block" v-html="renderCoachMd(part.content)"></span>
                <span v-else-if="part.type === 'diagram'" class="inline-flex items-center gap-1 text-xs text-blue-400 bg-blue-500/10 border border-blue-500/20 rounded px-2 py-0.5 mx-1 cursor-pointer" @click="diagrammeActif = part.content">△ Voir le diagramme</span>
                <button
                  v-else-if="part.type === 'suggestion'"
                  class="flex items-center gap-2 mt-2 px-3 py-1.5 rounded-lg text-xs font-medium border border-blue-500/30 bg-blue-500/10 text-blue-300 hover:bg-blue-500/20 hover:border-blue-400/50 transition-all"
                  :disabled="diagramChargement"
                  @click="genererDiagram(part.content)"
                >
                  <span v-if="diagramChargement" class="animate-spin">⏳</span>
                  <span v-else>△</span>
                  Générer : {{ part.content }}
                </button>
              </span>
            </template>
          </div>
        </div>

        <div v-if="chargement" class="flex justify-start">
          <div class="bg-white/5 rounded-2xl rounded-bl-sm px-4 py-3 text-sm text-white">
            <span class="animate-pulse">⏳ Réflexion en cours...</span>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-3 h-full min-h-0 overflow-y-auto">

        <div
          v-if="tousLesDiagrammes.length === 0"
          class="glass-card h-full flex flex-col items-center justify-center text-white select-none"
        >
          <span class="text-4xl mb-3">△</span>
          <p class="text-sm">Les diagrammes apparaîtront ici</p>
          <p class="text-xs mt-1">Demandez un schéma ou une illustration visuelle</p>
        </div>

        <template v-else>
          <div
            v-for="(d, di) in tousLesDiagrammes"
            :key="di"
            class="glass-card overflow-hidden flex-shrink-0"
          >
            <div class="flex items-center gap-2 px-4 py-2 border-b border-white/10 bg-white/5">
              <span class="text-xs font-semibold text-blue-400">△ Diagramme {{ tousLesDiagrammes.length > 1 ? di + 1 : '' }}</span>
            </div>
            <iframe
              :srcdoc="buildSrcdoc(d.content, `diag-${di}`)"
              class="w-full border-0 block"
              :style="{ height: (iframeHeights[`diag-${di}`] ?? 400) + 'px' }"
              sandbox="allow-scripts"
              title="Diagramme SMC"
            />
          </div>
        </template>
      </div>

    </div>

    <div class="flex gap-3 mt-3 flex-shrink-0">
      <textarea
        v-model="messageEnCours"
        rows="2"
        placeholder="Posez votre question trading SMC..."
        class="flex-1 bg-gray-800 border border-gray-600 text-white text-sm rounded-xl px-4 py-3 resize-none focus:outline-none focus:border-blue-500"
        :disabled="chargement || !ollamaOk"
        @keydown.enter.prevent="envoyerMessage"
      />
      <button
        class="px-5 py-2 rounded-xl font-bold transition-all self-end"
        :class="peutEnvoyer ? 'bg-blue-600 hover:bg-blue-500 text-white' : 'bg-gray-700 text-white cursor-not-allowed'"
        :disabled="!peutEnvoyer"
        @click="envoyerMessage"
      >
        ↑ Envoyer
      </button>
    </div>

    <div v-if="!ollamaOk" class="glass-card p-5 border-yellow-500/30 bg-yellow-900/10 mt-3 flex-shrink-0">
      <h3 class="text-yellow-400 font-semibold mb-2">⚠️ Ollama n'est pas démarré</h3>
      <pre class="text-xs text-white bg-black/30 p-3 rounded">ollama pull qwen2.5vl:7b
ollama serve</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import {
  parseContent, buildSrcdoc, genererDiagram,
  diagrammesGeneres, diagramChargement,
} from '@/composables/useCoachDiagram'

interface Message { role: 'user' | 'assistant'; contenu: string }

const alerteStore = useAlerteStore()
const chargement = ref(false)
const ollamaOk = ref(false)
const modeleActif = ref('qwen2.5vl:7b')
const messageEnCours = ref('')
const messages = ref<Message[]>([])
const zoneChat = ref<HTMLElement | null>(null)
const iframeHeights = ref<Record<string, number>>({})
const diagrammeActif = ref<string | null>(null)

type StatutAnthropic = 'non-configure' | 'ok' | 'credits-insuffisants'
const anthropicStatut = ref<StatutAnthropic>('non-configure')
const anthropicActif = ref<boolean>(localStorage.getItem('anthropic_actif') !== 'false')

function toggleAnthropic() {
  anthropicActif.value = !anthropicActif.value
  localStorage.setItem('anthropic_actif', String(anthropicActif.value))
}
function renderCoachMd(text: string): string {
  return text
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/^### (.+)$/gm, '<p class="text-purple-300 font-semibold mt-2 mb-0.5">$1</p>')
    .replace(/^## (.+)$/gm, '<p class="text-blue-300 font-bold mt-3 mb-1">$1</p>')
    .replace(/^# (.+)$/gm, '<p class="text-white font-bold text-base mt-3 mb-1">$1</p>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em class="text-white">$1</em>')
    .replace(/^- (.+)$/gm, '<span class="block pl-3 before:content-[\'-\'] before:mr-2 before:text-white">$1</span>')
    .replace(/\n\n/g, '<br><br>')
    .replace(/\n/g, '<br>')
}
const tousLesDiagrammes = computed(() => {
  const diags: Array<{ type: 'diagram'; content: string }> = []
  for (const msg of messages.value) {
    if (msg.role !== 'assistant') continue
    for (const part of parseContent(msg.contenu))
      if (part.type === 'diagram') diags.push({ type: 'diagram', content: part.content })
  }
  for (const d of diagrammesGeneres.value) diags.push(d)
  return diags
})
async function verifierCleAnthropic() {
  const cfg = await apiService.obtenirConfig('anthropic_api_key')
  anthropicStatut.value = (cfg?.valeur && cfg.valeur.length > 0) ? 'ok' : 'non-configure'
}

function onIframeMessage(e: MessageEvent) {
  if (e.data?.type === 'resize' && typeof e.data.height === 'number' && e.data.id)
    iframeHeights.value[e.data.id] = Math.min(Math.max(Number(e.data.height) + 24, 200), 800)
}
const questionsRapides = [
  'Explique-moi le concept d\'Order Block',
  'Quand utiliser la stratégie Straddle ?',
  'Comment calculer mon position sizing ?',
  'Qu\'est-ce que l\'IFVG en SMC ?',
]
const peutEnvoyer = computed(
  () => messageEnCours.value.trim().length > 0 && !chargement.value && ollamaOk.value
)
async function verifierStatut() {
  try { const s = await apiService.statutIA(); ollamaOk.value = s.ollama_disponible }
  catch { ollamaOk.value = false }
}
async function envoyerRapide(question: string) {
  messageEnCours.value = question
  await envoyerMessage()
}
async function envoyerMessage() {
  const texte = messageEnCours.value.trim()
  if (!texte || chargement.value || !ollamaOk.value) return

  messages.value.push({ role: 'user', contenu: texte })
  messageEnCours.value = ''
  chargement.value = true
  await scrollBas()

  try {
    const historique = messages.value.map(m => ({ role: m.role, contenu: m.contenu }))
    const res = await apiService.chatIA(historique, !anthropicActif.value)
    messages.value.push({ role: 'assistant', contenu: res.reponse })
    modeleActif.value = res.modele
    if (res.modele.includes('claude')) anthropicStatut.value = 'ok'
  } catch (e: unknown) {
    const axiosErr = e as any
    const detail: string = axiosErr?.response?.data?.error ?? (e as Error).message
    if (detail.toLowerCase().includes('crédit') || detail.toLowerCase().includes('credit')) {
      anthropicStatut.value = 'credits-insuffisants'
    }
    alerteStore.afficherErreur(detail)
    messages.value.push({
      role: 'assistant',
      contenu: `❌ Erreur Coach IA : ${detail}`
    })
  } finally {
    chargement.value = false
    await scrollBas()
  }
}
function effacer() { messages.value = []; diagrammesGeneres.value = [] }
async function scrollBas() {
  await nextTick()
  if (zoneChat.value) zoneChat.value.scrollTop = zoneChat.value.scrollHeight
}

onMounted(() => {
  verifierStatut()
  verifierCleAnthropic()
  window.addEventListener('message', onIframeMessage)
})
onUnmounted(() => window.removeEventListener('message', onIframeMessage))
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-sm { @apply text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
