<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
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
        <button class="btn-sm text-gray-400 hover:text-white" title="Effacer la conversation" @click="effacer">🗑</button>
      </div>
    </div>

    <!-- Conversation -->
    <div
      ref="zoneChat"
      class="glass-card p-4 space-y-4 overflow-y-auto"
      style="min-height: 420px; max-height: 520px;"
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
          class="max-w-[80%] rounded-2xl px-4 py-3 text-sm leading-relaxed"
          :class="msg.role === 'user'
            ? 'bg-blue-600/30 text-blue-100 rounded-br-sm'
            : 'bg-white/5 text-gray-100 rounded-bl-sm'"
        >
          <span v-if="msg.role === 'assistant'" class="text-xs text-purple-400 block mb-1">🤖 Coach IA</span>
          <span class="whitespace-pre-wrap">{{ msg.contenu }}</span>
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
    <div class="flex gap-3">
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
    <div v-if="!ollamaOk" class="glass-card p-5 border-yellow-500/30 bg-yellow-900/10">
      <h3 class="text-yellow-400 font-semibold mb-2">⚠️ Ollama n'est pas démarré</h3>
      <pre class="text-xs text-gray-300 bg-black/30 p-3 rounded">curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2.5:14b
ollama serve</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

interface Message { role: 'user' | 'assistant'; contenu: string }

const alerteStore = useAlerteStore()
const chargement = ref(false)
const ollamaOk = ref(false)
const modeleActif = ref('qwen2.5:14b')
const messageEnCours = ref('')
const messages = ref<Message[]>([])
const zoneChat = ref<HTMLElement | null>(null)

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
    modeleActif.value = s.modele
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
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Ollama: ${(e as Error).message}`)
    messages.value.push({
      role: 'assistant',
      contenu: '❌ Impossible de contacter Ollama. Vérifiez que le service est démarré.'
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

onMounted(verifierStatut)
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-sm { @apply text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
