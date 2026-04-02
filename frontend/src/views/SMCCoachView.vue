<template>
  <div class="flex flex-col" style="height: calc(100vh - 3rem);">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3 flex-shrink-0">
      <div>
        <h1 class="text-2xl font-bold">💬 Coach Trading IA</h1>
        <p class="text-xs text-gray-500 mt-0.5">Posez vos questions sur le trading SMC — {{ modeleActif }}</p>
      </div>
      <div class="flex gap-2 items-center">
        <span
          class="text-xs px-2 py-1 rounded-full font-semibold"
          :class="ollamaOk ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/50 text-red-300'"
        >
          {{ ollamaOk ? '🟢 Ollama actif' : '🔴 Ollama hors ligne' }}
        </span>
        <span
          class="text-xs px-2 py-1 rounded-full font-semibold"
          :class="{
            'bg-emerald-900/50 text-emerald-300': anthropicStatut === 'ok',
            'bg-red-900/50 text-red-300': anthropicStatut === 'credits-insuffisants',
            'bg-gray-800 text-gray-500': anthropicStatut === 'non-configure',
          }"
          :title="anthropicStatut === 'credits-insuffisants' ? 'Rechargez vos crédits sur console.anthropic.com' : ''"
        >
          <span v-if="anthropicStatut === 'ok'">🔑 Anthropic OK</span>
          <span v-else-if="anthropicStatut === 'credits-insuffisants'">⚠️ Crédits épuisés</span>
          <span v-else>🔑 Pas de clé</span>
        </span>
        <button class="btn-sm text-gray-400 hover:text-white" title="Effacer la conversation" @click="effacer">🗑</button>
      </div>
    </div>

    <!-- Conversation — prend tout l'espace restant -->
    <div
      ref="zoneChat"
      class="glass-card p-4 space-y-4 overflow-y-auto flex-1 min-h-0"
    >
      <!-- Message de bienvenue -->
      <div v-if="messages.length === 0" class="text-center text-gray-500 py-12 space-y-2">
        <p class="text-3xl">🤖</p>
        <p class="text-sm">Je suis votre coach trading SMC IA.</p>
        <p class="text-xs">Posez-moi une question sur vos signaux, stratégies ou l'analyse de marché.</p>
        <div class="flex flex-wrap gap-2 justify-center mt-4">
          <button
            v-for="q in questionsRapides"
            :key="q"
            class="text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 px-3 py-1.5 rounded-full transition"
            @click="envoyerRapide(q)"
          >{{ q }}</button>
        </div>
      </div>

      <!-- Messages -->
      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="flex"
        :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <div
          class="rounded-2xl px-4 py-3 text-sm leading-relaxed"
          :class="msg.role === 'user'
            ? 'max-w-[70%] bg-blue-600/30 text-blue-100 rounded-br-sm'
            : 'w-full bg-white/5 text-gray-100 rounded-bl-sm'"
        >
          <span v-if="msg.role === 'assistant'" class="text-xs text-purple-400 block mb-1">🤖 Coach IA</span>
          <span v-if="msg.role === 'user'" class="whitespace-pre-wrap">{{ msg.contenu }}</span>
          <template v-else>
            <template v-for="(part, pi) in parseContent(msg.contenu)" :key="pi">
              <span v-if="part.type === 'text'" class="whitespace-pre-wrap">{{ part.content }}</span>
              <iframe
                v-else
                :srcdoc="buildSrcdoc(part.content, `${i}-${pi}`)"
                class="w-full rounded-lg border border-white/10 my-2 block"
                :style="{ height: (iframeHeights[`${i}-${pi}`] ?? 400) + 'px' }"
                sandbox="allow-scripts"
                title="Diagramme SMC"
              />
            </template>
          </template>
        </div>
      </div>

      <!-- Indicateur de frappe -->
      <div v-if="chargement" class="flex justify-start">
        <div class="bg-white/5 rounded-2xl rounded-bl-sm px-4 py-3 text-sm text-gray-400">
          <span class="animate-pulse">⏳ Réflexion en cours...</span>
        </div>
      </div>
    </div>

    <!-- Zone de saisie -->
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
        :class="peutEnvoyer ? 'bg-blue-600 hover:bg-blue-500 text-white' : 'bg-gray-700 text-gray-500 cursor-not-allowed'"
        :disabled="!peutEnvoyer"
        @click="envoyerMessage"
      >
        ↑ Envoyer
      </button>
    </div>

    <!-- Aide installation -->
    <div v-if="!ollamaOk" class="glass-card p-5 border-yellow-500/30 bg-yellow-900/10 mt-3 flex-shrink-0">
      <h3 class="text-yellow-400 font-semibold mb-2">⚠️ Ollama n'est pas démarré</h3>
      <pre class="text-xs text-gray-300 bg-black/30 p-3 rounded">curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2.5:14b
ollama serve</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

interface Message { role: 'user' | 'assistant'; contenu: string }

const alerteStore = useAlerteStore()
const chargement = ref(false)
const ollamaOk = ref(false)
const modeleActif = ref('claude-sonnet-4-5') // Coach : Sonnet (qualité), Chart : Haiku 3.5 (économique)
const messageEnCours = ref('')
const messages = ref<Message[]>([])
const zoneChat = ref<HTMLElement | null>(null)
const iframeHeights = ref<Record<string, number>>({})

type StatutAnthropic = 'non-configure' | 'ok' | 'credits-insuffisants'
const anthropicStatut = ref<StatutAnthropic>('non-configure')

async function verifierCleAnthropic() {
  const cfg = await apiService.obtenirConfig('anthropic_api_key')
  anthropicStatut.value = (cfg?.valeur && cfg.valeur.length > 0) ? 'ok' : 'non-configure'
}

function parseContent(text: string): Array<{ type: 'text' | 'diagram'; content: string }> {
  const parts: Array<{ type: 'text' | 'diagram'; content: string }> = []
  const regex = /<htmldiagram>([\s\S]*?)<\/htmldiagram>/g
  let last = 0
  let match
  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) parts.push({ type: 'text', content: text.slice(last, match.index) })
    parts.push({ type: 'diagram', content: match[1] })
    last = regex.lastIndex
  }
  if (last < text.length) parts.push({ type: 'text', content: text.slice(last) })
  return parts
}

function buildSrcdoc(html: string, id: string): string {
  const js = `const id='${id}';function send(){window.parent.postMessage({type:'resize',id,height:document.body.scrollHeight},'*')}window.addEventListener('load',()=>{send();setTimeout(send,300);setTimeout(send,800)});new ResizeObserver(send).observe(document.body)`
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><style>*{margin:0;padding:0;box-sizing:border-box}body{background:#0d1117;color:#e6edf3;font-family:'Inter',-apple-system,sans-serif;padding:12px;overflow-x:hidden}</style></head><body>${html}<script>${js}<` + `/script></body></html>`
}

function onIframeMessage(e: MessageEvent) {
  if (e.data?.type === 'resize' && typeof e.data.height === 'number' && e.data.id) {
    iframeHeights.value[e.data.id] = Math.min(Math.max(Number(e.data.height) + 24, 200), 800)
  }
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
  try {
    const s = await apiService.statutIA()
    ollamaOk.value = s.ollama_disponible
  } catch {
    ollamaOk.value = false
  }
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
    const res = await apiService.chatIA(historique)
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

function effacer() {
  messages.value = []
}

async function scrollBas() {
  await nextTick()
  if (zoneChat.value) {
    zoneChat.value.scrollTop = zoneChat.value.scrollHeight
  }
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
